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
