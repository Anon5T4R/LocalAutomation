import type { Edge } from "@xyflow/react";
import { newNodeId, type FlowNode } from "./flow";

/**
 * Modelos prontos = a promessa do João ("converter/legendar todo arquivo que
 * cair nesta pasta") num clique. PORQUÊ estes dois e não os que abrem o
 * LocalScribe/LocalMedia: hoje esses apps NÃO aceitam um arquivo por linha de
 * comando (ignoram argv, sem file-association), então um modelo "manda pro
 * LocalScribe" seria integração FINGIDA. Em vez disso entrego dois modelos 100%
 * reais em cima do gatilho de pasta + nós que já existem:
 *   - notifyFolder: avisa quando um arquivo cai na pasta (pilar provado ponta a
 *     ponta: vigia + notificação).
 *   - commandFolder: roda um comando por arquivo novo (o leigo troca o `echo`
 *     pelo seu conversor real, ex.: ffmpeg) — andaime honesto, sem fingir app.
 * O hand-off de 1 clique pro LocalScribe/LocalMedia fica registrado como
 * próximo passo (exige esses apps aceitarem caminho por argumento).
 */
export type TemplateId = "notifyFolder" | "commandFolder";

export const TEMPLATE_IDS: TemplateId[] = ["notifyFolder", "commandFolder"];

export function buildTemplate(id: TemplateId): { nodes: FlowNode[]; edges: Edge[] } {
  const trig = newNodeId();
  const next = newNodeId();
  const triggerNode: FlowNode = {
    id: trig,
    type: "auto",
    position: { x: 120, y: 150 },
    // Já nasce como gatilho de pasta; falta só o leigo escolher a pasta.
    data: { kind: "trigger", config: { when: "folder", fileTypes: "" } },
  };

  if (id === "notifyFolder") {
    const notifyNode: FlowNode = {
      id: next,
      type: "auto",
      position: { x: 440, y: 150 },
      data: {
        kind: "notify",
        config: { title: "LocalAutomation", message: "Novo arquivo: {{ input.name }}" },
      },
    };
    return { nodes: [triggerNode, notifyNode], edges: [edge(trig, next)] };
  }

  // commandFolder — `echo` demonstra o padrão {{ input.path }}; roda de verdade.
  const commandNode: FlowNode = {
    id: next,
    type: "auto",
    position: { x: 440, y: 150 },
    data: { kind: "command", config: { command: 'echo Novo arquivo: "{{ input.path }}"' } },
  };
  return { nodes: [triggerNode, commandNode], edges: [edge(trig, next)] };
}

function edge(source: string, target: string): Edge {
  return { id: newNodeId(), source, target };
}
