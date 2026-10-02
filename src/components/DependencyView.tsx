import { useMemo, useState } from "react";
import { useProjectStore } from "../app/ProjectStore";
import PlaceholderPage from "./PlaceholderPage";
import type { DependencyGraphResultDto, GraphNodeDto } from "../types/prism";

function nodeLabel(graph: DependencyGraphResultDto, id: string): string {
  const node = graph.nodes.find((n) => n.id === id);
  if (node) return node.label;
  // external/builtin 节点 id 形如 node:external:external:name
  const parts = id.split(":");
  return parts[parts.length - 1] ?? id;
}

/**
 * Architecture：依赖与架构视图（M3）。
 * 数据全部来自 DependencyService 的真实解析结果；
 * SVG 图为无第三方依赖的基础渲染，节点上限如实说明（规格 §31/§62）。
 */
export default function DependencyView() {
  const { codeModel, dependencyGraph } = useProjectStore();
  const [selected, setSelected] = useState<GraphNodeDto | null>(null);
  const [nodeKindFilter, setNodeKindFilter] = useState<"all" | "file" | "external" | "builtin">("all");
  const [edgeFilter, setEdgeFilter] = useState<"all" | "resolved" | "external" | "builtin">("all");

  const summary = useMemo(() => {
    if (!dependencyGraph) return null;
    const { metrics } = dependencyGraph;
    return metrics;
  }, [dependencyGraph]);

  if (!codeModel) {
    return (
      <PlaceholderPage
        milestone="M2"
        description="Open and scan a project to see its dependency and architecture graph here."
      />
    );
  }
  if (!dependencyGraph || !summary) {
    return (
      <div className="card" data-testid="graph-missing">
        <p className="muted">Dependency analysis has not completed yet.</p>
      </div>
    );
  }

  const labelOf = (id: string) => {
    const node = dependencyGraph?.nodes.find((n) => n.id === id);
    if (node) return node.label;
    const parts = id.split(":");
    return parts[parts.length - 1] ?? id;
  };

  const visible = useMemo(() => {
    if (!dependencyGraph) return null;
    let nodes = dependencyGraph.nodes;
    if (nodeKindFilter === "external") {
      nodes = nodes.filter((n) => n.kind === "external_package");
    } else if (nodeKindFilter === "builtin") {
      nodes = nodes.filter((n) => n.kind === "builtin_module");
    } else if (nodeKindFilter === "file") {
      nodes = nodes.filter((n) => n.kind === "file_module");
    }
    const capped = nodes.length > 60;
    const shownNodes = capped ? nodes.slice(0, 60) : nodes;
    const nodeIds = new Set(shownNodes.map((n) => n.id));
    const shownEdges = dependencyGraph.edges.filter(
      (edge) =>
        nodeIds.has(edge.source) &&
        nodeIds.has(edge.target) &&
        (edgeFilter === "all" || edge.resolution === edgeFilter),
    );
    return { shownNodes, shownEdges, capped };
  }, [dependencyGraph, nodeKindFilter, edgeFilter]);

  const selectedIncoming = useMemo(() => {
    if (!dependencyGraph || !selected) return [];
    return dependencyGraph.edges.filter((e) => e.target === selected.id);
  }, [dependencyGraph, selected]);

  const selectedOutgoing = useMemo(() => {
    if (!dependencyGraph || !selected) return [];
    return dependencyGraph.edges.filter((e) => e.source === selected.id);
  }, [dependencyGraph, selected]);
  const { cycles, diagnostics, edges } = dependencyGraph;

  return (
    <section className="stack">
      <div className="card" data-testid="graph-summary">
        <h2 className="card-title">Dependency & Architecture</h2>
        <dl className="fact-grid">
          <div>
            <dt>Nodes</dt>
            <dd>{summary.node_count}</dd>
          </div>
          <div>
            <dt>Edges</dt>
            <dd>{summary.edge_count}</dd>
          </div>
          <div>
            <dt>Resolved</dt>
            <dd>{summary.resolved_edges}</dd>
          </div>
          <div>
            <dt>External</dt>
            <dd>{summary.external_edges}</dd>
          </div>
          <div>
            <dt>Builtin</dt>
            <dd>{summary.builtin_edges}</dd>
          </div>
          <div>
            <dt>Unresolved</dt>
            <dd>{summary.unresolved_edges}</dd>
          </div>
          <div>
            <dt>Calls</dt>
            <dd>{dependencyGraph.calls.length}</dd>
          </div>
          <div>
            <dt>Cycles</dt>
            <dd>{summary.cycle_count}</dd>
          </div>
          <div>
            <dt>Components</dt>
            <dd>{summary.connected_components}</dd>
          </div>
        </dl>
        <p className="muted small">Metrics are objective facts, not quality judgments.</p>
      </div>

      <div className="card" data-testid="graph-svg">
        <h2 className="card-title">File dependency graph</h2>
        <div className="filters">
          <label>
            Node kind:{" "}
            <select
              value={nodeKindFilter}
              onChange={(event) => setNodeKindFilter(event.target.value as typeof nodeKindFilter)}
              aria-label="Filter by node kind"
            >
              <option value="all">all</option>
              <option value="file">files</option>
              <option value="external">external packages</option>
              <option value="builtin">builtin modules</option>
            </select>
          </label>
          <label>
            Edge resolution:{" "}
            <select
              value={edgeFilter}
              onChange={(event) => setEdgeFilter(event.target.value as typeof edgeFilter)}
              aria-label="Filter by edge resolution"
            >
              <option value="all">all</option>
              <option value="resolved">resolved</option>
              <option value="external">external</option>
              <option value="builtin">builtin</option>
            </select>
          </label>
        </div>
        {visible && (
          <GraphSvg nodes={visible.shownNodes} edges={visible.shownEdges} onSelect={setSelected} />
        )}
        {visible?.capped && (
          <p className="muted small">
            Showing first 60 file nodes — use the CLI JSON output for the complete graph.
          </p>
        )}
      </div>

      {selected && (
        <div className="card" data-testid="node-detail">
          <h2 className="card-title">Node: {selected.label}</h2>
          <dl className="fact-grid">
            <div>
              <dt>Kind</dt>
              <dd>{selected.kind}</dd>
            </div>
            <div>
              <dt>Fan-in</dt>
              <dd>{edges.filter((e) => e.target === selected.id).length}</dd>
            </div>
            <div>
              <dt>Fan-out</dt>
              <dd>{edges.filter((e) => e.source === selected.id).length}</dd>
            </div>
          </dl>
          <p className="muted small mono">{selected.entity}</p>
          {selectedIncoming.length > 0 && (
            <>
              <p className="muted small">Incoming (from)</p>
              <ul>
                {selectedIncoming.map((edge, index) => (
                  <li key={index} className="mono small">
                    {labelOf(edge.source)} ({edge.resolution})
                  </li>
                ))}
              </ul>
            </>
          )}
          {selectedOutgoing.length > 0 && (
            <>
              <p className="muted small">Outgoing (to)</p>
              <ul>
                {selectedOutgoing.map((edge, index) => (
                  <li key={index} className="mono small">
                    {labelOf(edge.target)} ({edge.resolution})
                  </li>
                ))}
              </ul>
            </>
          )}
        </div>
      )}

      {cycles.length > 0 && (
        <div className="card" data-testid="cycles">
          <h2 className="card-title">Cycles ({cycles.length})</h2>
          <ul className="evidence-list">
            {cycles.map((cycle) => (
              <li key={cycle.id} className="mono small">
                {cycle.nodes.map((id) => nodeLabel(dependencyGraph, id)).join(" → ")}
              </li>
            ))}
          </ul>
          <p className="muted small">Recorded as structural facts — health judgment belongs to a later milestone.</p>
        </div>
      )}

      {diagnostics.length > 0 && (
        <div className="card" data-testid="unresolved">
          <h2 className="card-title">Unresolved dependencies ({diagnostics.length})</h2>
          <ul>
            {diagnostics.slice(0, 20).map((diagnostic, index) => (
              <li key={index} className="mono small">
                {diagnostic.source_path}: {diagnostic.raw_target} — {diagnostic.reason}
              </li>
            ))}
          </ul>
        </div>
      )}
    </section>
  );
}

