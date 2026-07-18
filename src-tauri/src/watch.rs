//! Gatilho "quando um arquivo novo aparecer numa pasta".
//!
//! PORQUÊ este módulo existe: pra um leigo, "escolher uma pasta e ser avisado
//! quando algo cair nela" é a automação mais óbvia do mundo. O difícil não é
//! ver o evento — é ver o evento DIREITO. A vigia do SO (`notify`) é
//! barulhenta: dispara vários eventos por um único arquivo, e dispara no MEIO
//! de uma cópia (o arquivo ainda está sendo escrito). Se a gente disparasse o
//! fluxo no primeiro evento, o leigo ia converter/legendar um arquivo pela
//! metade. Então aqui a gente faz duas coisas que a vigia crua não faz:
//!   1. DEBOUNCE: junta a rajada de eventos do mesmo arquivo em um só.
//!   2. ESTABILIZAR: só dispara quando o tamanho parou de crescer e o arquivo
//!      abre pra leitura (no Windows, cópia em andamento trava o arquivo).
//!
//! A execução do fluxo em si NÃO mora aqui: quando um arquivo estabiliza, a
//! gente emite o evento `watch-file` com o caminho, e o front roda o fluxo com
//! esse arquivo como entrada do gatilho. Assim todo o caminho de execução
//! continua sendo o mesmo (Executar), e este módulo é só o "olho".

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

/// Config de uma vigia. Os tempos têm default de gente no `lib.rs`; ficam aqui
/// explícitos pra o teste conseguir apertar (debounce curto) sem esperar.
#[derive(Clone)]
pub struct WatchConfig {
    pub folder: PathBuf,
    /// Extensões minúsculas SEM ponto (ex.: `["mp4","mkv"]`). Vazio = aceita tudo.
    pub file_types: Vec<String>,
    /// Quanto tempo sem novos eventos + tamanho parado antes de considerar pronto.
    pub debounce_ms: u64,
    /// Teto de espera por arquivo: se depois disso ainda não estabilizou, desiste
    /// (cópia travada/abortada) pra não vazar entrada pendente pra sempre.
    pub timeout_ms: u64,
    /// Intervalo de reavaliação dos pendentes.
    pub poll_ms: u64,
}

/// Arquivo que estabilizou e vai virar disparo.
pub struct FileHit {
    pub path: String,
    pub name: String,
    pub folder: String,
}

/// A extensão do arquivo casa com o filtro? Vazio = aceita tudo.
/// PORQUÊ separado e público: é a regra que o leigo mais vai errar
/// ("só vídeos") — vale um teste dedicado.
pub fn ext_matches(name: &str, types: &[String]) -> bool {
    if types.is_empty() {
        return true;
    }
    let lower = name.to_lowercase();
    types.iter().any(|t| {
        let t = t.trim_start_matches('.').to_lowercase();
        !t.is_empty() && lower.ends_with(&format!(".{t}"))
    })
}

/// Estado de um arquivo que a gente viu mas ainda não disparou.
struct Pending {
    first_seen: Instant,
    last_seen: Instant,
    last_size: u64,
}

/// O que fazer com um pendente AGORA. Função pura (recebe o "agora", o tamanho
/// atual e se abre pra leitura) justamente pra ser testável sem tocar disco nem
/// dormir: o teste injeta tempos e tamanhos e confere a decisão.
#[derive(Debug, PartialEq)]
enum Decision {
    Fire,
    Wait,
    Drop,
}

fn decide(
    p: &mut Pending,
    now: Instant,
    cur_size: Option<u64>,
    readable: bool,
    cfg: &WatchConfig,
) -> Decision {
    // Sumiu antes de estabilizar (apagado/movido) → esquece.
    let Some(size) = cur_size else {
        return Decision::Drop;
    };
    // Demorou demais pra estabilizar (cópia travada) → desiste pra não vazar.
    if now.duration_since(p.first_seen) > Duration::from_millis(cfg.timeout_ms) {
        return Decision::Drop;
    }
    // Ainda chegando evento há pouco → espera a rajada quietar (debounce).
    if now.duration_since(p.last_seen) < Duration::from_millis(cfg.debounce_ms) {
        return Decision::Wait;
    }
    // Tamanho mudou desde a última medição → ainda escrevendo; reinicia a espera.
    if size != p.last_size {
        p.last_size = size;
        p.last_seen = now;
        return Decision::Wait;
    }
    // Tamanho estável há >= debounce. Só falta garantir que não está travado
    // (no Windows, cópia em andamento nega leitura compartilhada).
    if readable {
        Decision::Fire
    } else {
        // Bump do last_seen pra dar mais uma janela de debounce antes de retentar.
        p.last_seen = now;
        Decision::Wait
    }
}

