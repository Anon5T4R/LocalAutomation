import type { Edge, Node } from "@xyflow/react";

/**
 * Formato `.tflow` (JSON do grafo) ↔ estado do React Flow.
 * O executor Rust lê exatamente este formato.
 */

export type NodeKind =
  | "trigger"
  | "http"
  | "command"
  | "readfile"
  | "writefile"
  | "transform"
  | "condition"
  | "delay"
  | "notify";

export interface NodeData {
  kind: NodeKind;
  config: Record<string, string>;
  [key: string]: unknown;
}

export type FlowNode = Node<NodeData>;

export interface Tflow {
  version: 1;
  nodes: { id: string; kind: NodeKind; config: Record<string, string>; x: number; y: number }[];
  edges: { id: string; from: string; to: string; port: string | null }[];
}

export function toTflow(nodes: FlowNode[], edges: Edge[]): Tflow {
  return {
    version: 1,
    nodes: nodes.map((n) => ({
      id: n.id,
      kind: n.data.kind,
      config: n.data.config,
      x: Math.round(n.position.x),
      y: Math.round(n.position.y),
    })),
    edges: edges.map((e) => ({
      id: e.id,
      from: e.source,
      to: e.target,
      port: e.sourceHandle ?? null,
    })),
  };
}

export function fromTflow(raw: string): { nodes: FlowNode[]; edges: Edge[] } {
  const t = JSON.parse(raw) as Tflow;
  if (!Array.isArray(t.nodes) || !Array.isArray(t.edges)) {
    throw new Error("arquivo .tflow inválido");
  }
  return {
    nodes: t.nodes.map((n) => ({
      id: n.id,
      type: "auto",
      position: { x: n.x, y: n.y },
      data: { kind: n.kind, config: n.config ?? {} },
    })),
    edges: t.edges.map((e) => ({
      id: e.id,
      source: e.from,
      target: e.to,
      sourceHandle: e.port ?? undefined,
    })),
  };
}

/** Campos de configuração por tipo de nó (rótulos via i18n `field.*`). */
export interface FieldDef {
  key: string;
  multiline?: boolean;
  mono?: boolean;
  options?: string[];
  placeholder?: string;
}

export const NODE_FIELDS: Record<NodeKind, FieldDef[]> = {
  trigger: [{ key: "payload", multiline: true, mono: true, placeholder: '{ "exemplo": 1 }' }],
  http: [
    { key: "method", options: ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD"] },
    { key: "url", placeholder: "https://…" },
    { key: "headers", multiline: true, mono: true, placeholder: "Content-Type: application/json" },
    { key: "body", multiline: true, mono: true },
  ],
  command: [{ key: "command", mono: true, placeholder: "echo olá" }],
  readfile: [{ key: "path", placeholder: "C:\\pasta\\arquivo.txt" }],
  writefile: [
    { key: "path", placeholder: "C:\\pasta\\saida.txt" },
    { key: "content", multiline: true, placeholder: "(vazio = escreve o input)" },
  ],
  transform: [{ key: "code", multiline: true, mono: true, placeholder: "({ dobro: input.n * 2 })" }],
  condition: [{ key: "expr", mono: true, placeholder: "input.status === 200" }],
  delay: [{ key: "ms", placeholder: "1000" }],
  notify: [
    { key: "title", placeholder: "LocalAutomation" },
    { key: "message", multiline: true },
  ],
};

/** Condição tem duas saídas nomeadas; os demais, uma saída anônima. */
export function outputPorts(kind: NodeKind): (string | null)[] {
  return kind === "condition" ? ["true", "false"] : [null];
}

export function hasInput(kind: NodeKind): boolean {
  return kind !== "trigger";
}

let seq = 1;
export function newNodeId(): string {
  return `n${Date.now().toString(36)}${seq++}`;
}
