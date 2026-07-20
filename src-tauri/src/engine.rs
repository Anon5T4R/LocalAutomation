//! Executor do fluxo (DAG) — execução sequencial, com log por nó via eventos
//! (`flow-log`/`flow-done`). SEM sandbox de propósito (Decisão nº 8: o usuário
//! é responsável pelo que roda na máquina dele). JS dos nós
//! transformar/condição roda no boa (100% Rust).
//!
//! Três coisas nasceram na v0.6 e mudam a forma do executor:
//!  - **Segredos**: `{{ secret.NOME }}` NÃO passa pelo JS — é resolvido no
//!    cofre do SO e o valor é redigido de todo log e todo erro (`Redactor`).
//!  - **Retry**: qualquer nó aceita `retries`/`retryDelayMs`; a espera dobra a
//!    cada tentativa (backoff exponencial).
//!  - **Repetir**: o nó `repeat` roda o RAMO seguinte N vezes, com condição de
//!    parada. Isso obrigou a trocar a pilha de DFS por recursão — a pilha não
//!    consegue "esperar o ramo terminar pra decidir se repete".

use std::collections::HashMap;
use std::time::{Duration, Instant};

use boa_engine::{js_string, property::Attribute, Context, JsValue, Source};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter};

use crate::backup;
use crate::secrets::{self, Vault};

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FlowNode {
    pub id: String,
    pub kind: String,
    #[serde(default)]
    pub config: HashMap<String, Value>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FlowEdge {
    pub from: String,
    pub to: String,
    /// Porta de saída ("true"/"false" no nó condição; null nos demais).
    #[serde(default)]
    pub port: Option<String>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Flow {
    pub nodes: Vec<FlowNode>,
    pub edges: Vec<FlowEdge>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FlowLog {
    pub run_id: u64,
    pub node_id: String,
    pub status: String, // running | ok | error
    pub ms: u64,
    pub preview: String,
    pub error: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FlowDone {
    pub run_id: u64,
    pub ok: bool,
    pub error: Option<String>,
}

fn cfg_str(node: &FlowNode, key: &str) -> String {
    node.config
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn cfg_u64(node: &FlowNode, key: &str, default: u64, max: u64) -> u64 {
    let raw = node.config.get(key);
    let n = match raw {
        Some(Value::Number(n)) => n.as_u64(),
        Some(Value::String(s)) if !s.trim().is_empty() => s.trim().parse::<u64>().ok(),
        _ => None,
    };
    n.unwrap_or(default).min(max)
}

/// Substitui `{{ expr }}` no texto. Duas naturezas de expressão:
///  - `secret.NOME` → valor do cofre do SO (NUNCA passa pelo motor JS: se o
///    segredo virasse global do boa, qualquer nó "Transformar" do fluxo poderia
///    lê-lo e mandar pra onde quisesse);
///  - qualquer outra → JS sobre o `input` (ex.: `input.json.nome`).
///
/// Sem `{{}}` o texto passa intacto. String → literal; null → vazio; outros →
/// JSON compacto.
fn interpolate(template: &str, input: &Value, vault: &mut Vault) -> Result<String, String> {
    if !template.contains("{{") {
        return Ok(template.to_string());
    }
    let mut out = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            out.push_str(&rest[start..]); // abertura sem fechamento: literal
            return Ok(out);
        };
        let expr = after[..end].trim();
        let s = if let Some(name) = secrets::secret_ref(expr) {
            vault.resolve(name)?
        } else {
            match run_js(expr, input)? {
                Value::String(s) => s,
                Value::Null => String::new(),
                other => other.to_string(),
            }
        };
        out.push_str(&s);
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

/// `cfg_str` + interpolação de `{{...}}` sobre o input.
fn interp(node: &FlowNode, key: &str, input: &Value, vault: &mut Vault) -> Result<String, String> {
    interpolate(&cfg_str(node, key), input, vault)
}

// --- Retry com backoff ---

/// Política de repetição em caso de falha. `attempts` = total de tentativas
/// (1 = sem retry). Vem de `retries` (tentativas EXTRA) pra a config falar a
/// língua do usuário: "tentar mais 3 vezes".
#[derive(Debug, PartialEq, Eq)]
pub struct RetryCfg {
    pub attempts: u32,
    pub delay_ms: u64,
}

/// Tetos: 10 retentativas e 60 s de espera base. Sem teto, um fluxo agendado
/// com `retries=99999` fica preso pra sempre segurando a thread.
fn retry_cfg(node: &FlowNode) -> RetryCfg {
    RetryCfg {
        attempts: cfg_u64(node, "retries", 0, 10) as u32 + 1,
        delay_ms: cfg_u64(node, "retryDelayMs", 500, 60_000),
    }
}

/// Espera antes da tentativa `n` (0 = a primeira, que não espera). Dobra a cada
/// vez, com teto de 60 s por espera — backoff exponencial saturado.
pub fn backoff_delay(cfg: &RetryCfg, attempt: u32) -> u64 {
    if attempt == 0 || cfg.delay_ms == 0 {
        return 0;
    }
    cfg.delay_ms.saturating_mul(1u64 << attempt.min(20)) .min(60_000)
}

/// Executa `f` até dar certo ou acabarem as tentativas. Devolve o resultado e
/// quantas tentativas gastou. `sleep` é injetado pra o teste medir a espera sem
/// esperar de verdade (o teste que dorme 8 s é o teste que ninguém roda).
pub fn with_retry<F, S>(cfg: &RetryCfg, mut f: F, mut sleep: S) -> (Result<Value, String>, u32)
where
    F: FnMut(u32) -> Result<Value, String>,
    S: FnMut(u64),
{
    let mut last = Err("nenhuma tentativa".to_string());
    for attempt in 0..cfg.attempts.max(1) {
        if attempt > 0 {
            sleep(backoff_delay(cfg, attempt));
        }
        last = f(attempt);
        if last.is_ok() {
            return (last, attempt + 1);
        }
    }
    (last, cfg.attempts.max(1))
}

fn preview(v: &Value) -> String {
    let s = v.to_string();
    if s.chars().count() > 300 {
        format!("{}…", s.chars().take(300).collect::<String>())
    } else {
        s
    }
}

/// Roda `code` JS com `input` global; o valor da última expressão é a saída.
fn run_js(code: &str, input: &Value) -> Result<Value, String> {
    let mut ctx = Context::default();
    let input_js = JsValue::from_json(input, &mut ctx).map_err(|e| e.to_string())?;
    ctx.register_global_property(js_string!("input"), input_js, Attribute::all())
        .map_err(|e| e.to_string())?;
    let result = ctx
        .eval(Source::from_bytes(code))
        .map_err(|e| e.to_string())?;
    if result.is_undefined() || result.is_null() {
        return Ok(Value::Null);
    }
    result.to_json(&mut ctx).map_err(|e| e.to_string())
}

fn run_node(node: &FlowNode, input: &Value, vault: &mut Vault) -> Result<Value, String> {
    match node.kind.as_str() {
        "trigger" => {
            let payload = cfg_str(node, "payload");
            if payload.trim().is_empty() {
                Ok(Value::Object(Default::default()))
            } else {
                serde_json::from_str(&payload).map_err(|e| format!("payload não é JSON: {e}"))
            }
        }
        "http" => {
            let method = cfg_str(node, "method");
            let url = interp(node, "url", input, vault)?;
            if url.is_empty() {
                return Err("URL vazia".into());
            }
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .user_agent("LocalAutomation/0.1")
                .build()
                .map_err(|e| e.to_string())?;
            let m = reqwest::Method::from_bytes(method.to_uppercase().as_bytes())
                .unwrap_or(reqwest::Method::GET);
            let mut req = client.request(m, &url);
            let headers = interp(node, "headers", input, vault)?;
            for line in headers.lines() {
                if let Some((k, v)) = line.split_once(':') {
                    req = req.header(k.trim(), v.trim());
                }
            }
            let body = interp(node, "body", input, vault)?;
            if !body.is_empty() {
                req = req.body(body);
            }
            let resp = req.send().map_err(|e| e.to_string())?;
            let status = resp.status().as_u16();
            let text = resp.text().map_err(|e| e.to_string())?;
            let json: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
            Ok(serde_json::json!({ "status": status, "body": text, "json": json }))
        }
        "command" => {
            let cmdline = interp(node, "command", input, vault)?;
            if cmdline.trim().is_empty() {
                return Err("comando vazio".into());
            }
            let output = if cfg!(windows) {
                std::process::Command::new("cmd").args(["/C", &cmdline]).output()
            } else {
                std::process::Command::new("sh").args(["-c", &cmdline]).output()
            }
            .map_err(|e| e.to_string())?;
            Ok(serde_json::json!({
                "code": output.status.code(),
                "stdout": String::from_utf8_lossy(&output.stdout),
                "stderr": String::from_utf8_lossy(&output.stderr),
            }))
        }
        "readfile" => {
            let path = interp(node, "path", input, vault)?;
            let text = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
            Ok(serde_json::json!({ "path": path, "text": text }))
        }
        "writefile" => {
            let path = interp(node, "path", input, vault)?;
            if path.is_empty() {
                return Err("caminho vazio".into());
            }
            let content = interp(node, "content", input, vault)?;
            let text = if content.is_empty() {
                match input {
                    Value::String(s) => s.clone(),
                    other => serde_json::to_string_pretty(other).unwrap_or_default(),
                }
            } else {
                content
            };
            if let Some(parent) = std::path::Path::new(&path).parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
            }
            std::fs::write(&path, &text).map_err(|e| format!("{path}: {e}"))?;
            Ok(input.clone())
        }
        "transform" => {
            let code = cfg_str(node, "code");
            run_js(&code, input)
        }
        "condition" => {
            // A saída é o próprio input; a PORTA é decidida em run_flow.
            Ok(input.clone())
        }
        "delay" => {
            // Pausa o ramo por N ms (teto de 1 h pra não travar o executor).
            let ms: u64 = cfg_str(node, "ms").trim().parse().unwrap_or(1000);
            std::thread::sleep(std::time::Duration::from_millis(ms.min(3_600_000)));
            Ok(input.clone())
        }
        "notify" => {
            // Notificação da bandeja do SO (via `notify-rust`); passa o input.
            let title = {
                let t = interp(node, "title", input, vault)?;
                if t.is_empty() { "LocalAutomation".to_string() } else { t }
            };
            let body = interp(node, "message", input, vault)?;
            let _ = notify_rust::Notification::new().summary(&title).body(&body).show();
            Ok(input.clone())
        }
        "backup" => {
            let source = interp(node, "source", input, vault)?;
            let dest = interp(node, "dest", input, vault)?;
            if source.trim().is_empty() || dest.trim().is_empty() {
                return Err("escolha a pasta de origem e a de destino".into());
            }
            let mode = backup::BackupMode::parse(&cfg_str(node, "mode"));
            let report = backup::run_backup(
                std::path::Path::new(source.trim()),
                std::path::Path::new(dest.trim()),
                mode,
            )?;
            let mut out = serde_json::to_value(&report).unwrap_or(Value::Null);
            if let Value::Object(ref mut m) = out {
                m.insert("source".into(), Value::String(source));
                m.insert("dest".into(), Value::String(dest));
                m.insert(
                    "mode".into(),
                    Value::String(
                        if mode == backup::BackupMode::Mirror { "mirror" } else { "incremental" }
                            .into(),
                    ),
                );
            }
            Ok(out)
        }
        "repeat" => {
            // O nó em si não faz nada: quem repete o ramo é `run_from`, que é
            // o único que enxerga as arestas de saída.
            Ok(input.clone())
        }
        other => Err(format!("tipo de nó desconhecido: {other}")),
    }
}

/// Máximo de voltas de um `repeat`. Teto por segurança, não por gosto: com o
/// orçamento de passos abaixo, um laço fugido para em vez de rodar pra sempre.
const MAX_REPEAT: u64 = 1_000;

/// Entrada de UMA volta do laço: o input do ramo mais `index`/`count`. Se o
/// input for objeto, os campos entram nele (o `{{ input.path }}` do usuário
/// continua valendo); senão, o valor original fica em `value`.
fn loop_input(base: &Value, index: u64, count: u64) -> Value {
    match base {
        Value::Object(m) => {
            let mut m = m.clone();
            m.insert("index".into(), Value::from(index));
            m.insert("count".into(), Value::from(count));
            Value::Object(m)
        }
        other => serde_json::json!({ "index": index, "count": count, "value": other }),
    }
}

/// Condição de parada do laço, avaliada sobre a saída da volta que acabou.
/// Vazia = nunca para antes da hora.
fn stop_now(node: &FlowNode, last: &Value) -> Result<bool, String> {
    let expr = cfg_str(node, "stopWhen");
    if expr.trim().is_empty() {
        return Ok(false);
    }
    let v = run_js(&format!("Boolean({expr})"), last)?;
    Ok(v.as_bool().unwrap_or(false))
}

fn condition_port(node: &FlowNode, input: &Value) -> Result<String, String> {
    let expr = cfg_str(node, "expr");
    if expr.trim().is_empty() {
        return Err("expressão vazia".into());
    }
    let v = run_js(&format!("Boolean({expr})"), input)?;
    Ok(if v.as_bool().unwrap_or(false) { "true".into() } else { "false".into() })
}

/// Orçamento de passos do fluxo inteiro. Subiu de 500 pra 20.000 quando o
/// `repeat` nasceu: 500 passos era generoso pra um DAG e ridículo pra um laço
/// de 1.000 voltas. Continua sendo a rede que pega ciclo.
const MAX_STEPS: u32 = 20_000;
/// Profundidade máxima da recursão (laço dentro de laço dentro de laço…).
const MAX_DEPTH: u32 = 64;

/// Estado de UMA execução. Existe pra o executor ser testável fora do Tauri:
/// `sink` recebe os logs (no app, vira `app.emit`; no teste, um `Vec`).
struct Ctx<'a> {
    run_id: u64,
    nodes: HashMap<String, FlowNode>,
    edges: Vec<FlowEdge>,
    sink: &'a mut dyn FnMut(FlowLog),
    vault: Vault<'a>,
    /// Espera do backoff. Injetada pra o teste não dormir de verdade.
    sleep: &'a dyn Fn(u64),
    steps: u32,
    ok: bool,
    first_error: Option<String>,
    /// Nós que estão executando AGORA, do gatilho até o atual. Só serve pra
    /// achar ciclo: um nó reaparecer aqui significa que ele é ancestral de si
    /// mesmo. Não é histórico — um nó que roda duas vezes em ramos diferentes
    /// (losango) ou em voltas do `repeat` entra e sai limpo.
    path: Vec<String>,
}

impl Ctx<'_> {
    /// TODO log passa por aqui — é o único ponto onde texto vira evento, e é
    /// por isso que a redação de segredo mora aqui e não em cada nó.
    fn emit(&mut self, node_id: &str, status: &str, ms: u64, preview: String, error: Option<String>) {
        let (preview, error) = if self.vault.redactor.is_empty() {
            (preview, error)
        } else {
            (
                self.vault.redactor.scrub(&preview),
                error.map(|e| self.vault.redactor.scrub(&e)),
            )
        };
        (self.sink)(FlowLog {
            run_id: self.run_id,
            node_id: node_id.to_string(),
            status: status.to_string(),
            ms,
            preview,
            error,
        });
    }

    fn fail(&mut self, node_id: &str, e: String, ms: u64) {
        self.ok = false;
        let scrubbed = self.vault.redactor.scrub(&e);
        self.first_error.get_or_insert(format!("{node_id}: {scrubbed}"));
        self.emit(node_id, "error", ms, String::new(), Some(e));
    }
}

/// Roda um nó e, se ele mandar, o ramo que sai dele. Devolve a saída do ÚLTIMO
/// nó bem-sucedido do ramo — é o que a condição de parada do `repeat` observa.
///
/// Esta camada só cuida do ciclo. Antes, um `a → b → a` estourava a recursão e
/// o usuário lia "fluxo aninhado demais" — diagnóstico que manda procurar
/// aninhamento quando o problema é uma seta ligada de volta. Agora a mensagem
/// nomeia os nós do ciclo, que é o que dá pra consertar na tela.
fn run_from(ctx: &mut Ctx, node_id: &str, input: &Value, depth: u32) -> Option<Value> {
    if let Some(inicio) = ctx.path.iter().position(|n| n == node_id) {
        ctx.ok = false;
        let mut volta: Vec<&str> = ctx.path[inicio..].iter().map(String::as_str).collect();
        volta.push(node_id);
        let desenho = volta.join(" → ");
        ctx.first_error
            .get_or_insert(format!("ciclo no fluxo: {desenho}"));
        return None;
    }
    ctx.path.push(node_id.to_string());
    let saida = run_no_no(ctx, node_id, input, depth);
    ctx.path.pop();
    saida
}

fn run_no_no(ctx: &mut Ctx, node_id: &str, input: &Value, depth: u32) -> Option<Value> {
    ctx.steps += 1;
    if ctx.steps > MAX_STEPS {
        ctx.ok = false;
        ctx.first_error
            .get_or_insert(format!("fluxo passou de {MAX_STEPS} passos (ciclo?)"));
        return None;
    }
    if depth > MAX_DEPTH {
        ctx.ok = false;
        ctx.first_error.get_or_insert("fluxo aninhado demais".into());
        return None;
    }
    let node = ctx.nodes.get(node_id)?.clone();
    ctx.emit(node_id, "running", 0, String::new(), None);

    let started = Instant::now();
    let retry = retry_cfg(&node);
    // A tentativa que não é a primeira aparece no log: sem isso, um fluxo que
    // só funciona na 3ª tentativa parece um fluxo que funciona.
    let mut attempts_used = 1;
    let (result, used) = {
        let vault = &mut ctx.vault;
        let sleeper = ctx.sleep;
        with_retry(
            &retry,
            |_| run_node(&node, input, vault),
            |ms| sleeper(ms),
        )
    };
    attempts_used = attempts_used.max(used);

    let output = match result {
        Ok(o) => o,
        Err(e) => {
            let msg = if attempts_used > 1 {
                format!("{e} (após {attempts_used} tentativas)")
            } else {
                e
            };
            ctx.fail(node_id, msg, started.elapsed().as_millis() as u64);
            return None;
        }
    };
    let mut prev = preview(&output);
    if attempts_used > 1 {
        prev = format!("[tentativa {attempts_used}] {prev}");
    }
    ctx.emit(node_id, "ok", started.elapsed().as_millis() as u64, prev, None);

    // Condição: só a porta que bateu segue.
    let port_filter = if node.kind == "condition" {
        match condition_port(&node, input) {
            Ok(p) => Some(p),
            Err(e) => {
                ctx.fail(node_id, e, started.elapsed().as_millis() as u64);
                return None;
            }
        }
    } else {
        None
    };

    if node.kind == "repeat" {
        let times = cfg_u64(&node, "times", 1, MAX_REPEAT).max(1);
        let mut last: Option<Value> = None;
        for i in 0..times {
            let it = loop_input(&output, i, times);
            let branch = run_successors(ctx, node_id, &it, &port_filter, depth + 1);
            let observed = branch.clone().unwrap_or(it);
            last = branch.or(last);
            match stop_now(&node, &observed) {
                Ok(true) => break,
                Ok(false) => {}
                Err(e) => {
                    ctx.fail(node_id, format!("condição de parada: {e}"), 0);
                    break;
                }
            }
            if !ctx.ok && ctx.steps > MAX_STEPS {
                break;
            }
        }
        return last;
    }

    run_successors(ctx, node_id, &output, &port_filter, depth + 1)
}

/// Roda os sucessores na ordem em que as arestas aparecem no `.tflow` e devolve
/// a saída do último ramo que produziu algo. (A pilha antiga rodava os irmãos
/// ao contrário — ordem de declaração é a que o usuário desenhou.)
fn run_successors(
    ctx: &mut Ctx,
    node_id: &str,
    output: &Value,
    port_filter: &Option<String>,
    depth: u32,
) -> Option<Value> {
    let targets: Vec<String> = ctx
        .edges
        .iter()
        .filter(|e| e.from == node_id)
        .filter(|e| match (port_filter, &e.port) {
            (Some(want), Some(have)) => want == have,
            (Some(_), None) => false,
            (None, _) => true,
        })
        .map(|e| e.to.clone())
        .collect();
    if targets.is_empty() {
        return Some(output.clone());
    }
    let mut last = None;
    for to in targets {
        // Erro para ESTE ramo; os irmãos continuam (comportamento antigo).
        if let Some(v) = run_from(ctx, &to, output, depth) {
            last = Some(v);
        }
    }
    last
}

/// Executa o fluxo. Separado de `run_flow` pra rodar em teste sem Tauri: recebe
/// o cofre (fake nos testes) e o coletor de logs.
pub fn execute(
    run_id: u64,
    flow: Flow,
    vault: Vault<'_>,
    sink: &mut dyn FnMut(FlowLog),
    sleep: &dyn Fn(u64),
) -> FlowDone {
    let Some(trigger) = flow.nodes.iter().find(|n| n.kind == "trigger").cloned() else {
        return FlowDone {
            run_id,
            ok: false,
            error: Some("o fluxo precisa de um nó Gatilho".into()),
        };
    };
    let mut ctx = Ctx {
        run_id,
        nodes: flow.nodes.iter().map(|n| (n.id.clone(), n.clone())).collect(),
        edges: flow.edges.clone(),
        sink,
        vault,
        sleep,
        steps: 0,
        ok: true,
        first_error: None,
        path: Vec::new(),
    };
    run_from(&mut ctx, &trigger.id, &Value::Null, 0);
    FlowDone { run_id, ok: ctx.ok, error: ctx.first_error }
}

/// Executa o fluxo a partir do gatilho e transmite tudo pela ponte do Tauri.
pub fn run_flow(app: &AppHandle, run_id: u64, flow: Flow) {
    let vault = Vault::from_os_keyring();
    let mut sink = |log: FlowLog| {
        let _ = app.emit("flow-log", log);
    };
    let sleep = |ms: u64| std::thread::sleep(Duration::from_millis(ms));
    let done = execute(run_id, flow, vault, &mut sink, &sleep);
    let _ = app.emit("flow-done", done);
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn node(id: &str, kind: &str, cfg: &[(&str, &str)]) -> FlowNode {
        FlowNode {
            id: id.into(),
            kind: kind.into(),
            config: cfg.iter().map(|(k, v)| (k.to_string(), Value::String(v.to_string()))).collect(),
        }
    }

    fn edge(from: &str, to: &str) -> FlowEdge {
        FlowEdge { from: from.into(), to: to.into(), port: None }
    }

    /// Cofre falso. Os testes NUNCA falam com o keyring do SO: o CI Linux não
    /// tem serviço de segredos e o teste ficaria vermelho por ambiente, não por
    /// bug (sonda de capacidade mente — aqui a gente exercita o que controla).
    fn fake_vault(pairs: &'static [(&'static str, &'static str)]) -> Vault<'static> {
        Vault::new(move |name: &str| {
            pairs
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| v.to_string())
                .ok_or_else(|| format!("o segredo \u{201c}{name}\u{201d} não está definido neste computador"))
        })
    }

    fn no_vault() -> Vault<'static> {
        Vault::new(|n: &str| Err(format!("sem cofre no teste: {n}")))
    }

    /// Roda um fluxo inteiro e devolve (veredito, logs).
    fn run(flow: Flow, vault: Vault<'_>) -> (FlowDone, Vec<FlowLog>) {
        let logs = RefCell::new(Vec::new());
        let done = {
            let mut sink = |l: FlowLog| logs.borrow_mut().push(l);
            execute(1, flow, vault, &mut sink, &|_| {})
        };
        (done, logs.into_inner())
    }

    #[test]
    fn js_transform_recebe_input() {
        // boa serializa número inteiro como inteiro (42, não 42.0).
        let out = run_js("input.a + 1", &serde_json::json!({ "a": 41 })).unwrap();
        assert_eq!(out, serde_json::json!(42));
    }

    #[test]
    fn js_objeto_de_saida() {
        let out = run_js("({ dobro: input.n * 2 })", &serde_json::json!({ "n": 21 })).unwrap();
        assert_eq!(out, serde_json::json!({ "dobro": 42 }));
    }

    #[test]
    fn condition_port_true_false() {
        let n = node("c", "condition", &[("expr", "input.x > 10")]);
        assert_eq!(condition_port(&n, &serde_json::json!({ "x": 11 })).unwrap(), "true");
        assert_eq!(condition_port(&n, &serde_json::json!({ "x": 5 })).unwrap(), "false");
    }

    #[test]
    fn interpolate_substitui_e_preserva() {
        let inp = serde_json::json!({ "nome": "Ana", "n": 3, "json": { "id": 7 } });
        let mut v = no_vault();
        assert_eq!(interpolate("Oi {{ input.nome }}!", &inp, &mut v).unwrap(), "Oi Ana!");
        assert_eq!(interpolate("id={{input.json.id}}", &inp, &mut v).unwrap(), "id=7");
        assert_eq!(interpolate("sem template", &inp, &mut v).unwrap(), "sem template");
        // abertura sem fechamento fica literal
        assert_eq!(interpolate("a {{ b", &inp, &mut v).unwrap(), "a {{ b");
        // objeto vira JSON compacto
        assert_eq!(interpolate("{{ input.json }}", &inp, &mut v).unwrap(), "{\"id\":7}");
    }

    #[test]
    fn trigger_payload_json() {
        let mut v = no_vault();
        let n = node("t", "trigger", &[("payload", r#"{"oi": 1}"#)]);
        assert_eq!(run_node(&n, &Value::Null, &mut v).unwrap(), serde_json::json!({ "oi": 1 }));
        let vazio = node("t2", "trigger", &[]);
        assert_eq!(run_node(&vazio, &Value::Null, &mut v).unwrap(), serde_json::json!({}));
    }

    #[test]
    fn readfile_writefile_roundtrip() {
        let mut v = no_vault();
        let dir = std::env::temp_dir().join("localautomation-test");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("x.txt").to_string_lossy().into_owned();
        let w = node("w", "writefile", &[("path", path.as_str()), ("content", "olá fluxo")]);
        run_node(&w, &Value::Null, &mut v).unwrap();
        let r = node("r", "readfile", &[("path", path.as_str())]);
        let out = run_node(&r, &Value::Null, &mut v).unwrap();
        assert_eq!(out["text"], "olá fluxo");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // --- Retry ---

    #[test]
    fn retry_reexecuta_de_verdade_e_para_quando_da_certo() {
        let cfg = RetryCfg { attempts: 5, delay_ms: 100 };
        let chamadas = RefCell::new(0u32);
        let (res, used) = with_retry(
            &cfg,
            |_| {
                let mut c = chamadas.borrow_mut();
                *c += 1;
                if *c < 3 {
                    Err("caiu a rede".into())
                } else {
                    Ok(serde_json::json!("enfim"))
                }
            },
            |_| {},
        );
        assert_eq!(res.unwrap(), serde_json::json!("enfim"));
        assert_eq!(*chamadas.borrow(), 3, "tem que ter re-executado 3 vezes");
        assert_eq!(used, 3);
    }

    #[test]
    fn retry_desiste_na_hora_certa() {
        // 2 retentativas = 3 tentativas no total. Nem 2, nem 4.
        let cfg = RetryCfg { attempts: 3, delay_ms: 10 };
        let chamadas = RefCell::new(0u32);
        let (res, used) = with_retry(
            &cfg,
            |_| {
                *chamadas.borrow_mut() += 1;
                Err::<Value, String>("sempre falha".into())
            },
            |_| {},
        );
        assert!(res.is_err());
        assert_eq!(*chamadas.borrow(), 3, "exatamente 3 tentativas");
        assert_eq!(used, 3);
    }

    #[test]
    fn retry_zero_e_uma_tentativa_so() {
        let cfg = retry_cfg(&node("n", "http", &[]));
        assert_eq!(cfg, RetryCfg { attempts: 1, delay_ms: 500 });
        let chamadas = RefCell::new(0u32);
        let (_, used) = with_retry(
            &cfg,
            |_| {
                *chamadas.borrow_mut() += 1;
                Err::<Value, String>("x".into())
            },
            |_| {},
        );
        assert_eq!(*chamadas.borrow(), 1);
        assert_eq!(used, 1);
    }

    #[test]
    fn backoff_dobra_e_satura() {
        let cfg = RetryCfg { attempts: 9, delay_ms: 1_000 };
        let esperas = RefCell::new(Vec::new());
        with_retry(
            &cfg,
            |_| Err::<Value, String>("nao".into()),
            |ms| esperas.borrow_mut().push(ms),
        );
        // 8 esperas (a 1ª tentativa não espera), dobrando até saturar em 60 s.
        assert_eq!(
            *esperas.borrow(),
            vec![2_000, 4_000, 8_000, 16_000, 32_000, 60_000, 60_000, 60_000]
        );
    }

    #[test]
    fn retry_do_config_respeita_o_teto() {
        let n = node("n", "http", &[("retries", "9999"), ("retryDelayMs", "9999999")]);
        let cfg = retry_cfg(&n);
        assert_eq!(cfg.attempts, 11, "10 retentativas no máximo");
        assert_eq!(cfg.delay_ms, 60_000);
    }

    #[test]
    fn retry_no_fluxo_deixa_rastro_no_log() {
        // Nó que falha na 1a e 2a tentativa: escreve num arquivo que só existe
        // depois. Aqui usamos readfile com retry sobre um arquivo que criamos
        // entre as tentativas — o sleep injetado cria o arquivo.
        let dir = std::env::temp_dir().join("localautomation-retry-fluxo");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let alvo = dir.join("aparece.txt");
        let alvo_s = alvo.to_string_lossy().into_owned();

        let flow = Flow {
            nodes: vec![
                node("t", "trigger", &[]),
                node("r", "readfile", &[("path", alvo_s.as_str()), ("retries", "3"), ("retryDelayMs", "1")]),
            ],
            edges: vec![edge("t", "r")],
        };
        let logs = RefCell::new(Vec::new());
        let esperas = RefCell::new(0u32);
        let done = {
            let mut sink = |l: FlowLog| logs.borrow_mut().push(l);
            let alvo2 = alvo.clone();
            let sleep = move |_ms: u64| {
                let mut n = esperas.borrow_mut();
                *n += 1;
                // Na 2a espera o arquivo aparece: a 3a tentativa tem que achar.
                if *n == 2 {
                    std::fs::write(&alvo2, "chegou").unwrap();
                }
            };
            execute(1, flow, no_vault(), &mut sink, &sleep)
        };
        assert!(done.ok, "erro: {:?}", done.error);
        let ok = logs.borrow().iter().find(|l| l.status == "ok" && l.node_id == "r").cloned().unwrap();
        assert!(ok.preview.starts_with("[tentativa 3]"), "preview foi: {}", ok.preview);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // --- Laço (repeat) ---

    #[test]
    fn repeat_roda_o_ramo_n_vezes() {
        let dir = std::env::temp_dir().join("localautomation-loop-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let padrao = dir.join("v{{ input.index }}.txt").to_string_lossy().into_owned();
        let flow = Flow {
            nodes: vec![
                node("t", "trigger", &[]),
                node("r", "repeat", &[("times", "4")]),
                node("w", "writefile", &[("path", padrao.as_str()), ("content", "volta")]),
            ],
            edges: vec![edge("t", "r"), edge("r", "w")],
        };
        let (done, _) = run(flow, no_vault());
        assert!(done.ok, "erro: {:?}", done.error);
        for i in 0..4 {
            assert!(dir.join(format!("v{i}.txt")).exists(), "faltou a volta {i}");
        }
        assert!(!dir.join("v4.txt").exists(), "rodou uma volta a mais");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn repeat_para_na_condicao_de_parada() {
        let flow = Flow {
            nodes: vec![
                node("t", "trigger", &[]),
                node("r", "repeat", &[("times", "10"), ("stopWhen", "input.n >= 2")]),
                node("x", "transform", &[("code", "({ n: input.index })")]),
            ],
            edges: vec![edge("t", "r"), edge("r", "x")],
        };
        let (done, logs) = run(flow, no_vault());
        assert!(done.ok, "erro: {:?}", done.error);
        // index 0, 1, 2 → para na volta em que n>=2. 3 voltas, não 10.
        let voltas = logs.iter().filter(|l| l.node_id == "x" && l.status == "ok").count();
        assert_eq!(voltas, 3);
    }

    #[test]
    fn repeat_sem_times_roda_uma_vez() {
        let flow = Flow {
            nodes: vec![
                node("t", "trigger", &[]),
                node("r", "repeat", &[]),
                node("x", "transform", &[("code", "1")]),
            ],
            edges: vec![edge("t", "r"), edge("r", "x")],
        };
        let (_, logs) = run(flow, no_vault());
        assert_eq!(logs.iter().filter(|l| l.node_id == "x" && l.status == "ok").count(), 1);
    }

    #[test]
    fn ciclo_no_grafo_e_nomeado() {
        // a -> b -> a. Sem isto, o app trava; com só o orçamento de passos, o
        // usuário lia "fluxo aninhado demais" e ia procurar aninhamento.
        let flow = Flow {
            nodes: vec![
                node("t", "trigger", &[]),
                node("a", "transform", &[("code", "1")]),
                node("b", "transform", &[("code", "2")]),
            ],
            edges: vec![edge("t", "a"), edge("a", "b"), edge("b", "a")],
        };
        let (done, _) = run(flow, no_vault());
        assert!(!done.ok);
        // A mensagem entra no panic: um assert que só diz "false" manda
        // procurar no lugar errado quando o erro é OUTRO erro.
        let erro = done.error.unwrap();
        assert!(erro.contains("ciclo"), "erro inesperado: {erro}");
        // Nomear os nós é o ponto: é o que dá pra consertar na tela.
        assert!(erro.contains("a → b → a"), "ciclo sem os nós: {erro}");
    }

    #[test]
    fn losango_nao_e_confundido_com_ciclo() {
        // t -> a -> c e t -> b -> c. O `c` roda DUAS vezes, mas nunca sendo
        // ancestral de si mesmo — se o detector olhasse histórico em vez de
        // caminho, este fluxo legítimo quebraria.
        let flow = Flow {
            nodes: vec![
                node("t", "trigger", &[]),
                node("a", "transform", &[("code", "1")]),
                node("b", "transform", &[("code", "2")]),
                node("c", "transform", &[("code", "3")]),
            ],
            edges: vec![
                edge("t", "a"),
                edge("t", "b"),
                edge("a", "c"),
                edge("b", "c"),
            ],
        };
        let (done, logs) = run(flow, no_vault());
        assert!(done.ok, "erro: {:?}", done.error);
        assert_eq!(
            logs.iter().filter(|l| l.node_id == "c" && l.status == "ok").count(),
            2
        );
    }

    // --- Segredos: o valor não pode aparecer em lugar nenhum ---

    #[test]
    fn segredo_resolve_na_interpolacao_e_nao_entra_no_js() {
        let mut v = fake_vault(&[("TOKEN", "ghp_naopodevazar")]);
        let out = interpolate("Bearer {{ secret.TOKEN }}", &Value::Null, &mut v).unwrap();
        assert_eq!(out, "Bearer ghp_naopodevazar");

        // O motor JS não conhece `secret`: um nó Transformar não consegue lê-lo.
        let err = run_js("secret.TOKEN", &Value::Null).unwrap_err();
        assert!(!err.contains("ghp_naopodevazar"));
    }

    #[test]
    fn segredo_nao_vaza_no_log_de_sucesso() {
        // O comando ECOA o segredo — é o pior caso, e o que um usuário faz sem
        // pensar ao depurar. O preview vai pro painel e pro histórico.
        let flow = Flow {
            nodes: vec![
                node("t", "trigger", &[]),
                node("c", "command", &[("command", "echo tok={{ secret.TOKEN }}")]),
            ],
            edges: vec![edge("t", "c")],
        };
        let (done, logs) = run(flow, fake_vault(&[("TOKEN", "ghp_naopodevazar")]));
        assert!(done.ok, "erro: {:?}", done.error);
        let ok = logs.iter().find(|l| l.node_id == "c" && l.status == "ok").unwrap();
        assert!(ok.preview.contains("tok="), "o comando tem que ter rodado mesmo: {}", ok.preview);
        assert!(
            !ok.preview.contains("ghp_naopodevazar"),
            "SEGREDO VAZOU no preview: {}",
            ok.preview
        );
        assert!(ok.preview.contains("segredo:TOKEN"));
        // E em NENHUM log da execução.
        for l in &logs {
            assert!(!l.preview.contains("ghp_naopodevazar"));
            assert!(!l.error.clone().unwrap_or_default().contains("ghp_naopodevazar"));
        }
    }

    #[test]
    fn segredo_nao_vaza_no_erro_nem_no_veredito() {
        // O caminho do erro é o que costuma vazar: a mensagem do SO repete o
        // caminho/URL inteiro, com o segredo dentro.
        let flow = Flow {
            nodes: vec![
                node("t", "trigger", &[]),
                node(
                    "r",
                    "readfile",
                    &[("path", "/pasta/que/nao/existe/{{ secret.TOKEN }}.txt")],
                ),
            ],
            edges: vec![edge("t", "r")],
        };
        let (done, logs) = run(flow, fake_vault(&[("TOKEN", "ghp_naopodevazar")]));
        assert!(!done.ok);
        let err = done.error.unwrap();
        assert!(!err.contains("ghp_naopodevazar"), "SEGREDO VAZOU no veredito: {err}");
        assert!(err.contains("segredo:TOKEN"));
        let log_err = logs.iter().find(|l| l.status == "error").unwrap();
        let msg = log_err.error.clone().unwrap();
        assert!(!msg.contains("ghp_naopodevazar"), "SEGREDO VAZOU no log: {msg}");
    }

    #[test]
    fn segredo_ausente_da_erro_de_gente_sem_inventar_valor() {
        let mut v = fake_vault(&[]);
        let err = interpolate("{{ secret.SUMIU }}", &Value::Null, &mut v).unwrap_err();
        assert!(err.contains("SUMIU"));
        assert!(err.contains("não está definido"));
    }

    // --- Backup pelo nó do fluxo (o motor de verdade está em backup.rs) ---

    #[test]
    fn no_backup_roda_num_diretorio_temporario_de_verdade() {
        let base = std::env::temp_dir().join("localautomation-no-backup");
        let _ = std::fs::remove_dir_all(&base);
        let src = base.join("origem");
        let dst = base.join("destino");
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("a.txt"), "conteúdo A").unwrap();
        std::fs::write(src.join("sub/b.txt"), "conteúdo B").unwrap();

        let mut v = no_vault();
        let n = node(
            "b",
            "backup",
            &[
                ("source", src.to_string_lossy().as_ref()),
                ("dest", dst.to_string_lossy().as_ref()),
                ("mode", "incremental"),
            ],
        );
        let out = run_node(&n, &Value::Null, &mut v).unwrap();
        assert_eq!(out["copied"], 2);
        assert_eq!(out["mode"], "incremental");
        assert_eq!(std::fs::read_to_string(dst.join("a.txt")).unwrap(), "conteúdo A");
        assert_eq!(std::fs::read_to_string(dst.join("sub/b.txt")).unwrap(), "conteúdo B");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn no_backup_sem_pastas_da_erro_claro() {
        let mut v = no_vault();
        let n = node("b", "backup", &[("source", ""), ("dest", "")]);
        let err = run_node(&n, &Value::Null, &mut v).unwrap_err();
        assert!(err.contains("origem"), "erro foi: {err}");
    }
}
