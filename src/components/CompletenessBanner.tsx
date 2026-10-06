import { useState } from "react";

/**
 * 数据完整度横幅（P2-5，§46 精神：Unknown 不得画成正常）。
 * 读各域 report 的 completeness/diagnostics，把"这次看到的数据缺了什么"
 * 摆到页面顶部，而不是埋在底部 limitations 卡里。
 * 完整（complete 且无诊断）时不渲染——不制造告警噪音。
 */

export interface CompletenessDiagnostic {
  kind: string;
  subject?: string | null;
  detail: string;
}

export interface CompletenessData {
  /** 域名，如「健康分析」，用于横幅标题。 */
  domain: string;
  /** 该域 report 的完整度字段（complete/partial/truncated/failed/unknown/…）。 */
  completeness: string;
  /** 顶层诊断明细；验证域由"未产出结论的检查"聚合而来。 */
  diagnostics: CompletenessDiagnostic[];
}

const COMPLETENESS_LABEL: Record<string, string> = {
  complete: "完整",
  partial: "部分缺失",
  truncated: "已截断",
  failed: "失败",
  unknown: "未知",
};

/** 该域数据是否称得上完整：状态 complete 且没有任何诊断明细。 */
export function isComplete(data: CompletenessData): boolean {
  return data.completeness === "complete" && data.diagnostics.length === 0;
}

/** 折叠时展示的明细条数。 */
const PREVIEW_COUNT = 3;

export default function CompletenessBanner({
  data,
  testId = "completeness-banner",
}: {
  data: CompletenessData;
  testId?: string;
}) {
  const [expanded, setExpanded] = useState(false);

  if (isComplete(data)) return null;

  const label = COMPLETENESS_LABEL[data.completeness] ?? data.completeness;
  const visible = expanded ? data.diagnostics : data.diagnostics.slice(0, PREVIEW_COUNT);
  const hidden = data.diagnostics.length - visible.length;

  return (
    <div className="warning-box completeness-banner" data-testid={testId}>
      <p className="completeness-banner-title">
        数据完整度（Data Completeness）· {data.domain}：
        <strong>{label}</strong>
        {data.diagnostics.length > 0 && ` · ${data.diagnostics.length} 条明细`}
      </p>
      <ul>
        {visible.map((diagnostic, index) => (
          <li key={index} className="small">
            <span className="mono">{diagnostic.kind}</span>
            {diagnostic.subject ? (
              <span className="muted"> [{diagnostic.subject}]</span>
            ) : null}{" "}
            — {diagnostic.detail}
          </li>
        ))}
      </ul>
      {hidden > 0 && (
        <button
          type="button"
          className="linklike small"
          onClick={() => setExpanded(true)}
        >
          展开其余 {hidden} 条
        </button>
      )}
      {expanded && data.diagnostics.length > PREVIEW_COUNT && (
        <button
          type="button"
          className="linklike small"
          onClick={() => setExpanded(false)}
        >
          收起
        </button>
      )}
    </div>
  );
}
