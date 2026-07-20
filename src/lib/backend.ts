import { invoke } from "@tauri-apps/api/core";

/** Rodando dentro do Tauri? (o smoke em navegador puro não tem a ponte.) */
export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export interface FlowLog {
  runId: number;
  nodeId: string;
  status: "running" | "ok" | "error";
  ms: number;
  preview: string;
  error: string | null;
}

export interface FlowDone {
  runId: number;
  ok: boolean;
  error: string | null;
}

export function runFlow(flowJson: string): Promise<number> {
  return invoke("run_flow", { flowJson });
}

export function readFlow(path: string): Promise<string> {
  return invoke("read_flow", { path });
}

export function writeFlow(path: string, content: string): Promise<void> {
  return invoke("write_flow", { path, content });
}

export function getStartupFile(): Promise<string | null> {
  return invoke("get_startup_file");
}

/** Evento emitido pela vigia quando um arquivo estabiliza numa pasta vigiada. */
export interface WatchFile {
  watchId: string;
  path: string;
  name: string;
  folder: string;
}

/** Liga a vigia de uma pasta (id = id do nó-gatilho). Emite `watch-file`. */
export function startWatch(id: string, folder: string, fileTypes: string[]): Promise<void> {
  return invoke("start_watch", { id, folder, fileTypes });
}

/** Desliga a vigia daquele nó (silencioso se não existir). */
export function stopWatch(id: string): Promise<void> {
  return invoke("stop_watch", { id });
}

// ---------- segundo plano: bandeja, autostart e batida de relógio ----------

/**
 * Avisa o backend se há gatilho armado. Com gatilho armado o Rust passa a
 * emitir `bg-tick` (a batida de relógio dos gatilhos de tempo) e o X da janela
 * minimiza pra bandeja em vez de matar o agendamento.
 */
export function setArmed(armed: boolean): Promise<void> {
  return invoke("set_armed", { armed });
}

/**
 * A intenção de "abrir com o sistema" mora no BACKEND (settings.json na pasta
 * de dados), não no localStorage nem no registro do Windows: o registro é só o
 * efeito, e um efeito que envelhece sozinho quando o exe muda de lugar.
 */
export function autostartGet(): Promise<boolean> {
  return invoke("autostart_get");
}

export function autostartSet(enabled: boolean): Promise<void> {
  return invoke("autostart_set", { enabled });
}

export function closeToTrayGet(): Promise<boolean> {
  return invoke("close_to_tray_get");
}

export function closeToTraySet(enabled: boolean): Promise<void> {
  return invoke("close_to_tray_set", { enabled });
}

/** Manda os rótulos traduzidos pro menu da bandeja (que nasce antes do front). */
export function trayLabelsSet(show: string, quit: string): Promise<void> {
  return invoke("tray_labels_set", { show, quit });
}
