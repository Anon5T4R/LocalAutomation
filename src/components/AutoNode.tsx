import { Handle, Position, type NodeProps } from "@xyflow/react";
import { hasInput, outputPorts, type FlowNode } from "../lib/flow";
import { t, type MessageKey } from "../lib/i18n";

const KIND_ICON: Record<string, string> = {
  trigger: "▶",
  http: "🌐",
  command: "⌨",
  readfile: "📄",
  writefile: "💾",
  transform: "ƒ",
  condition: "◇",
};

/** Estado da última execução, injetado no data pelo App (colore a borda). */
export type RunStatus = "running" | "ok" | "error" | undefined;

/** Nó do canvas: ícone + nome do tipo + resumo da config + handles. */
export default function AutoNode({ data, selected }: NodeProps<FlowNode>) {
  const kind = data.kind;
  const ports = outputPorts(kind);
  const status = data.status as RunStatus;

  const summary =
    data.config.url ||
    data.config.command ||
    data.config.path ||
    data.config.expr ||
    (data.config.code ? "ƒ(x)" : "");

  return (
    <div className={`auto-node ${selected ? "selected" : ""} ${status ?? ""}`}>
      {hasInput(kind) && <Handle type="target" position={Position.Left} />}
      <div className="node-head">
        <span className="node-icon">{KIND_ICON[kind]}</span>
        <span className="node-title">{t(`node.${kind}` as MessageKey)}</span>
      </div>
      {summary && <div className="node-summary">{summary}</div>}
      {ports.map((port, i) => (
        <Handle
          key={port ?? "out"}
          id={port ?? undefined}
          type="source"
          position={Position.Right}
          style={ports.length > 1 ? { top: `${35 + i * 30}%` } : undefined}
        />
      ))}
      {ports.length > 1 && (
        <div className="node-ports">
          <span>true</span>
          <span>false</span>
        </div>
      )}
    </div>
  );
}
