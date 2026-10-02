import { useMemo, useState } from "react";
import type { FileInfoDto } from "../types/prism";

const MAX_RENDERED_FILES = 500;

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

type SortKey = "relative_path" | "size_bytes" | "language";

/**
 * 文件列表：Path / Kind / Language / Size，带文本过滤与排序
 * （纲领 §44：轻量即可）。大列表截断展示并如实说明。
 */
export default function FileList({ files }: { files: FileInfoDto[] }) {
  const [filter, setFilter] = useState("");
  const [sortKey, setSortKey] = useState<SortKey>("relative_path");
  const [ascending, setAscending] = useState(true);

  const visible = useMemo(() => {
    const needle = filter.trim().toLowerCase();
    const filtered = needle
      ? files.filter((file) => file.relative_path.toLowerCase().includes(needle))
      : files;
    const sorted = [...filtered].sort((a, b) => {
      const left = a[sortKey] ?? "";
      const right = b[sortKey] ?? "";
      const result = String(left).localeCompare(String(right), undefined, { numeric: true });
      return ascending ? result : -result;
    });
    return sorted;
  }, [files, filter, sortKey, ascending]);

  const toggleSort = (key: SortKey) => {
    if (sortKey === key) {
      setAscending(!ascending);
    } else {
      setSortKey(key);
      setAscending(true);
    }
  };

  return (
    <div className="card">
      <h2 className="card-title">
        Files <span className="muted">({files.length})</span>
      </h2>
      <input
        type="search"
        className="text-input"
        placeholder="Filter by path…"
        value={filter}
        onChange={(event) => setFilter(event.target.value)}
        aria-label="Filter files by path"
      />
      <table className="file-table">
        <thead>
          <tr>
            <th>
              <button type="button" className="sort-button" onClick={() => toggleSort("relative_path")}>
                Path {sortKey === "relative_path" && (ascending ? "↑" : "↓")}
              </button>
            </th>
            <th>Kind</th>
            <th>
              <button type="button" className="sort-button" onClick={() => toggleSort("language")}>
                Language {sortKey === "language" && (ascending ? "↑" : "↓")}
              </button>
            </th>
            <th>
              <button type="button" className="sort-button" onClick={() => toggleSort("size_bytes")}>
                Size {sortKey === "size_bytes" && (ascending ? "↑" : "↓")}
              </button>
            </th>
          </tr>
        </thead>
        <tbody>
          {visible.slice(0, MAX_RENDERED_FILES).map((file) => (
            <tr key={file.id}>
              <td className="mono">{file.relative_path}</td>
              <td>{file.classification.kind}</td>
              <td>{file.language ?? "—"}</td>
              <td className="mono">{formatBytes(file.size_bytes)}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {visible.length > MAX_RENDERED_FILES && (
        <p className="muted small">
          Showing first {MAX_RENDERED_FILES} of {visible.length} matching files.
        </p>
      )}
      {visible.length === 0 && <p className="muted">No files match the filter.</p>}
    </div>
  );
}
