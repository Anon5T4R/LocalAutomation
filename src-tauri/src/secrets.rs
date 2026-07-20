//! Segredos das automações no **cofre do SO** (keyring), nunca num JSON de
//! config em texto puro.
//!
//! PORQUÊ este módulo existe: um fluxo real precisa de token de API, senha de
//! SMTP, chave de webhook. Guardar isso no `.tflow` significa que compartilhar
//! o fluxo (ou fazer backup dele) vaza a credencial. Aqui o `.tflow` guarda só
//! o NOME (`{{ secret.TOKEN_GITHUB }}`); o valor mora no keyring do SO — o
//! mesmo padrão do LocalKeys (`keyring::Entry::new(SERVICE, nome)`).
//!
//! Três invariantes que o resto do app depende:
//!  1. **O frontend nunca lê valor.** Não existe comando `get_secret`. A UI só
//!     escreve, apaga e pergunta "existe?". Quem resolve o valor é o motor, em
//!     Rust, na hora de executar o nó.
//!  2. **Valor resolvido é redigido em TODO log e TODO erro** (`Redactor`).
//!     Sem isso, um `Rodar comando` que ecoa o token cospe ele no painel de
//!     execução — e o painel vai pro histórico.
//!  3. **O keyring não lista.** Ele é um mapa por chave; enumerar não faz parte
//!     da API. Então o índice de NOMES (só nomes) vive num JSON ao lado da
//!     config. Perder o índice não perde os segredos, só a listagem.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use zeroize::Zeroizing;

/// Serviço no cofre do SO. Um por app da suíte (o LocalKeys usa o dele).
pub const SERVICE: &str = "LocalAutomation";

/// Marcador que substitui o valor nos logs. Mostra o NOME de propósito: sem
/// ele, "•••••" no log não diz qual credencial passou por ali.
fn mask(name: &str) -> String {
    format!("«segredo:{name}»")
}

/// Nome válido = o que dá pra digitar em `{{ secret.X }}` sem ambiguidade.
/// Recusar cedo evita um segredo gravado com nome que a interpolação nunca
/// encontraria (falha silenciosa clássica).
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Erro do keyring nunca carrega o valor (a API não devolve o segredo no erro),
/// mas passamos por aqui pra ter uma única frase e não vazar o nome do usuário
/// do SO em mensagens de plataforma.
fn ke(e: keyring::Error) -> String {
    format!("cofre do sistema: {e}")
}

pub fn set(name: &str, value: &str) -> Result<(), String> {
    if !valid_name(name) {
        return Err(format!("nome inválido: {name} (use letras, números, _ ou -)"));
    }
    keyring::Entry::new(SERVICE, name)
        .map_err(ke)?
        .set_password(value)
        .map_err(ke)
}

pub fn get(name: &str) -> Result<String, String> {
    if !valid_name(name) {
        return Err(format!("nome inválido: {name}"));
    }
    keyring::Entry::new(SERVICE, name)
        .map_err(ke)?
        .get_password()
        // NÃO repassamos o erro cru: "NoEntry" não diz nada pro leigo.
        .map_err(|_| format!("o segredo “{name}” não está definido neste computador"))
}

pub fn delete(name: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE, name).map_err(ke)?;
    match entry.delete_credential() {
        // Apagar o que já não existe é sucesso: a intenção era "não exista".
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(ke(e)),
    }
}

pub fn exists(name: &str) -> bool {
    keyring::Entry::new(SERVICE, name)
        .and_then(|e| e.get_password())
        .is_ok()
}

// --- Índice de nomes (só nomes; o valor jamais encosta aqui) ---

pub fn index_path(config_dir: &Path) -> PathBuf {
    config_dir.join("segredos.json")
}

pub fn list_names(config_dir: &Path) -> Vec<String> {
    let raw = match std::fs::read_to_string(index_path(config_dir)) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut names: Vec<String> = serde_json::from_str(&raw).unwrap_or_default();
    names.retain(|n| valid_name(n));
    names.sort();
    names.dedup();
    names
}

