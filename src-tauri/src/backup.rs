//! Backup de pasta — o nó `backup` e o modelo pronto que **substituem o app
//! LocalBackup** (decisão D5: o backup não ganha app próprio, ganha um nó de
//! primeira classe aqui, agendável pelo gatilho "todo dia às HH:MM").
//!
//! Dois modos, com a diferença dita em uma frase:
//!  - **incremental**: copia o que mudou. NUNCA apaga nada no destino.
//!  - **espelho**: deixa o destino idêntico à origem — o que sumiu da origem
//!    some do destino.
//!
//! O modo espelho é o que pode destruir dado do usuário, então ele tem três
//! travas (todas testadas):
//!  1. origem e destino não podem ser a mesma pasta, nem uma dentro da outra
//!     (espelhar `C:\Fotos` em `C:\Fotos\bkp` copiaria a cópia pra sempre;
//!     espelhar num pai apagaria a própria origem);
//!  2. destino com arquivos que **não** tem o marcador `.localbackup.json` é
//!     recusado — é o caso do leigo que escolhe "Documentos" como destino;
//!  3. pontos de reparse (link simbólico, junção do Windows) são pulados, não
//!     seguidos: a lição do treemap do LocalMonitor é que `is_symlink()` NÃO vê
//!     junção, e a varredura recursionaria pra sempre.

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Serialize;

/// Nome do marcador escrito na raiz do destino. Serve de prova de que aquela
/// pasta é um backup DESTE app — é o que autoriza o espelho a apagar.
pub const MARKER: &str = ".localbackup.json";

/// Tolerância de mtime. FAT/exFAT guardam com 2 s de granularidade; sem isso um
/// pendrive re-copiaria a coleção inteira toda vez.
const MTIME_TOLERANCE_SECS: u64 = 2;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BackupMode {
    Incremental,
    Mirror,
}

impl BackupMode {
    pub fn parse(s: &str) -> BackupMode {
        match s.trim().to_lowercase().as_str() {
            "mirror" | "espelho" => BackupMode::Mirror,
            // Incremental é o default POR SEGURANÇA: config em branco ou
            // escrita errada não pode virar o modo que apaga.
            _ => BackupMode::Incremental,
        }
    }
}

#[derive(Serialize, Default, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BackupReport {
    /// Arquivos que não existiam no destino.
    pub copied: u64,
    /// Arquivos que existiam mas estavam diferentes (tamanho ou data).
    pub updated: u64,
    /// Arquivos iguais — nem tocados. É o número que prova o "incremental".
    pub unchanged: u64,
    /// Apagados no destino (só no modo espelho).
    pub deleted: u64,
    /// Pulados (ponto de reparse, ou erro de leitura de um item só).
    pub skipped: u64,
    /// Bytes efetivamente escritos (não o tamanho da origem).
    pub bytes: u64,
}

/// Caminho absoluto "resolvido" mesmo quando ainda não existe: canonicaliza o
/// ancestral mais próximo que exista e reanexa o resto. PORQUÊ: as travas de
/// contenção precisam comparar maçãs com maçãs (no Windows a canonicalização
/// devolve o prefixo `\\?\`), e o destino do primeiro backup ainda não existe.
fn resolve(p: &Path) -> PathBuf {
    if let Ok(c) = p.canonicalize() {
        return c;
    }
    let mut tail: Vec<OsString> = Vec::new();
    let mut cur = p;
    while let Some(parent) = cur.parent() {
        if let Some(name) = cur.file_name() {
            tail.push(name.to_os_string());
        }
        if let Ok(c) = parent.canonicalize() {
            let mut out = c;
            for t in tail.iter().rev() {
                out.push(t);
            }
            return out;
        }
        cur = parent;
    }
    p.to_path_buf()
}

