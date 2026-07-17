import { useCallback, useEffect, useRef, useState } from "react";
import {
  Background,
  Controls,
  MiniMap,
  ReactFlow,
  addEdge,
  useEdgesState,
  useNodesState,
  type Connection,
  type Edge,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import AutoNode from "./components/AutoNode";
import ConfigPanel from "./components/ConfigPanel";
import SettingsModal from "./components/SettingsModal";
import Toasts from "./components/Toasts";
import * as backend from "./lib/backend";
import {
  fromTflow,
  newNodeId,
  toTflow,
  NODE_FIELDS,
  type FlowNode,
  type NodeKind,
} from "./lib/flow";
import { t, type MessageKey } from "./lib/i18n";
import { useUi } from "./state/ui";

const nodeTypes = { auto: AutoNode };

interface LogRow {
  nodeId: string;
  status: string;
  ms: number;
  preview: string;
  error: string | null;
}

const KINDS: NodeKind[] = Object.keys(NODE_FIELDS) as NodeKind[];

interface RunRow {
  ts: number;
  ok: boolean;
  error: string | null;
  steps: number;
}

const HIST_KEY = "localautomation.history";
const AUTO_KEY = "localautomation.autoMs";
/** Opções de auto-execução (ms). 0 = desligado. */
const AUTO_OPTIONS = [0, 30_000, 60_000, 300_000, 900_000];

function loadHistory(): RunRow[] {
  try {
    const v = JSON.parse(localStorage.getItem(HIST_KEY) ?? "[]");
    return Array.isArray(v) ? v.slice(0, 40) : [];
  } catch {
    return [];
  }
}

export default function App() {
  const [nodes, setNodes, onNodesChange] = useNodesState<FlowNode>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [filePath, setFilePath] = useState<string | null>(null);
  const [dirty, setDirty] = useState(false);
  const [running, setRunning] = useState(false);
  const [logs, setLogs] = useState<LogRow[]>([]);
  const [importedWarn, setImportedWarn] = useState(false);
  const [history, setHistory] = useState<RunRow[]>(loadHistory);
  const [autoMs, setAutoMs] = useState(() => Number(localStorage.getItem(AUTO_KEY)) || 0);
  const [showHistory, setShowHistory] = useState(false);
  const runIdRef = useRef<number | null>(null);
  const logsRef = useRef<LogRow[]>([]);
  logsRef.current = logs;
  const setSettingsOpen = useUi((s) => s.setSettingsOpen);
  const pushToast = useUi((s) => s.pushToast);

  const markDirty = () => setDirty(true);

  const onConnect = useCallback(
    (c: Connection) => {
      setEdges((eds) => addEdge(c, eds));
      markDirty();
    },
    [setEdges],
  );

  const addNode = (kind: NodeKind) => {
    const id = newNodeId();
    setNodes((ns) => [
      ...ns,
      {
        id,
        type: "auto",
        position: { x: 120 + ns.length * 40, y: 120 + ns.length * 30 },
        data: { kind, config: {} },
      },
    ]);
    setSelectedId(id);
    markDirty();
  };

  const changeConfig = (id: string, key: string, value: string) => {
    setNodes((ns) =>
      ns.map((n) =>
        n.id === id ? { ...n, data: { ...n.data, config: { ...n.data.config, [key]: value } } } : n,
      ),
    );
    markDirty();
  };

  const deleteNode = (id: string) => {
    setNodes((ns) => ns.filter((n) => n.id !== id));
    setEdges((es) => es.filter((e) => e.source !== id && e.target !== id));
    setSelectedId(null);
    markDirty();
  };

  const loadFrom = useCallback(
    async (path: string) => {
      try {
        const raw = await backend.readFlow(path);
        const parsed = fromTflow(raw);
        setNodes(parsed.nodes);
        setEdges(parsed.edges);
        setFilePath(path);
        setDirty(false);
        setLogs([]);
        setImportedWarn(true); // Decisão nº 8: avisar o que o fluxo executa
      } catch (e) {
        pushToast("error", t("toast.openFailed", { error: String(e) }));
      }
    },
    [setNodes, setEdges, pushToast],
  );

  // Boot: .tflow do launch + evento da 2ª instância + eventos de execução.
  useEffect(() => {
    if (!backend.isTauri) return;
    void backend.getStartupFile().then((f) => {
      if (f) void loadFrom(f);
    });
    const un1 = listen<string>("open-flow", (e) => void loadFrom(e.payload));
    const un2 = listen<backend.FlowLog>("flow-log", (e) => {
      if (runIdRef.current !== e.payload.runId) return;
      const { nodeId, status, ms, preview, error } = e.payload;
      setLogs((ls) => {
        // "running" vira linha; ok/error substitui a última do mesmo nó.
        const idx = ls.findIndex((l) => l.nodeId === nodeId && l.status === "running");
        const row = { nodeId, status, ms, preview, error };
        if (status !== "running" && idx >= 0) {
          const next = [...ls];
          next[idx] = row;
          return next;
        }
        return [...ls, row];
      });
      setNodes((ns) =>
        ns.map((n) => (n.id === nodeId ? { ...n, data: { ...n.data, status } } : n)),
      );
    });
    const un3 = listen<backend.FlowDone>("flow-done", (e) => {
      if (runIdRef.current !== e.payload.runId) return;
      setRunning(false);
      const rec: RunRow = {
        ts: Date.now(),
        ok: e.payload.ok,
        error: e.payload.error ?? null,
        steps: logsRef.current.length,
      };
      setHistory((h) => {
        const next = [rec, ...h].slice(0, 40);
        localStorage.setItem(HIST_KEY, JSON.stringify(next));
        return next;
      });
      if (e.payload.ok) pushToast("ok", t("logs.done"));
      else pushToast("error", t("logs.failed", { error: e.payload.error ?? "?" }));
    });
    return () => {
      for (const un of [un1, un2, un3]) void un.then((f) => f());
    };
  }, [loadFrom, pushToast, setNodes]);

  const run = async () => {
    if (running) return;
    if (!nodes.some((n) => n.data.kind === "trigger")) {
      pushToast("error", t("toast.needTrigger"));
      return;
    }
    setLogs([]);
    setImportedWarn(false);
    setNodes((ns) => ns.map((n) => ({ ...n, data: { ...n.data, status: undefined } })));
    setRunning(true);
    try {
      runIdRef.current = await backend.runFlow(JSON.stringify(toTflow(nodes, edges)));
    } catch (e) {
      setRunning(false);
      pushToast("error", t("toast.runFailed", { error: String(e) }));
    }
  };

  // Auto-execução: enquanto o app está aberto, roda o fluxo a cada N ms (sem
  // sobrepor uma execução em andamento). Gatilhos em background/bandeja = v0.4.
  const autoRef = useRef<{ run: () => Promise<void>; running: boolean; hasTrigger: boolean }>({
    run,
    running,
    hasTrigger: false,
  });
  autoRef.current = { run, running, hasTrigger: nodes.some((n) => n.data.kind === "trigger") };
  useEffect(() => {
    if (!autoMs || !backend.isTauri) return;
    const id = setInterval(() => {
      const s = autoRef.current;
      if (s.hasTrigger && !s.running) void s.run();
    }, autoMs);
    return () => clearInterval(id);
  }, [autoMs]);

  const changeAuto = (ms: number) => {
    localStorage.setItem(AUTO_KEY, String(ms));
    setAutoMs(ms);
  };

  const clearHistory = () => {
    localStorage.setItem(HIST_KEY, "[]");
    setHistory([]);
  };

  const doSave = async (forcePick: boolean) => {
    let dest = filePath;
    if (!dest || forcePick) {
      const picked = await save({
        title: t("dlg.saveTitle"),
        defaultPath: "fluxo.tflow",
        filters: [{ name: "tflow", extensions: ["tflow"] }],
      });
      if (typeof picked !== "string") return;
      dest = picked;
    }
    try {
      await backend.writeFlow(dest, JSON.stringify(toTflow(nodes, edges), null, 2));
      setFilePath(dest);
      setDirty(false);
      pushToast("ok", t("toast.saved", { path: dest }));
    } catch (e) {
      pushToast("error", t("toast.saveFailed", { error: String(e) }));
    }
  };

  const doOpen = async () => {
    if (dirty && !window.confirm(t("confirm.discard"))) return;
    const picked = await open({
      title: t("dlg.openTitle"),
      filters: [{ name: "tflow", extensions: ["tflow"] }],
    });
    if (typeof picked === "string") void loadFrom(picked);
  };

  const doNew = () => {
    if (dirty && !window.confirm(t("confirm.discard"))) return;
    setNodes([]);
    setEdges([]);
    setFilePath(null);
    setDirty(false);
    setLogs([]);
    setImportedWarn(false);
  };

  const selected = nodes.find((n) => n.id === selectedId) ?? null;
  const fileName = filePath?.split(/[\\/]/).pop() ?? t("top.untitled");

  return (
    <div className="app">
      <div className="toolbar">
        <button onClick={doNew}>{t("top.new")}</button>
        <button onClick={() => void doOpen()}>{t("top.open")}</button>
        <button onClick={() => void doSave(false)}>{t("top.save")}</button>
        <button onClick={() => void doSave(true)}>{t("top.saveAs")}</button>
        <span className="file-name muted">
          {fileName} {dirty && t("top.modified")}
        </span>
        <span className="toolbar-fill" />
        <label className="auto-run" title={t("top.autoHint")}>
          <span className="muted">{t("top.auto")}</span>
          <select value={autoMs} onChange={(e) => changeAuto(Number(e.target.value))}>
            {AUTO_OPTIONS.map((ms) => (
              <option key={ms} value={ms}>
                {ms === 0 ? t("top.autoOff") : ms < 60_000 ? `${ms / 1000}s` : `${ms / 60_000}min`}
              </option>
            ))}
          </select>
        </label>
        <button className="primary" disabled={running} onClick={() => void run()}>
          {running ? t("top.running") : t("top.run")}
        </button>
        <button title={t("top.settingsTitle")} onClick={() => setSettingsOpen(true)}>
          ⚙
        </button>
      </div>

      {importedWarn && <div className="banner warn">{t("warn.imported")}</div>}

      <div className="main">
        <aside className="palette">
          <div className="palette-title muted">{t("palette.title")}</div>
          {KINDS.map((k) => (
            <button key={k} onClick={() => addNode(k)}>
              {t(`node.${k}` as MessageKey)}
            </button>
          ))}
        </aside>

        <div className="canvas">
          <ReactFlow
            nodes={nodes}
            edges={edges}
            nodeTypes={nodeTypes}
            onNodesChange={(ch) => {
              onNodesChange(ch);
              if (ch.some((c) => c.type === "position" || c.type === "remove")) markDirty();
            }}
            onEdgesChange={(ch) => {
              onEdgesChange(ch);
              if (ch.some((c) => c.type === "remove")) markDirty();
            }}
            onConnect={onConnect}
            onSelectionChange={(sel) => setSelectedId(sel.nodes[0]?.id ?? null)}
            deleteKeyCode={["Delete", "Backspace"]}
            fitView
            proOptions={{ hideAttribution: true }}
          >
            <Background gap={18} />
            <MiniMap pannable zoomable />
            <Controls />
          </ReactFlow>
        </div>

        <ConfigPanel node={selected} onChange={changeConfig} onDelete={deleteNode} />
      </div>

      <div className="logs">
        <div className="logs-head">
          <strong>{t("logs.title")}</strong>
          {logs.length > 0 && (
            <button className="small-btn" onClick={() => setLogs([])}>
              {t("logs.clear")}
            </button>
          )}
          <span className="toolbar-fill" />
          <button className="small-btn" onClick={() => setShowHistory((v) => !v)}>
            {t("history.title")} ({history.length})
          </button>
        </div>
        {showHistory && (
          <div className="history-panel">
            <div className="history-head">
              <span className="muted small">{t("history.subtitle")}</span>
              {history.length > 0 && (
                <button className="small-btn" onClick={clearHistory}>
                  {t("logs.clear")}
                </button>
              )}
            </div>
            {history.length === 0 && <span className="muted small">{t("history.empty")}</span>}
            {history.map((r, i) => (
              <div key={i} className={`history-row ${r.ok ? "ok" : "error"}`}>
                <span className={`hist-dot ${r.ok ? "ok" : "error"}`} />
                <span className="hist-time">{new Date(r.ts).toLocaleString()}</span>
                <span className="hist-steps muted">{t("history.steps", { n: r.steps })}</span>
                <span className="hist-msg muted">{r.error ?? (r.ok ? t("history.ok") : "")}</span>
              </div>
            ))}
          </div>
        )}
        <div className="logs-body">
          {logs.length === 0 && <span className="muted">{t("logs.empty")}</span>}
          {logs.map((l, i) => {
            const node = nodes.find((n) => n.id === l.nodeId);
            const name = node ? t(`node.${node.data.kind}` as MessageKey) : l.nodeId;
            return (
              <div key={i} className={`log-row ${l.status}`}>
                <span className="log-node">{name}</span>
                <span className="log-status">
                  {l.status === "running" ? t("logs.running") : `${l.status} · ${l.ms} ms`}
                </span>
                <span className="log-preview">{l.error ?? l.preview}</span>
              </div>
            );
          })}
        </div>
      </div>

      <SettingsModal />
      <Toasts />
    </div>
  );
}
