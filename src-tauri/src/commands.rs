use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::command;

use crate::tiling::{
    LayoutPlan, LayoutSpec, PdfReport, PdfRequest, TileError, oriented_image_dimensions,
    tile_to_pdf as run_tiling,
};

/// Tells the frontend what kind of failure occurred, so it can react
/// (e.g. offer an overwrite confirmation) instead of string-matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorKind {
    OutputExists,
    InvalidTileSize,
    InvalidDpi,
    InvalidOverlap,
    InvalidMargin,
    InvalidGrid,
    PageTooSmall,
    PageTooLarge,
    EmptyImage,
    ImageTooLarge,
    TooManyPages,
    OutputPathNotAbsolute,
    Io,
    Image,
    Pdf,
    TaskFailed,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiError {
    pub kind: ApiErrorKind,
    pub message: String,
}

impl ApiError {
    fn new(kind: ApiErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl From<TileError> for ApiError {
    fn from(err: TileError) -> Self {
        let kind = match &err {
            TileError::OutputExists(_) => ApiErrorKind::OutputExists,
            TileError::InvalidTileSize => ApiErrorKind::InvalidTileSize,
            TileError::InvalidDpi => ApiErrorKind::InvalidDpi,
            TileError::InvalidOverlap => ApiErrorKind::InvalidOverlap,
            TileError::InvalidMargin => ApiErrorKind::InvalidMargin,
            TileError::InvalidGrid => ApiErrorKind::InvalidGrid,
            TileError::PageTooSmall => ApiErrorKind::PageTooSmall,
            TileError::PageTooLarge { .. } => ApiErrorKind::PageTooLarge,
            TileError::EmptyImage => ApiErrorKind::EmptyImage,
            TileError::ImageTooLarge { .. } => ApiErrorKind::ImageTooLarge,
            TileError::TooManyPages { .. } => ApiErrorKind::TooManyPages,
            TileError::OutputPathNotAbsolute(_) => ApiErrorKind::OutputPathNotAbsolute,
            TileError::Io(_) | TileError::OutputWrite { .. } => ApiErrorKind::Io,
            TileError::Image(_) => ApiErrorKind::Image,
            TileError::Pdf(_) => ApiErrorKind::Pdf,
        };
        ApiError::new(kind, err.to_string())
    }
}

/// Holds a path delivered by the OS ("Open With" / double-click) before the
/// frontend is ready, so a cold start can still open the file.
#[derive(Default)]
pub struct PendingOpen(Mutex<Option<String>>);

impl PendingOpen {
    pub fn set(&self, path: String) {
        if let Ok(mut pending) = self.0.lock() {
            *pending = Some(path);
        }
    }

    fn take(&self) -> Option<String> {
        self.0.lock().ok().and_then(|mut pending| pending.take())
    }
}

/// File to open on startup: an OS-delivered file first, then the first
/// existing file among the command-line arguments.
#[command]
pub fn startup_input(pending: tauri::State<'_, PendingOpen>) -> Option<String> {
    pending
        .take()
        .or_else(|| std::env::args_os().skip(1).find_map(resolve_startup_input))
}

fn resolve_startup_input(arg: OsString) -> Option<String> {
    let canonical = std::fs::canonicalize(PathBuf::from(arg)).ok()?;
    if !canonical.is_file() {
        return None;
    }
    canonical.to_str().map(str::to_owned)
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlanRequest {
    pub input: PathBuf,
    pub spec: LayoutSpec,
}

/// Layout-only command: reads the image header (no decode) and returns the
/// exact plan the PDF writer will use, so the preview can match it.
#[command]
pub fn plan_layout(request: PlanRequest) -> Result<LayoutPlan, ApiError> {
    let (width, height) = oriented_image_dimensions(&request.input).map_err(ApiError::from)?;
    crate::tiling::layout::plan_layout(&request.spec, width, height).map_err(ApiError::from)
}

/// Extends the asset-protocol scope to the file the user just picked, so
/// previews work outside the static scope without exposing whole directories.
/// Returns the canonical path the frontend should load.
#[command]
pub fn allow_preview(app: tauri::AppHandle, path: String) -> Result<String, String> {
    use tauri::Manager;

    let canonical =
        std::fs::canonicalize(&path).map_err(|err| format!("cannot resolve {path}: {err}"))?;
    if !canonical.is_file() {
        return Err(format!("not a file: {path}"));
    }
    app.asset_protocol_scope()
        .allow_file(&canonical)
        .map_err(|err| format!("cannot allow preview: {err}"))?;
    Ok(canonical.to_string_lossy().into_owned())
}

#[command]
pub fn share_supported() -> bool {
    cfg!(target_os = "macos")
}

/// Shows the macOS share sheet for a generated PDF.
#[command]
pub fn share_pdf(window: tauri::WebviewWindow, path: String) -> Result<(), String> {
    let path = PathBuf::from(path);
    if !path.is_file() {
        return Err("The PDF to share no longer exists.".to_string());
    }

    #[cfg(target_os = "macos")]
    {
        let ns_window = window
            .ns_window()
            .map_err(|err| format!("cannot access the window: {err}"))?
            as usize;

        window
            .run_on_main_thread(move || {
                // SAFETY: run_on_main_thread runs AppKit code on the main thread.
                if let Err(err) = unsafe { crate::share::present(ns_window, &path) } {
                    tracing::error!(%err, "failed to present the share sheet");
                }
            })
            .map_err(|err| format!("cannot present the share sheet: {err}"))?;

        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, path);
        Err("Sharing is only supported on macOS.".to_string())
    }
}

/// Runs a tiling job off the main thread and off the async scheduler.
#[command]
pub async fn tile_to_pdf(request: PdfRequest) -> Result<PdfReport, ApiError> {
    let join_result = tauri::async_runtime::spawn_blocking(move || run_tiling(&request)).await;

    let job_result = join_result.map_err(|err| {
        ApiError::new(
            ApiErrorKind::TaskFailed,
            format!("tiling task failed: {err}"),
        )
    })?;

    job_result.map_err(|err| {
        tracing::error!(%err, "tiling job failed");
        ApiError::from(err)
    })
}

#[cfg(test)]
mod tests;