/// Consegue abrir o arquivo pra leitura? Prova concreta de que não está travado
/// no meio de uma cópia (sonda de capacidade: só tentar diz a verdade).
fn readable(path: &PathBuf) -> bool {
    std::fs::File::open(path).is_ok()
}

/// Roda a vigia até `stop` virar true. Bloqueante — o chamador roda numa thread.
/// `on_hit` é chamado UMA vez por arquivo estabilizado. O callback é injetável
/// pra o teste receber os disparos por canal em vez de emitir evento Tauri.
pub fn run_watch(
    cfg: WatchConfig,
    stop: Arc<AtomicBool>,
    on_hit: impl Fn(FileHit),
) -> notify::Result<()> {
    let (tx, rx) = channel();
    let mut watcher =
        RecommendedWatcher::new(move |res| { let _ = tx.send(res); }, notify::Config::default())?;
    watcher.watch(&cfg.folder, RecursiveMode::NonRecursive)?;

    let mut pending: HashMap<PathBuf, Pending> = HashMap::new();
    // Anti-duplo: caminho que acabou de disparar não pode redisparar por um
    // evento atrasado da mesma cópia.
    let mut fired: HashMap<PathBuf, Instant> = HashMap::new();
    let poll = Duration::from_millis(cfg.poll_ms.max(50));
    let refire_guard = Duration::from_millis(cfg.debounce_ms.saturating_mul(4).max(1000));

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        match rx.recv_timeout(poll) {
            Ok(Ok(event)) => {
                if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                    let now = Instant::now();
                    for path in event.paths {
                        if !path.is_file() {
                            continue;
                        }
                        let name = path
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_string();
                        if !ext_matches(&name, &cfg.file_types) {
                            continue;
                        }
                        if fired
                            .get(&path)
                            .is_some_and(|t| now.duration_since(*t) < refire_guard)
                        {
                            continue;
                        }
                        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                        pending
                            .entry(path)
                            .and_modify(|p| p.last_seen = now)
                            .or_insert(Pending { first_seen: now, last_seen: now, last_size: size });
                    }
                }
            }
            Ok(Err(_)) => {} // erro da vigia num evento pontual: ignora
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        // Reavalia todos os pendentes a cada volta (mesmo sem evento novo, pra o
        // debounce/estabilização vencer no tempo).
        let now = Instant::now();
        let mut to_fire = Vec::new();
        let mut to_drop = Vec::new();
        for (path, p) in pending.iter_mut() {
            let cur = std::fs::metadata(&*path).ok().map(|m| m.len());
            match decide(p, now, cur, readable(path), &cfg) {
                Decision::Fire => to_fire.push(path.clone()),
                Decision::Drop => to_drop.push(path.clone()),
                Decision::Wait => {}
            }
        }
        for path in to_drop {
            pending.remove(&path);
        }
        for path in to_fire {
            pending.remove(&path);
            fired.insert(path.clone(), now);
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let folder = path.parent().map(|p| p.display().to_string()).unwrap_or_default();
            on_hit(FileHit { path: path.display().to_string(), name, folder });
        }
        // Poda o anti-duplo pra não crescer sem fim.
        fired.retain(|_, t| now.duration_since(*t) < Duration::from_secs(30));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ext_matches_filtra_por_tipo() {
        let so_video = vec!["mp4".to_string(), "mkv".to_string()];
        assert!(ext_matches("ferias.MP4", &so_video)); // case-insensitive
        assert!(ext_matches("filme.mkv", &so_video));
        assert!(!ext_matches("nota.txt", &so_video));
        assert!(!ext_matches("mp4", &so_video)); // sem ponto não é extensão
        // filtro vazio aceita tudo
        assert!(ext_matches("qualquer.coisa", &[]));
        // ponto no filtro é tolerado (leigo digita ".mp4")
        assert!(ext_matches("x.mp4", &vec![".mp4".to_string()]));
    }

    fn cfg() -> WatchConfig {
        WatchConfig {
            folder: PathBuf::from("."),
            file_types: vec![],
            debounce_ms: 300,
            timeout_ms: 60_000,
            poll_ms: 100,
        }
    }

    #[test]
    fn decide_espera_enquanto_cresce_e_dispara_ao_estabilizar() {
        let t0 = Instant::now();
        let mut p = Pending { first_seen: t0, last_seen: t0, last_size: 100 };
        let c = cfg();
        // logo após o evento: dentro do debounce → espera
        assert_eq!(decide(&mut p, t0 + Duration::from_millis(50), Some(100), true, &c), Decision::Wait);
        // passou o debounce mas o tamanho cresceu → ainda escrevendo → espera
        assert_eq!(decide(&mut p, t0 + Duration::from_millis(400), Some(500), true, &c), Decision::Wait);
        // agora tamanho estável por > debounce e legível → dispara
        assert_eq!(decide(&mut p, t0 + Duration::from_millis(800), Some(500), true, &c), Decision::Fire);
    }

    #[test]
    fn decide_espera_se_estavel_mas_travado() {
        let t0 = Instant::now();
        let mut p = Pending { first_seen: t0, last_seen: t0, last_size: 500 };
        let c = cfg();
        // tamanho parado, debounce vencido, MAS não abre pra leitura → espera
        assert_eq!(decide(&mut p, t0 + Duration::from_millis(400), Some(500), false, &c), Decision::Wait);
    }

    #[test]
    fn decide_desiste_no_timeout_e_se_sumir() {
        let t0 = Instant::now();
        let mut p = Pending { first_seen: t0, last_seen: t0, last_size: 500 };
        let c = cfg();
        // arquivo sumiu (apagado antes de estabilizar)
        assert_eq!(decide(&mut p, t0 + Duration::from_millis(400), None, false, &c), Decision::Drop);
        // passou do teto de espera
        assert_eq!(decide(&mut p, t0 + Duration::from_millis(60_001), Some(500), true, &c), Decision::Drop);
    }

    /// Prova viva: vigia uma pasta de verdade, simula uma cópia LENTA (escreve
    /// em pedaços com pausa), e confere que dispara UMA vez, só depois de
    /// estabilizar, respeitando o filtro de tipo e sem disparo duplo.
    #[test]
    fn vigia_dispara_uma_vez_apos_copia_lenta_respeitando_filtro() {
        use std::io::Write;
        use std::sync::mpsc::channel as chan;

        let dir = std::env::temp_dir().join(format!("la-watch-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let cfg = WatchConfig {
            folder: dir.clone(),
            file_types: vec!["txt".to_string()], // .png deve ser ignorado
            debounce_ms: 300,
            timeout_ms: 60_000,
            poll_ms: 80,
        };
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx) = chan::<String>();
        let stop_w = stop.clone();
        let h = std::thread::spawn(move || {
            let _ = run_watch(cfg, stop_w, move |hit| {
                let _ = tx.send(hit.name);
            });
        });

        // Deixa a vigia assentar antes de mexer na pasta.
        std::thread::sleep(Duration::from_millis(400));

        // Distrator que o filtro deve ignorar.
        std::fs::write(dir.join("ignora.png"), b"nao sou eu").unwrap();

        // "Cópia lenta" do arquivo alvo: cresce em 3 etapas com pausa entre elas.
        let alvo = dir.join("video.txt");
        {
            let mut f = std::fs::File::create(&alvo).unwrap();
            f.write_all(b"aaaa").unwrap();
            f.flush().unwrap();
            std::thread::sleep(Duration::from_millis(250));
            f.write_all(b"bbbb").unwrap();
            f.flush().unwrap();
            std::thread::sleep(Duration::from_millis(250));
            f.write_all(b"cccc").unwrap();
            f.flush().unwrap();
        } // fecha o arquivo → agora estabiliza

        // Deve disparar dentro de alguns segundos, e ser o .txt.
        let got = rx.recv_timeout(Duration::from_secs(10)).expect("gatilho deveria disparar");
        assert_eq!(got, "video.txt");

        // Não pode disparar de novo (nem pro .png, nem duplo pro .txt).
        assert!(
            rx.recv_timeout(Duration::from_secs(2)).is_err(),
            "não deveria disparar uma segunda vez"
        );

        stop.store(true, Ordering::Relaxed);
        let _ = h.join();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