/// Ponto de reparse? No Windows a junção NÃO é `is_symlink()` — só o atributo
/// do sistema de arquivos a denuncia. Duas versões da mesma pergunta, uma por
/// plataforma, porque `#[cfg(windows)]` numa função chamada do código comum é
/// exatamente o que quebra o job Linux do CI.
#[cfg(windows)]
fn is_reparse(meta: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse(meta: &std::fs::Metadata) -> bool {
    meta.file_type().is_symlink()
}

fn mtime_secs(meta: &std::fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Precisa copiar? Decisão pura (recebe os metadados) pra ser testável sem
/// mexer no relógio do sistema.
fn needs_copy(src: &std::fs::Metadata, dst: &std::fs::Metadata) -> bool {
    if src.len() != dst.len() {
        return true;
    }
    mtime_secs(src) > mtime_secs(dst).saturating_add(MTIME_TOLERANCE_SECS)
}

/// Valida origem/destino ANTES de escrever qualquer coisa. Separada de
/// `run_backup` pra ser testável e pra a UI poder avisar sem executar.
pub fn check_paths(source: &Path, dest: &Path, mode: BackupMode) -> Result<(PathBuf, PathBuf), String> {
    if !source.is_dir() {
        return Err(format!("a pasta de origem não existe: {}", source.display()));
    }
    let s = resolve(source);
    let d = resolve(dest);
    if s == d {
        return Err("origem e destino são a mesma pasta".into());
    }
    if d.starts_with(&s) {
        return Err("o destino está DENTRO da origem — o backup copiaria a si mesmo".into());
    }
    if s.starts_with(&d) {
        return Err("a origem está DENTRO do destino — espelhar apagaria a própria origem".into());
    }
    if mode == BackupMode::Mirror && d.is_dir() {
        let has_marker = d.join(MARKER).exists();
        let non_empty = std::fs::read_dir(&d)
            .map_err(|e| format!("{}: {e}", d.display()))?
            .flatten()
            .any(|e| e.file_name() != OsString::from(MARKER));
        if non_empty && !has_marker {
            return Err(format!(
                "“{}” tem arquivos e não é um backup deste app — espelhar apagaria eles. \
                 Use o modo incremental ou escolha uma pasta vazia.",
                d.display()
            ));
        }
    }
    Ok((s, d))
}

/// Executa o backup. Devolve o relatório; erro só quando NADA pôde ser feito
/// (origem inexistente, trava de segurança). Falha de item isolado conta em
/// `skipped` — um arquivo travado por outro programa não pode abortar o backup
/// inteiro no meio.
pub fn run_backup(source: &Path, dest: &Path, mode: BackupMode) -> Result<BackupReport, String> {
    let (src, dst) = check_paths(source, dest, mode)?;
    std::fs::create_dir_all(&dst).map_err(|e| format!("{}: {e}", dst.display()))?;

    let mut report = BackupReport::default();
    // Caminhos relativos vistos na origem — é a lista que o espelho usa depois
    // pra saber o que sobra no destino.
    let mut seen: HashSet<PathBuf> = HashSet::new();
    copy_dir(&src, &dst, Path::new(""), &mut report, &mut seen)?;

    if mode == BackupMode::Mirror {
        prune(&dst, Path::new(""), &seen, &mut report);
    }

    write_marker(&dst, &src, mode);
    Ok(report)
}

fn copy_dir(
    src_dir: &Path,
    dst_root: &Path,
    rel: &Path,
    report: &mut BackupReport,
    seen: &mut HashSet<PathBuf>,
) -> Result<(), String> {
    let entries = std::fs::read_dir(src_dir).map_err(|e| format!("{}: {e}", src_dir.display()))?;
    for entry in entries {
        let Ok(entry) = entry else {
            report.skipped += 1;
            continue;
        };
        let name = entry.file_name();
        let Ok(meta) = entry.path().symlink_metadata() else {
            report.skipped += 1;
            continue;
        };
        // Link/junção: pula. Seguir daria loop infinito e cópia duplicada.
        if is_reparse(&meta) {
            report.skipped += 1;
            continue;
        }
        let rel_child = rel.join(&name);
        let dst_path = dst_root.join(&rel_child);
        seen.insert(rel_child.clone());

        if meta.is_dir() {
            if std::fs::create_dir_all(&dst_path).is_err() {
                report.skipped += 1;
                continue;
            }
            copy_dir(&entry.path(), dst_root, &rel_child, report, seen)?;
            continue;
        }

        let dst_meta = dst_path.symlink_metadata().ok();
        // Onde a origem tem arquivo e o destino tem PASTA, a cópia falharia
        // calada pra sempre. Tira a pasta antes.
        if dst_meta.as_ref().is_some_and(|m| m.is_dir()) {
            let _ = std::fs::remove_dir_all(&dst_path);
        }
        let existed = dst_meta.as_ref().is_some_and(|m| m.is_file());
        if existed {
            let dm = dst_path.metadata().map_err(|e| e.to_string());
            if let Ok(dm) = dm {
                if !needs_copy(&meta, &dm) {
                    report.unchanged += 1;
                    continue;
                }
            }
        }
        match std::fs::copy(entry.path(), &dst_path) {
            Ok(n) => {
                report.bytes += n;
                if existed {
                    report.updated += 1;
                } else {
                    report.copied += 1;
                }
                // Carrega o mtime junto: sem isso o arquivo copiado nasce
                // "agora" e a comparação por data nunca converge — todo backup
                // seguinte recopiaria tudo. (Foi o que a medição mostrou.)
                if let Ok(t) = meta.modified() {
                    let _ = filetime_set(&dst_path, t);
                }
            }
            Err(_) => report.skipped += 1,
        }
    }
    Ok(())
}

/// Espelho: apaga no destino tudo que não foi visto na origem. Percorre de
/// baixo pra cima (arquivos antes das pastas) pra a pasta poder sumir vazia.
fn prune(dst_root: &Path, rel: &Path, seen: &HashSet<PathBuf>, report: &mut BackupReport) {
    let dir = dst_root.join(rel);
    let Ok(entries) = std::fs::read_dir(&dir) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let rel_child = rel.join(&name);
        // O marcador é nosso; nunca some.
        if rel_child == Path::new(MARKER) {
            continue;
        }
        let Ok(meta) = entry.path().symlink_metadata() else { continue };
        let keep = seen.contains(&rel_child);
        if meta.is_dir() && !is_reparse(&meta) {
            prune(dst_root, &rel_child, seen, report);
            if !keep && std::fs::remove_dir(entry.path()).is_ok() {
                report.deleted += 1;
            }
        } else if !keep && std::fs::remove_file(entry.path()).is_ok() {
            report.deleted += 1;
        }
    }
}

/// Escreve o marcador. Não é fatal se falhar (destino somente-leitura), mas aí
/// o próximo espelho vai recusar — e recusar é o lado certo pra errar.
fn write_marker(dst: &Path, src: &Path, mode: BackupMode) {
    let json = serde_json::json!({
        "app": "LocalAutomation",
        "source": src.display().to_string(),
        "mode": if mode == BackupMode::Mirror { "mirror" } else { "incremental" },
        "updatedAt": SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    });
    let _ = std::fs::write(dst.join(MARKER), serde_json::to_string_pretty(&json).unwrap_or_default());
}

/// Copia o mtime da origem pro arquivo de destino. `std` sabe LER mtime mas não
/// sabe escrever de forma portátil — daí o crate `filetime` (2 arquivos, sem
/// dependência transitiva além do libc/windows-sys que já estão na árvore).
fn filetime_set(path: &Path, t: SystemTime) -> std::io::Result<()> {
    filetime::set_file_mtime(path, filetime::FileTime::from_system_time(t))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Diretório temporário DE VERDADE, único por teste.
    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("localautomation-bkp-{tag}"));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn write(p: &Path, content: &str) {
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(p, content).unwrap();
    }

    fn read(p: &Path) -> String {
        std::fs::read_to_string(p).unwrap()
    }

    #[test]
    fn modo_default_e_o_que_nao_apaga() {
        assert_eq!(BackupMode::parse(""), BackupMode::Incremental);
        assert_eq!(BackupMode::parse("lixo"), BackupMode::Incremental);
        assert_eq!(BackupMode::parse("mirror"), BackupMode::Mirror);
        assert_eq!(BackupMode::parse("Espelho"), BackupMode::Mirror);
    }

    #[test]
    fn backup_real_copia_arvore_e_confere_conteudo() {
        let base = tmp("copia");
        let src = base.join("origem");
        let dst = base.join("destino");
        write(&src.join("a.txt"), "conteúdo A");
        write(&src.join("sub/b.txt"), "conteúdo B");
        write(&src.join("sub/fundo/c.txt"), "conteúdo C");

        let r = run_backup(&src, &dst, BackupMode::Incremental).unwrap();
        assert_eq!(r.copied, 3, "três arquivos novos");
        assert_eq!(r.updated, 0);
        assert_eq!(r.deleted, 0);
        assert!(r.bytes > 0);

        // Conteúdo conferido de verdade, não só a contagem.
        assert_eq!(read(&dst.join("a.txt")), "conteúdo A");
        assert_eq!(read(&dst.join("sub/b.txt")), "conteúdo B");
        assert_eq!(read(&dst.join("sub/fundo/c.txt")), "conteúdo C");
        assert!(dst.join(MARKER).exists(), "marcador tem que nascer");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn segunda_rodada_nao_recopia_nada() {
        let base = tmp("incremental");
        let src = base.join("origem");
        let dst = base.join("destino");
        write(&src.join("a.txt"), "um");
        write(&src.join("b.txt"), "dois");

        let r1 = run_backup(&src, &dst, BackupMode::Incremental).unwrap();
        assert_eq!(r1.copied, 2);

        // MEDIÇÃO que pegou o bug: sem propagar o mtime, aqui vinha 2
        // atualizados em vez de 2 intocados, e todo backup seria integral.
        let r2 = run_backup(&src, &dst, BackupMode::Incremental).unwrap();
        assert_eq!(r2.unchanged, 2, "nada mudou, nada pode ser recopiado");
        assert_eq!(r2.copied, 0);
        assert_eq!(r2.updated, 0);
        assert_eq!(r2.bytes, 0);

        // Muda um arquivo (tamanho diferente) → só ele volta.
        write(&src.join("a.txt"), "um bem maior agora");
        let r3 = run_backup(&src, &dst, BackupMode::Incremental).unwrap();
        assert_eq!(r3.updated, 1);
        assert_eq!(r3.unchanged, 1);
        assert_eq!(read(&dst.join("a.txt")), "um bem maior agora");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn incremental_nunca_apaga_e_espelho_apaga() {
        let base = tmp("espelho");
        let src = base.join("origem");
        let dst = base.join("destino");
        write(&src.join("fica.txt"), "fica");
        write(&src.join("some.txt"), "some");
        run_backup(&src, &dst, BackupMode::Incremental).unwrap();

        std::fs::remove_file(src.join("some.txt")).unwrap();

        // Incremental: o arquivo apagado na origem CONTINUA no destino.
        let r = run_backup(&src, &dst, BackupMode::Incremental).unwrap();
        assert_eq!(r.deleted, 0);
        assert!(dst.join("some.txt").exists(), "incremental não pode apagar");

        // Espelho: some (o destino já tem o marcador, então pode).
        let r = run_backup(&src, &dst, BackupMode::Mirror).unwrap();
        assert_eq!(r.deleted, 1);
        assert!(!dst.join("some.txt").exists());
        assert!(dst.join("fica.txt").exists());
        assert!(dst.join(MARKER).exists(), "o marcador nunca é podado");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn espelho_poda_pasta_inteira_que_sumiu() {
        let base = tmp("poda-pasta");
        let src = base.join("origem");
        let dst = base.join("destino");
        write(&src.join("mantem/x.txt"), "x");
        write(&src.join("sumiu/y.txt"), "y");
        run_backup(&src, &dst, BackupMode::Mirror).unwrap();
        assert!(dst.join("sumiu/y.txt").exists());

        std::fs::remove_dir_all(src.join("sumiu")).unwrap();
        let r = run_backup(&src, &dst, BackupMode::Mirror).unwrap();
        assert_eq!(r.deleted, 2, "o arquivo e a pasta");
        assert!(!dst.join("sumiu").exists());
        assert!(dst.join("mantem/x.txt").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn espelho_recusa_destino_que_nao_e_backup() {
        let base = tmp("trava-destino");
        let src = base.join("origem");
        let docs = base.join("documentos-do-joao");
        write(&src.join("a.txt"), "a");
        write(&docs.join("tese-de-doutorado.docx"), "10 anos de trabalho");

        let err = run_backup(&src, &docs, BackupMode::Mirror).unwrap_err();
        assert!(err.contains("não é um backup deste app"), "erro foi: {err}");
        // E o principal: NADA foi apagado.
        assert!(docs.join("tese-de-doutorado.docx").exists());

        // Incremental na mesma pasta é permitido (não apaga nada).
        let r = run_backup(&src, &docs, BackupMode::Incremental).unwrap();
        assert_eq!(r.copied, 1);
        assert!(docs.join("tese-de-doutorado.docx").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn recusa_origem_e_destino_aninhados() {
        let base = tmp("aninhado");
        let src = base.join("fotos");
        write(&src.join("a.jpg"), "jpg");
        let dentro = src.join("backup");

        let err = run_backup(&src, &dentro, BackupMode::Incremental).unwrap_err();
        assert!(err.contains("DENTRO da origem"), "erro foi: {err}");

        // Ao contrário: a origem dentro do destino.
        let err = run_backup(&src, &base, BackupMode::Mirror).unwrap_err();
        assert!(err.contains("DENTRO do destino"), "erro foi: {err}");

        // Mesma pasta.
        let err = run_backup(&src, &src, BackupMode::Incremental).unwrap_err();
        assert!(err.contains("mesma pasta"), "erro foi: {err}");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn origem_inexistente_da_erro_de_gente() {
        let base = tmp("sem-origem");
        let err = run_backup(&base.join("nao-existe"), &base.join("d"), BackupMode::Incremental)
            .unwrap_err();
        assert!(err.contains("não existe"), "erro foi: {err}");
        let _ = std::fs::remove_dir_all(&base);
    }
}
