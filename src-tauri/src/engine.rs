//! Executor do fluxo (DAG) — v0.1: execução manual, sequencial, com log por
//! nó via eventos (`flow-log`/`flow-done`). SEM sandbox de propósito
//! (Decisão nº 8: o usuário é responsável pelo que roda na máquina dele).
//! JS dos nós transformar/condição roda no boa (100% Rust).

use std::collections::HashMap;
use std::time::Instant;

use boa_engine::{js_string, property::Attribute, Context, JsValue, Source};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter};

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

fn run_node(node: &FlowNode, input: &Value) -> Result<Value, String> {
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
            let url = cfg_str(node, "url");
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
            for line in cfg_str(node, "headers").lines() {
                if let Some((k, v)) = line.split_once(':') {
                    req = req.header(k.trim(), v.trim());
                }
            }
            let body = cfg_str(node, "body");
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
            let cmdline = cfg_str(node, "command");
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
            let path = cfg_str(node, "path");
            let text = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
            Ok(serde_json::json!({ "path": path, "text": text }))
        }
        "writefile" => {
            let path = cfg_str(node, "path");
            if path.is_empty() {
                return Err("caminho vazio".into());
            }
            let content = cfg_str(node, "content");
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
                let t = cfg_str(node, "title");
                if t.is_empty() { "LocalAutomation".to_string() } else { t }
            };
            let body = cfg_str(node, "message");
            let _ = notify_rust::Notification::new().summary(&title).body(&body).show();
            Ok(input.clone())
        }
        other => Err(format!("tipo de nó desconhecido: {other}")),
    }
}

fn condition_port(node: &FlowNode, input: &Value) -> Result<String, String> {
    let expr = cfg_str(node, "expr");
    if expr.trim().is_empty() {
        return Err("expressão vazia".into());
    }
    let v = run_js(&format!("Boolean({expr})"), input)?;
    Ok(if v.as_bool().unwrap_or(false) { "true".into() } else { "false".into() })
}

/// Executa o fluxo a partir do gatilho (DFS; condição segue só a porta que
/// bateu). Nó com vários pais roda a cada chegada (documentado; merge = v0.2).
pub fn run_flow(app: &AppHandle, run_id: u64, flow: Flow) {
    let nodes: HashMap<String, FlowNode> =
        flow.nodes.iter().map(|n| (n.id.clone(), n.clone())).collect();

    let Some(trigger) = flow.nodes.iter().find(|n| n.kind == "trigger") else {
        let _ = app.emit("flow-done", FlowDone {
            run_id,
            ok: false,
            error: Some("o fluxo precisa de um nó Gatilho".into()),
        });
        return;
    };

    // pilha de (nó, input)
    let mut stack: Vec<(String, Value)> = vec![(trigger.id.clone(), Value::Null)];
    let mut steps = 0u32;
    let mut ok = true;
    let mut first_error: Option<String> = None;

    while let Some((node_id, input)) = stack.pop() {
        steps += 1;
        if steps > 500 {
            ok = false;
            first_error = Some("fluxo passou de 500 passos (ciclo?)".into());
            break;
        }
        let Some(node) = nodes.get(&node_id) else { continue };
        let _ = app.emit("flow-log", FlowLog {
            run_id,
            node_id: node_id.clone(),
            status: "running".into(),
            ms: 0,
            preview: String::new(),
            error: None,
        });
        let started = Instant::now();
        match run_node(node, &input) {
            Ok(output) => {
                let _ = app.emit("flow-log", FlowLog {
                    run_id,
                    node_id: node_id.clone(),
                    status: "ok".into(),
                    ms: started.elapsed().as_millis() as u64,
                    preview: preview(&output),
                    error: None,
                });
                // Sucessores: condição filtra pela porta.
                let port_filter = if node.kind == "condition" {
                    match condition_port(node, &input) {
                        Ok(p) => Some(p),
                        Err(e) => {
                            ok = false;
                            first_error.get_or_insert(format!("{node_id}: {e}"));
                            let _ = app.emit("flow-log", FlowLog {
                                run_id,
                                node_id: node_id.clone(),
                                status: "error".into(),
                                ms: started.elapsed().as_millis() as u64,
                                preview: String::new(),
                                error: Some(e),
                            });
                            continue;
                        }
                    }
                } else {
                    None
                };
                for edge in flow.edges.iter().filter(|e| e.from == node_id) {
                    let pass = match (&port_filter, &edge.port) {
                        (Some(want), Some(have)) => want == have,
                        (Some(_), None) => false,
                        (None, _) => true,
                    };
                    if pass {
                        stack.push((edge.to.clone(), output.clone()));
                    }
                }
            }
            Err(e) => {
                ok = false;
                first_error.get_or_insert(format!("{node_id}: {e}"));
                let _ = app.emit("flow-log", FlowLog {
                    run_id,
                    node_id: node_id.clone(),
                    status: "error".into(),
                    ms: started.elapsed().as_millis() as u64,
                    preview: String::new(),
                    error: Some(e),
                });
                // Erro para ESTE ramo; os outros ramos empilhados continuam.
            }
        }
    }

    let _ = app.emit("flow-done", FlowDone { run_id, ok, error: first_error });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, kind: &str, cfg: &[(&str, &str)]) -> FlowNode {
        FlowNode {
            id: id.into(),
            kind: kind.into(),
            config: cfg.iter().map(|(k, v)| (k.to_string(), Value::String(v.to_string()))).collect(),
        }
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
    fn trigger_payload_json() {
        let n = node("t", "trigger", &[("payload", r#"{"oi": 1}"#)]);
        assert_eq!(run_node(&n, &Value::Null).unwrap(), serde_json::json!({ "oi": 1 }));
        let vazio = node("t2", "trigger", &[]);
        assert_eq!(run_node(&vazio, &Value::Null).unwrap(), serde_json::json!({}));
    }

    #[test]
    fn readfile_writefile_roundtrip() {
        let dir = std::env::temp_dir().join("localautomation-test");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("x.txt").to_string_lossy().into_owned();
        let w = node("w", "writefile", &[("path", path.as_str()), ("content", "olá fluxo")]);
        run_node(&w, &Value::Null).unwrap();
        let r = node("r", "readfile", &[("path", path.as_str())]);
        let out = run_node(&r, &Value::Null).unwrap();
        assert_eq!(out["text"], "olá fluxo");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
