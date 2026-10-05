import { useEffect, useMemo, useState } from "react";
import { getVisualizationGraph, toPrismError } from "../services/ipc";
import type { VizGraph, VizNode, VizView } from "../types/viz";

/**
 * M212 Visualization 投影面板（§39/§86/§89）。
 *
 * 数据 = prism-viz 投影（get_visualization_graph）；本组件只做
 * 渲染/选择/Inspector/Legend——不自行分析（§3/§86）。
 * Unknown/Partial/截断全部如实呈现（§46/§28）。
 */

const VIEWS: { key: VizView; label: string }[] = [
  { key: "godot", label: "Godot" },
  { key: "architecture", label: "Architecture" },
  { key: "dependency", label: "Dependencies" },
];

const KIND_ICON: Record<string, string> = {
  project: "▣",
  directory: "▸",
  file: "•",
  symbol: "ƒ",
  package: "◆",
  scene: "▤",
  godot_node: "○",
  script: "📜",
  resource: "▪",
  unknown: "?",
};

export default function VizGraphPanel() {
  const [view, setView] = useState<VizView>("godot");
  const [graph, setGraph] = useState<VizGraph | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [selected, setSelected] = useState<VizNode | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    setSelected(null);
    getVisualizationGraph(view)
      .then((result) => {
        if (!cancelled) setGraph(result);
      })
      .catch((err) => {
        if (!cancelled) setError(toPrismError(err).message);
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [view]);

  // 子节点（层级渲染：project → scene/dir → …）。
  const childrenOf = useMemo(() => {
    const map = new Map<string | null, VizNode[]>();
    if (!graph) return map;
    for (const node of graph.nodes) {
      const list = map.get(node.parent) ?? [];
      list.push(node);
      map.set(node.parent, list);
    }
    for (const list of map.values()) {
      list.sort((a, b) => a.label.localeCompare(b.label));
    }
    return map;
  }, [graph]);

  const relatedEdges = useMemo(() => {
    if (!graph || !selected) return [];
    return graph.edges.filter(
      (edge) => edge.source === selected.id || edge.target === selected.id,
    );
  }, [graph, selected]);

  const renderTree = (parent: string | null, depth: number): React.ReactNode[] => {
    if (depth > 6) return [];
    const children = childrenOf.get(parent) ?? [];
    return children.flatMap((node) => [
      <li key={node.id}>
        <button
          type="button"
          className={`viz-node viz-status-${node.status}`}
          data-testid={`viz-node-${node.label}`}
          onClick={() => setSelected(node)}
        >
          <span aria-hidden>{KIND_ICON[node.kind] ?? "?"}</span> {node.label}
          {node.status !== "complete" && (
            <span className="muted small"> [{node.status}]</span>
          )}
        </button>
      </li>,
      ...renderTree(node.id, depth + 1),
    ]);
  };

  if (error) {
    return (
      <div className="card" data-testid="viz-error">
        <p className="muted">Visualization unavailable: {error}</p>
      </div>
    );
  }

  return (
    <div className="card" data-testid="viz-panel">
      <div role="tablist" aria-label="Visualization view">
        {VIEWS.map((entry) => (
          <button
            key={entry.key}
            type="button"
            role="tab"
            aria-selected={view === entry.key}
            onClick={() => setView(entry.key)}
          >
            {entry.label}
          </button>
        ))}
      </div>
      {loading && <p className="muted small">Projecting…</p>}
      {!loading && graph && (
        <>
          {graph.status !== "complete" && (
            <p className="muted small" data-testid="viz-status">
              view status: {graph.status}
            </p>
          )}
          {graph.truncated && (
            <p className="muted small" data-testid="viz-truncated">
              view truncated by resource limits (§28) — not all nodes shown
            </p>
          )}
          <ul data-testid="viz-tree">{renderTree(null, 0)}</ul>
          <div data-testid="viz-legend">
            <p className="muted small">Legend:</p>
            <ul>
              {graph.legend.map((entry) => (
                <li key={entry.key} className="muted small">
                  {entry.key}: {entry.description}
                </li>
              ))}
            </ul>
          </div>
        </>
      )}
      {selected && (
        <div data-testid="viz-inspector">
          <p className="small">
            <strong>{selected.label}</strong> ({selected.kind}, {selected.status})
          </p>
          <p className="muted small">entity: {selected.entity_id}</p>
          {Object.entries(selected.metadata).map(([key, value]) => (
            <p key={key} className="muted small">
              {key}: {value}
            </p>
          ))}
          <p className="muted small">relations: {relatedEdges.length}</p>
          {relatedEdges.map((edge) => (
            <p key={edge.id} className="muted small">
              {edge.relation}: {edge.source} → {edge.target}
            </p>
          ))}
        </div>
      )}
    </div>
  );
}
