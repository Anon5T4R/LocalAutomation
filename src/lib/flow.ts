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

/**
 * GATILHOS EM LINGUAGEM DE GENTE.
 *
 * PORQUÊ: antes o "gatilho" era um nó com um campo cru de payload JSON e a
 * única forma de rodar era clicar Executar (ou um "Auto" que repolava o fluxo
 * inteiro a cada N ms). Zero leigo entende isso. Agora o mesmo nó `trigger`
 * ganha um campo `when` que escolhe, em FRASE, quando o fluxo deve rodar. A
 * expressão técnica (payload, intervalo) fica escondida atrás da frase; o modo
 * avançado continua lá pra quem quiser.
 */
export type TriggerWhen = "folder" | "interval" | "schedule" | "startup" | "manual";

/** Ordem de descoberta: o mais concreto (pasta) primeiro; manual por último. */
export const TRIGGER_TYPES: { when: TriggerWhen; icon: string }[] = [
  { when: "folder", icon: "📂" },
  { when: "interval", icon: "🔁" },
  { when: "schedule", icon: "⏰" },
  { when: "startup", icon: "💻" },
  { when: "manual", icon: "▶" },
];

/** Lê o `when` do config, caindo em "manual" se ausente/inválido (compat). */
export function triggerWhen(config: Record<string, string>): TriggerWhen {
  const w = config.when;
  return w === "folder" || w === "interval" || w === "schedule" || w === "startup"
    ? w
    : "manual";
}

/** Gatilho que roda sozinho em segundo plano? (precisa ser "Ativado"). */
export function isBackgroundTrigger(w: TriggerWhen): boolean {
  return w !== "manual";
}

/** Último trecho de um caminho (nome da pasta/arquivo), pra frase amigável. */
export function baseName(path: string): string {
  const parts = path.split(/[\\/]+/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

/**
 * Um agendamento diário já disparou hoje? Decisão pura (recebe o "agora") pra
 * ser testável sem relógio real. `time` = "HH:MM". Dispara quando o minuto
 * atual bate e ainda não disparou nesse dia.
 */
export function scheduleDue(now: Date, time: string, lastFiredDay: string): boolean {
  const hhmm = `${String(now.getHours()).padStart(2, "0")}:${String(
    now.getMinutes(),
  ).padStart(2, "0")}`;
  return hhmm === time && lastFiredDay !== now.toDateString();
}

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
