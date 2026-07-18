import { describe, expect, it } from "vitest";
import {
  baseName,
  fromTflow,
  isBackgroundTrigger,
  outputPorts,
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
    // horário diferente → não dispara
    const at8 = new Date(2026, 6, 17, 8, 30, 0);
    expect(scheduleDue(at8, "09:00", "")).toBe(false);
  });
});
