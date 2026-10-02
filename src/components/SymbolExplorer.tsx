import { useMemo, useState } from "react";
import type { ParseResultDto, SymbolDto } from "../types/prism";

const MAX_RENDERED_SYMBOLS = 500;

interface FlatSymbol {
  symbol: SymbolDto;
  depth: number;
}

/** 按文件分组的确定性深度优先平铺（渲染截断如实说明）。 */
function flatten(results: ParseResultDto[]): { groups: { result: ParseResultDto; items: FlatSymbol[] }[]; capped: boolean } {
  let total = 0;
  let capped = false;
  const groups = results.map((result) => {
    const byParent = new Map<string, SymbolDto[]>();
    let fileRootId: string | null = null;
    for (const symbol of result.symbols) {
      if (symbol.kind === "file") {
        fileRootId = symbol.id;
        continue;
      }
      const key = symbol.parent_id ?? "__root__";
      byParent.set(key, [...(byParent.get(key) ?? []), symbol]);
    }
    const items: FlatSymbol[] = [];
    const walk = (parentId: string, depth: number) => {
      const children = byParent.get(parentId) ?? [];
      for (const symbol of children) {
        if (total >= MAX_RENDERED_SYMBOLS) {
          capped = true;
          return;
        }
        total += 1;
        items.push({ symbol, depth });
        walk(symbol.id, depth + 1);
      }
    };
    walk(fileRootId ?? "__root__", 0);
    // 无 File 根（如 failed/占位）时兜底遍历剩余符号。
    if (!fileRootId) walk("__root__", 0);
    return { result, items };
  });
  return { groups, capped };
}

interface SymbolExplorerProps {
  results: ParseResultDto[];
  selectedId: string | null;
  onSelect: (symbol: SymbolDto) => void;
}

/** 每文件的符号树（按父子关系缩进），只渲染真实解析结果。 */
export default function SymbolExplorer({ results, selectedId, onSelect }: SymbolExplorerProps) {
  const { groups, capped } = useMemo(() => flatten(results), [results]);
  const [selected, setSelected] = useState<string | null>(selectedId);
  const activeId = selected ?? selectedId;

  return (
    <div data-testid="symbol-explorer">
      {groups.map(({ result, items }) => (
        <div key={result.file_id} className="stack">
          <p className="mono tree-dir">{result.relative_path}</p>
          {result.status === "failed" ? (
            <p className="muted small">parse failed — see diagnostics</p>
          ) : items.length === 0 ? (
            <p className="muted small">no symbols extracted</p>
          ) : (
            <ul className="symbol-list" role="tree">
              {items.map(({ symbol, depth }) => (
                <li key={symbol.id} role="treeitem" style={{ paddingLeft: depth * 16 }}>
                  <button
                    type="button"
                    className={`symbol-row${symbol.id === activeId ? " symbol-selected" : ""}`}
                    onClick={() => {
                      setSelected(symbol.id);
                      onSelect(symbol);
                    }}
                  >
                    <span className="mono">
                      {symbol.name} <span className="muted">({symbol.kind})</span>
                    </span>
                    <span className="muted mono small">L{symbol.range.start.line}</span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      ))}
      {capped && <p className="muted small">Rendering capped at {MAX_RENDERED_SYMBOLS} symbols.</p>}
    </div>
  );
}

export function SymbolDetail({ symbol }: { symbol: SymbolDto }) {
  return (
    <div className="card" data-testid="symbol-detail">
      <h2 className="card-title">Symbol</h2>
      <dl className="fact-grid">
        <div>
          <dt>Name</dt>
          <dd>{symbol.name}</dd>
        </div>
        <div>
          <dt>Kind</dt>
          <dd>{symbol.kind}</dd>
        </div>
        <div>
          <dt>Language</dt>
          <dd>{symbol.language}</dd>
        </div>
        <div>
          <dt>Visibility</dt>
          <dd>{symbol.visibility}</dd>
        </div>
        <div>
          <dt>Qualified name</dt>
          <dd>{symbol.qualified_name}</dd>
        </div>
        <div>
          <dt>Location</dt>
          <dd>
            L{symbol.range.start.line}:C{symbol.range.start.column} – L{symbol.range.end.line}
          </dd>
        </div>
        {symbol.signature && (
          <div>
            <dt>Signature</dt>
            <dd>{symbol.signature}</dd>
          </div>
        )}
        <div>
          <dt>Documented</dt>
          <dd>{symbol.documentation_present ? "yes" : "no"}</dd>
        </div>
      </dl>
    </div>
  );
}
