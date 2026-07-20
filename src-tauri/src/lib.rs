mod backup;
mod engine;
mod secrets;
mod watch;

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use tauri::{AppHandle, Emitter, Manager};

static NEXT_RUN: AtomicU64 = AtomicU64::new(1);

/// Vigias de pasta ativas, por id do nó-gatilho. PORQUÊ global: a vigia vive
/// numa thread e precisa sobreviver às chamadas de comando; guardar o
/// "sinalizador de parada" aqui deixa `stop_watch` desligar a thread certa.
fn watchers() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    static W: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
    W.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Trava o mapa de vigias recuperando de poison: se alguma thread panicou com
/// a trava, perder os sinalizadores de parada deixaria as vigias órfãs pra
/// sempre — pior do que seguir com o mapa como está.
fn watchers_lock() -> std::sync::MutexGuard<'static, HashMap<String, Arc<AtomicBool>>> {
    watchers().lock().unwrap_or_else(|p| p.into_inner())
}

/// Liga a vigia de uma pasta. Idempotente por `id`: religar troca a config sem
/// vazar a thread antiga. Emite `watch-file` a cada arquivo estabilizado.
#[tauri::command(async)]
fn start_watch(
    app: AppHandle,
    id: String,
    folder: String,
    file_types: Vec<String>,
) -> Result<(), String> {
    let dir = Path::new(&folder);
    if !dir.is_dir() {
        // Erro de gente: o leigo escolheu (ou perdeu) uma pasta que não existe.
        return Err(format!("a pasta não existe: {folder}"));
    }
    stop_watch(id.clone()); // religar: mata a anterior antes

    let stop = Arc::new(AtomicBool::new(false));
    watchers_lock().insert(id.clone(), stop.clone());

    let cfg = watch::WatchConfig {
        folder: dir.to_path_buf(),
        file_types: file_types
            .into_iter()
            .map(|t| t.trim().trim_start_matches('.').to_lowercase())
            .filter(|t| !t.is_empty())
            .collect(),
        // Defaults de gente: 1,5 s parado = pronto; teto de 10 min por arquivo
        // (cópia de vídeo grande é lenta, mas não eterna).
        debounce_ms: 1_500,
        timeout_ms: 600_000,
        poll_ms: 400,
    };

    std::thread::spawn(move || {
        let emit = move |hit: watch::FileHit| {
            let _ = app.emit(
                "watch-file",
                serde_json::json!({
                    "watchId": id,
                    "path": hit.path,
                    "name": hit.name,
                    "folder": hit.folder,
                }),
            );
        };
        if let Err(e) = watch::run_watch(cfg, stop, emit) {
            eprintln!("vigia falhou: {e}");
        }
    });
    Ok(())
}

/// Desliga a vigia daquele nó (se houver). Sem erro se não existir.
#[tauri::command(async)]
fn stop_watch(id: String) {
    if let Some(stop) = watchers_lock().remove(&id) {
        stop.store(true, Ordering::Relaxed);
    }
}

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

// --- Variáveis secretas no cofre do SO ---
//
// REPARE NO QUE NÃO EXISTE AQUI: não há comando `get_secret`. O frontend
// escreve, apaga e pergunta "existe?", nunca lê. Quem lê é o motor, em Rust,
// no instante de montar a URL/comando — e o valor sai redigido dos logs.
// Um `get_secret` transformaria qualquer XSS no webview num vazamento total.

fn config_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path().app_config_dir().map_err(|e| format!("pasta de config: {e}"))
}

#[tauri::command(async)]
fn list_secrets(app: AppHandle) -> Result<Vec<serde_json::Value>, String> {
    let dir = config_dir(&app)?;
    Ok(secrets::list_names(&dir)
        .into_iter()
        .map(|name| {
            // `defined` separa "está no índice" de "está no cofre DESTE
            // computador": o .tflow viaja entre máquinas, o segredo não.
            let defined = secrets::exists(&name);
            serde_json::json!({ "name": name, "defined": defined })
        })
        .collect())
}

#[tauri::command(async)]
fn set_secret(app: AppHandle, name: String, value: String) -> Result<(), String> {
    if value.is_empty() {
        return Err("o valor não pode ser vazio".into());
    }
    secrets::set(&name, &value)?;
    secrets::index_add(&config_dir(&app)?, &name)
}

#[tauri::command(async)]
fn delete_secret(app: AppHandle, name: String) -> Result<(), String> {
    secrets::delete(&name)?;
    secrets::index_remove(&config_dir(&app)?, &name)
}

/// Checagem das travas do backup SEM executar — a UI usa pra avisar o usuário
/// antes de ele agendar um espelho que seria recusado toda madrugada.
#[tauri::command(async)]
fn check_backup(source: String, dest: String, mode: String) -> Result<(), String> {
    backup::check_paths(
        Path::new(source.trim()),
        Path::new(dest.trim()),
        backup::BackupMode::parse(&mode),
    )
    .map(|_| ())
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
            start_watch,
            stop_watch,
            list_secrets,
            set_secret,
            delete_secret,
            check_backup,
        ])
        .run(tauri::generate_context!())
        // Falha aqui é fatal por definição: sem o runtime Tauri não há app.
        .expect("erro ao iniciar a aplicação Tauri");
}
