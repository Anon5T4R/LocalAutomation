import { describe, expect, it } from "vitest";
import { fromTflow, outputPorts, toTflow, type FlowNode } from "../flow";
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