fn save_names(config_dir: &Path, names: &[String]) -> Result<(), String> {
    std::fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(names).map_err(|e| e.to_string())?;
    std::fs::write(index_path(config_dir), json).map_err(|e| e.to_string())
}

pub fn index_add(config_dir: &Path, name: &str) -> Result<(), String> {
    let mut names = list_names(config_dir);
    if !names.iter().any(|n| n == name) {
        names.push(name.to_string());
        names.sort();
    }
    save_names(config_dir, &names)
}

pub fn index_remove(config_dir: &Path, name: &str) -> Result<(), String> {
    let mut names = list_names(config_dir);
    names.retain(|n| n != name);
    save_names(config_dir, &names)
}

// --- Redação: o valor não sai vivo de nenhum log ou erro ---

/// Guarda os valores que o motor resolveu durante UMA execução e os apaga de
/// qualquer texto que vá pra tela.
///
/// Limite honesto: redigimos o valor **literal**. Se o fluxo transformar o
/// segredo (base64, hash, metade dele), o resultado não é reconhecido — não
/// existe redação perfeita sem marcar dados, e marcar dados é outro projeto.
#[derive(Default)]
pub struct Redactor {
    /// (nome, valor). `Zeroizing` limpa a cópia na memória ao fim da execução.
    seen: Vec<(String, Zeroizing<String>)>,
}

impl Redactor {
    pub fn note(&mut self, name: &str, value: &str) {
        if value.is_empty() || self.seen.iter().any(|(_, v)| v.as_str() == value) {
            return;
        }
        self.seen.push((name.to_string(), Zeroizing::new(value.to_string())));
    }

    /// Troca toda ocorrência de todo valor conhecido pelo marcador.
    /// Os mais longos primeiro: se um segredo contém outro, o curto sozinho
    /// picotaria o longo e deixaria pedaço legível na tela.
    pub fn scrub(&self, text: &str) -> String {
        if self.seen.is_empty() || text.is_empty() {
            return text.to_string();
        }
        let mut order: Vec<&(String, Zeroizing<String>)> = self.seen.iter().collect();
        order.sort_by_key(|(_, v)| std::cmp::Reverse(v.len()));
        let mut out = text.to_string();
        for (name, value) in order {
            if out.contains(value.as_str()) {
                out = out.replace(value.as_str(), &mask(name));
            }
        }
        out
    }

    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }
}

/// Resolve `secret.NOME` durante a execução, com cache e redação embutida.
///
/// O `lookup` é injetado (não é `get` fixo) por dois motivos: teste roda sem
/// cofre do SO nenhum, e o CI Linux não tem serviço de segredos ativo — um
/// teste que falasse com o keyring de verdade seria vermelho por ambiente.
pub struct Vault<'a> {
    lookup: Box<dyn Fn(&str) -> Result<String, String> + 'a>,
    cache: HashMap<String, Zeroizing<String>>,
    pub redactor: Redactor,
}

impl<'a> Vault<'a> {
    pub fn new(lookup: impl Fn(&str) -> Result<String, String> + 'a) -> Self {
        Self { lookup: Box::new(lookup), cache: HashMap::new(), redactor: Redactor::default() }
    }

    /// Vault do app de verdade: fala com o cofre do SO.
    pub fn from_os_keyring() -> Vault<'static> {
        Vault::new(|name: &str| get(name))
    }

    pub fn resolve(&mut self, name: &str) -> Result<String, String> {
        if !valid_name(name) {
            return Err(format!("nome de segredo inválido: {name}"));
        }
        if let Some(v) = self.cache.get(name) {
            return Ok(v.to_string());
        }
        let value = (self.lookup)(name)?;
        // Registra ANTES de devolver: se o nó falhar logo em seguida, o erro
        // já passa pela redação.
        self.redactor.note(name, &value);
        self.cache.insert(name.to_string(), Zeroizing::new(value.clone()));
        Ok(value)
    }
}

