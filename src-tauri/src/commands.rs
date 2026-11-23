use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::Serialize;
use tauri::command;

use crate::tiling::{tile_to_pdf as run_tiling, PdfRequest, PdfReport, TileError};

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
    InvalidPageSize,
    PageTooSmall,
    InvalidInputName,
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
            TileError::InvalidPageSize => ApiErrorKind::InvalidPageSize,
            TileError::PageTooSmall => ApiErrorKind::PageTooSmall,
            TileError::InvalidInputName(_) => ApiErrorKind::InvalidInputName,
            TileError::Io(_) => ApiErrorKind::Io,
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

/// File to open on startup: an OS-delivered file first, then `argv[1]`.
#[command]
pub fn startup_input(pending: tauri::State<'_, PendingOpen>) -> Option<String> {
    pending
        .take()
        .or_else(|| resolve_startup_input(std::env::args_os().nth(1)))
}

fn resolve_startup_input(arg: Option<OsString>) -> Option<String> {
    let canonical = std::fs::canonicalize(PathBuf::from(arg?)).ok()?;
    if !canonical.is_file() {
        return None;
    }
    canonical.to_str().map(str::to_owned)
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
            .map_err(|err| format!("cannot access the window: {err}"))? as usize;

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
