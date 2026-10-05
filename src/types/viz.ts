// M212 Visualization 类型（与 prism-viz VizGraph serde 契约一一对应）。

export type VizNodeKind =
  | "project"
  | "directory"
  | "file"
  | "symbol"
  | "package"
  | "scene"
  | "godot_node"
  | "script"
  | "resource"
  | "unknown";

export type VizStatus = "complete" | "partial" | "unknown" | "unavailable";

export interface VizNode {
  id: string;
  entity_id: string;
  kind: VizNodeKind;
  label: string;
  parent: string | null;
  status: VizStatus;
  metadata: Record<string, string>;
}

export interface VizEdge {
  id: string;
  relation: string;
  source: string;
  target: string;
  status: VizStatus;
  metadata: Record<string, string>;
}

export interface VizLegendEntry {
  key: string;
  description: string;
}

export interface VizGraph {
  schema_version: number;
  view: string;
  status: VizStatus;
  nodes: VizNode[];
  edges: VizEdge[];
  legend: VizLegendEntry[];
  truncated: boolean;
  diagnostics: string[];
}

export type VizView = "architecture" | "godot" | "dependency";