/// A expressão `{{ ... }}` é uma referência a segredo? Devolve o nome.
/// Aceita `secret.X`, `secrets.X` e `segredo.X` (o app é PT por padrão; um
/// leigo que escreve em português merece que funcione).
pub fn secret_ref(expr: &str) -> Option<&str> {
    let e = expr.trim();
    for p in ["secret.", "secrets.", "segredo.", "segredos."] {
        if let Some(rest) = e.strip_prefix(p) {
            let name = rest.trim();
            if valid_name(name) {
                return Some(name);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nome_valido_recusa_o_que_a_interpolacao_nao_acharia() {
        assert!(valid_name("TOKEN_GITHUB"));
        assert!(valid_name("api-key-2"));
        assert!(!valid_name(""));
        assert!(!valid_name("com espaço"));
        assert!(!valid_name("ponto.no.meio")); // viraria `secret.a.b`
        assert!(!valid_name(&"x".repeat(65)));
    }

    #[test]
    fn secret_ref_reconhece_os_quatro_prefixos() {
        assert_eq!(secret_ref("secret.TOKEN"), Some("TOKEN"));
        assert_eq!(secret_ref(" secrets.TOKEN "), Some("TOKEN"));
        assert_eq!(secret_ref("segredo.TOKEN"), Some("TOKEN"));
        assert_eq!(secret_ref("segredos.TOKEN"), Some("TOKEN"));
        // expressão JS comum não é segredo
        assert_eq!(secret_ref("input.nome"), None);
        assert_eq!(secret_ref("secret."), None);
    }

    #[test]
    fn redactor_apaga_o_valor_e_mostra_o_nome() {
        let mut r = Redactor::default();
        r.note("TOKEN", "ghp_supersecreto123");
        let scrubbed = r.scrub("curl -H 'Authorization: ghp_supersecreto123' https://api");
        assert!(!scrubbed.contains("ghp_supersecreto123"));
        assert!(scrubbed.contains("«segredo:TOKEN»"));
    }

    #[test]
    fn redactor_trata_o_longo_primeiro() {
        // O curto é substring do longo: na ordem ingênua sobraria "abc" legível.
        let mut r = Redactor::default();
        r.note("CURTO", "abc");
        r.note("LONGO", "abc123def");
        let s = r.scrub("valor=abc123def e outro=abc");
        assert!(!s.contains("abc123def"));
        assert!(s.contains("«segredo:LONGO»"));
        assert!(s.contains("«segredo:CURTO»"));
    }

    #[test]
    fn vault_cacheia_e_registra_pra_redacao() {
        let hits = std::cell::Cell::new(0);
        let mut v = Vault::new(|n: &str| {
            hits.set(hits.get() + 1);
            Ok(format!("valor-de-{n}"))
        });
        assert_eq!(v.resolve("A").unwrap(), "valor-de-A");
        assert_eq!(v.resolve("A").unwrap(), "valor-de-A");
        assert_eq!(hits.get(), 1, "segunda resolução tem que vir do cache");
        assert_eq!(v.redactor.scrub("x=valor-de-A"), "x=«segredo:A»");
    }

    #[test]
    fn indice_guarda_so_nomes_e_ordena() {
        let dir = std::env::temp_dir().join("localautomation-idx-test");
        let _ = std::fs::remove_dir_all(&dir);
        index_add(&dir, "ZETA").unwrap();
        index_add(&dir, "ALFA").unwrap();
        index_add(&dir, "ALFA").unwrap(); // idempotente
        assert_eq!(list_names(&dir), vec!["ALFA", "ZETA"]);
        let raw = std::fs::read_to_string(index_path(&dir)).unwrap();
        assert!(!raw.contains("valor"), "o índice não pode ter valor nenhum");
        index_remove(&dir, "ALFA").unwrap();
        assert_eq!(list_names(&dir), vec!["ZETA"]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
