// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
pub mod commands;
#[cfg(target_os = "macos")]
mod share;
pub mod tiling;

use tauri::{Emitter, Manager};
use tracing_subscriber::{EnvFilter, fmt};

fn init_tracing() {
    let default_filter = if cfg!(debug_assertions) {
        "image_tiler_lib=debug,warn"
    } else {
        "image_tiler_lib=info,warn"
    };

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));

    // try_init: a second subscriber is not a crash-worthy condition.
    let _ = fmt().with_env_filter(filter).with_target(true).try_init();
}

/// Handles OS "open with" requests (macOS/iOS/Android): buffer the path for a
/// cold start and notify the frontend when a file is opened while running.
fn handle_run_event(app: &tauri::AppHandle, event: tauri::RunEvent) {
    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "android"))]
    {
        if let tauri::RunEvent::Opened { urls } = event {
            for url in urls {
                let Ok(path) = url.to_file_path() else {
                    continue;
                };
                if !path.is_file() {
                    continue;
                }

                let path = path.to_string_lossy().into_owned();
                app.state::<commands::PendingOpen>().set(path.clone());

                if let Err(err) = app.emit("open-image", path) {
                    tracing::warn!(%err, "failed to notify the frontend about an opened file");
                }
            }
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
    {
        let _ = (app, event);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::PendingOpen::default())
        .invoke_handler(tauri::generate_handler![
            commands::tile_to_pdf,
            commands::plan_layout,
            commands::startup_input,
            commands::allow_preview,
            commands::share_supported,
            commands::share_pdf
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(handle_run_event);
}
