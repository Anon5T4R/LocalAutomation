mod engine;

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use tauri::{AppHandle, Emitter, Manager};

static NEXT_RUN: AtomicU64 = AtomicU64::new(1);

/// Executa o fluxo (JSON do grafo) numa thread; logs por evento
/// `flow-log`/`flow-done`. Devolve o run_id na hora.
#[tauri::command(async)]
fn run_flow(app: AppHandle, flow_json: String) -> Result<u64, String> {
    let flow: engine::Flow =
        serde_json::from_str(&flow_json).map_err(|e| format!("fluxo inválido: {e}"))?;
    let run_id = NEXT_RUN.fetch_add(1, Ordering::Relaxed);
    std::thread::spawn(move || engine::run_flow(&app, run_id, flow));
    Ok(run_id)
}

#[tauri::command(async)]
fn read_flow(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))
}

#[tauri::command(async)]
fn write_flow(path: String, content: String) -> Result<(), String> {
    if let Some(parent) = Path::new(&path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    std::fs::write(&path, content).map_err(|e| format!("{path}: {e}"))
}

/// Arquivo `.tflow` passado no launch (associação), se houver.
#[tauri::command(async)]
fn get_startup_file() -> Option<String> {
    std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .find(|a| a.to_lowercase().ends_with(".tflow") && Path::new(a).is_file())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.set_focus();
            }
            if let Some(f) = args.into_iter().skip(1).find(|a| a.to_lowercase().ends_with(".tflow")) {
                let _ = app.emit("open-flow", f);
            }
        }));
    }

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            run_flow,
            read_flow,
            write_flow,
            get_startup_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
