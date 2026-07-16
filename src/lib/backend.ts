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
