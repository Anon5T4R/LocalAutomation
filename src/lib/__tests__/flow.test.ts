import { describe, expect, it } from "vitest";
import {
  baseName,
  fromTflow,
  isBackgroundTrigger,
  outputPorts,
  initialFiredDay,
  scheduleDue,
  toTflow,
  triggerWhen,
  type FlowNode,
} from "../flow";
import type { Edge } from "@xyflow/react";

describe("tflow — round-trip", () => {
  it("serializa e volta igual", () => {
    const nodes: FlowNode[] = [
      { id: "a", type: "auto", position: { x: 10, y: 20 }, data: { kind: "trigger", config: {} } },
      {
        id: "b",
        type: "auto",
        position: { x: 300, y: 20 },
        data: { kind: "condition", config: { expr: "input.x > 1" } },
      },
    ];
    const edges: Edge[] = [{ id: "e1", source: "a", target: "b", sourceHandle: undefined }];
    const t = toTflow(nodes, edges);
    expect(t.version).toBe(1);
    expect(t.edges[0].port).toBeNull();

    const back = fromTflow(JSON.stringify(t));
    expect(back.nodes).toHaveLength(2);
    expect(back.nodes[1].data.config.expr).toBe("input.x > 1");
    expect(back.edges[0].source).toBe("a");
  });

  it("porta da condição sobrevive", () => {
    const nodes: FlowNode[] = [
      { id: "c", type: "auto", position: { x: 0, y: 0 }, data: { kind: "condition", config: {} } },
      { id: "d", type: "auto", position: { x: 0, y: 0 }, data: { kind: "command", config: {} } },
    ];
    const edges: Edge[] = [{ id: "e", source: "c", target: "d", sourceHandle: "true" }];
    const back = fromTflow(JSON.stringify(toTflow(nodes, edges)));
    expect(back.edges[0].sourceHandle).toBe("true");
  });

  it("tflow inválido dá erro claro", () => {
    expect(() => fromTflow("{}")).toThrow();
  });
});

describe("outputPorts", () => {
  it("condição tem true/false; resto uma saída", () => {
    expect(outputPorts("condition")).toEqual(["true", "false"]);
    expect(outputPorts("http")).toEqual([null]);
  });
});

describe("gatilhos pra leigos", () => {
  it("triggerWhen cai em manual quando ausente/inválido", () => {
    expect(triggerWhen({})).toBe("manual");
    expect(triggerWhen({ when: "folder" })).toBe("folder");
    expect(triggerWhen({ when: "xpto" })).toBe("manual");
  });

  it("só manual NÃO é gatilho de segundo plano", () => {
    expect(isBackgroundTrigger("manual")).toBe(false);
    expect(isBackgroundTrigger("folder")).toBe(true);
    expect(isBackgroundTrigger("interval")).toBe(true);
    expect(isBackgroundTrigger("schedule")).toBe(true);
    expect(isBackgroundTrigger("startup")).toBe(true);
  });

  it("baseName pega o último trecho do caminho (Windows e POSIX)", () => {
    expect(baseName("C:\\Users\\Joao\\Downloads")).toBe("Downloads");
    expect(baseName("/home/joao/videos/")).toBe("videos");
    expect(baseName("Downloads")).toBe("Downloads");
  });

  it("scheduleDue dispara uma vez no horário certo", () => {
    const at9 = new Date(2026, 6, 17, 9, 0, 0); // 09:00
    expect(scheduleDue(at9, "09:00", "")).toBe(true);
    // já disparou hoje → não repete
    expect(scheduleDue(at9, "09:00", at9.toDateString())).toBe(false);
    // antes do horário → não dispara
    const at8 = new Date(2026, 6, 17, 8, 30, 0);
    expect(scheduleDue(at8, "09:00", "")).toBe(false);
  });

  it("scheduleDue cobra o horário mesmo se a batida atrasar", () => {
    // O caso que motivou a mudança de `===` pra `>=`: com a janela escondida o
    // WebView2 estrangula os timers da página, e uma batida pode cair DEPOIS do
    // minuto-alvo. Com casamento exato, o agendamento das 9h simplesmente não
    // acontecia naquele dia — sem erro nenhum.
    const at9h02 = new Date(2026, 6, 17, 9, 2, 0);
    expect(scheduleDue(at9h02, "09:00", "")).toBe(true);
    // Batida MUITO atrasada (uma hora) ainda cobra.
    const at10 = new Date(2026, 6, 17, 10, 0, 0);
    expect(scheduleDue(at10, "09:00", "")).toBe(true);
    // Mas só uma vez por dia.
    expect(scheduleDue(at10, "09:00", at10.toDateString())).toBe(false);
    // E o dia seguinte volta a valer.
    const amanha = new Date(2026, 6, 18, 9, 0, 0);
    expect(scheduleDue(amanha, "09:00", at10.toDateString())).toBe(true);
  });

  it("initialFiredDay impede o disparo retroativo ao armar", () => {
    // Armar às 14h um agendamento das 9h NÃO pode rodar o fluxo na hora — o
    // `>=` recupera batida perdida pra frente, não o passado.
    const at14 = new Date(2026, 6, 17, 14, 0, 0);
    const marcado = initialFiredDay(at14, "09:00");
    expect(marcado).toBe(at14.toDateString());
    expect(scheduleDue(at14, "09:00", marcado)).toBe(false);
    // Já armar às 8h deixa o dia em aberto: as 9h ainda vão chegar.
    const at8 = new Date(2026, 6, 17, 8, 0, 0);
    expect(initialFiredDay(at8, "09:00")).toBe("");
    expect(scheduleDue(new Date(2026, 6, 17, 9, 0, 0), "09:00", "")).toBe(true);
  });

  it("scheduleDue compara HH:MM com zero à esquerda (não como número)", () => {
    // "9:05" > "10:00" se a comparação fosse ingênua; com zero-preenchimento a
    // ordem de string é a ordem do relógio.
    const at9h05 = new Date(2026, 6, 17, 9, 5, 0);
    expect(scheduleDue(at9h05, "10:00", "")).toBe(false);
    const at10 = new Date(2026, 6, 17, 10, 0, 0);
    expect(scheduleDue(at10, "09:30", "")).toBe(true);
  });
});