function GraphSvg({
  nodes,
  edges,
  onSelect,
}: {
  nodes: GraphNodeDto[];
  edges: { id: string; source: string; target: string }[];
  onSelect: (node: GraphNodeDto) => void;
}) {
  // 环形布局（确定性：按 label 排序后均分角度）。
  const positions = useMemo(() => {
    const sorted = [...nodes].sort((a, b) => a.label.localeCompare(b.label));
    const cx = 360;
    const cy = 240;
    const radius = 180;
    const map = new Map<string, { x: number; y: number }>();
    sorted.forEach((node, index) => {
      const angle = (2 * Math.PI * index) / sorted.length - Math.PI / 2;
      map.set(node.id, {
        x: cx + radius * Math.cos(angle),
        y: cy + radius * Math.sin(angle),
      });
    });
    return map;
  }, [nodes]);

  if (nodes.length === 0) {
    return <p className="muted">No file modules to visualize.</p>;
  }

  return (
    <div>
      <svg
        viewBox="0 0 720 480"
        width="100%"
        role="img"
        aria-label="File dependency graph"
        className="graph-svg"
      >
        {edges.map((edge: { id: string; source: string; target: string }) => {
          const a = positions.get(edge.source);
          const b = positions.get(edge.target);
          if (!a || !b) return null;
          return (
            <line
              key={edge.id}
              x1={a.x}
              y1={a.y}
              x2={b.x}
              y2={b.y}
              className="graph-edge"
            />
          );
        })}
        {nodes.map((node) => {
          const pos = positions.get(node.id);
          if (!pos) return null;
          return (
            <g
              key={node.id}
              transform={`translate(${pos.x}, ${pos.y})`}
              className="graph-node"
              onClick={() => onSelect(node)}
            >
              <circle r={6} />
              <text x={8} y={3}>
                {node.label}
              </text>
            </g>
          );
        })}
      </svg>
    </div>
  );
}
