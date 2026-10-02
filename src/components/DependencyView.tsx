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
        <GraphSvg graph={dependencyGraph} onSelect={setSelected} />
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

const MAX_GRAPH_NODES = 60;

function GraphSvg({
  graph,
  onSelect,
}: {
  graph: DependencyGraphResultDto;
  onSelect: (node: GraphNodeDto) => void;
}) {
  const { positions, width, height, capped, shown } = useMemo(() => {
    const fileNodes = graph.nodes.filter((n) => n.kind === "file_module");
    const capped = fileNodes.length > MAX_GRAPH_NODES;
    const shown = capped ? fileNodes.slice(0, MAX_GRAPH_NODES) : fileNodes;
    // 环形布局（确定性：按 label 排序后均分角度）。
    const sorted = [...shown].sort((a, b) => a.label.localeCompare(b.label));
    const w = 720;
    const h = 480;
    const cx = w / 2;
    const cy = h / 2;
    const radius = Math.min(w, h) / 2 - 60;
    const positions = new Map<string, { x: number; y: number }>();
    sorted.forEach((node, index) => {
      const angle = (2 * Math.PI * index) / sorted.length - Math.PI / 2;
      positions.set(node.id, {
        x: cx + radius * Math.cos(angle),
        y: cy + radius * Math.sin(angle),
      });
    });
    return { positions, width: w, height: h, capped, shown };
  }, [graph]);

  const shownEdges = graph.edges.filter(
    (edge) =>
      edge.kind === "imports" &&
      edge.resolution === "resolved" &&
      positions.has(edge.source) &&
      positions.has(edge.target),
  );

  if (fileNodeCount(graph) === 0) {
    return <p className="muted">No file modules to visualize.</p>;
  }

  return (
    <div>
      <svg
        viewBox={`0 0 ${width} ${height}`}
        width="100%"
        role="img"
        aria-label="File dependency graph"
        className="graph-svg"
      >
        {shownEdges.map((edge) => {
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
        {shown.map((node) => {
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
      {capped && (
        <p className="muted small">
          Showing first {MAX_GRAPH_NODES} file nodes — use the CLI JSON output for the
          complete graph.
        </p>
      )}
    </div>
  );
}

function fileNodeCount(graph: DependencyGraphResultDto): number {
  return graph.nodes.filter((n) => n.kind === "file_module").length;
}
