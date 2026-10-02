import { useState } from "react";
import type { ProjectTypeCandidate } from "../types/prism";

/** 项目类型 + 证据展示（纲领 §46：点击查看 Evidence）。 */
export default function ProjectTypesCard({ candidates }: { candidates: ProjectTypeCandidate[] }) {
  const [expanded, setExpanded] = useState<string | null>(null);

  if (candidates.length === 0) {
    return (
      <div className="card">
        <h2 className="card-title">Detected project types</h2>
        <p className="muted">No known project type signatures found.</p>
      </div>
    );
  }

  return (
    <div className="card">
      <h2 className="card-title">Detected project types</h2>
      <ul className="row-list">
        {candidates.map((candidate) => (
          <li key={candidate.name}>
            <button
              type="button"
              className="row-button"
              aria-expanded={expanded === candidate.name}
              onClick={() => setExpanded(expanded === candidate.name ? null : candidate.name)}
            >
              <span>{candidate.name}</span>
              <span className="pill">{candidate.confidence}</span>
            </button>
            {expanded === candidate.name && (
              <ul className="evidence-list" data-testid={`evidence-${candidate.name}`}>
                {candidate.evidence.map((item, index) => (
                  <li key={index}>
                    <code>{item.path}</code> — {item.reason}
                  </li>
                ))}
              </ul>
            )}
          </li>
        ))}
      </ul>
      <p className="muted small">Candidates are evidence-based, not a verdict.</p>
    </div>
  );
}
