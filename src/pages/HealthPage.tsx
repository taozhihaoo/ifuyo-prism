import { useState } from "react";
import { useProjectStore } from "../app/ProjectStore";
import ClassicHealthPanel from "./ClassicHealthPanel";
import Health2Panel from "../components/Health2Panel";
import EvolutionPanel from "../components/EvolutionPanel";

type Rail = "classic" | "health2";

/**
 * 健康页（P1-1/P2-4）：双轨规则集。
 *
 * - 1.0 内置规则集（M7 经典管线，本会话后台分析）
 * - 2.0 health2（`prism_health2::analyze_project`——与 CLI `prism health2`
 *   同一函数，GUI 与 CLI 输出一致由构造保证，§71）
 *
 * 两条轨道是相互独立的管线，结论可能不同；每张卡片标注其规则来源
 * （P2-4a/P2-4b）。演化（M211 热点/耦合）作为独立按需卡片挂在页尾。
 */
export default function HealthPage() {
  const { project } = useProjectStore();
  const [rail, setRail] = useState<Rail>("classic");

  if (!project) {
    return (
      <div className="card" data-testid="health-needs-project">
        <p className="muted">Open a project to see its health findings here.</p>
      </div>
    );
  }

  return (
    <section className="stack">
      <div className="card" data-testid="health-rails">
        <div className="chip-row" role="tablist" aria-label="规则集轨道">
          <button
            type="button"
            role="tab"
            aria-selected={rail === "classic"}
            className={`chip${rail === "classic" ? " selected" : ""}`}
            onClick={() => setRail("classic")}
            data-testid="health-rail-classic"
          >
            1.0 内置规则集
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={rail === "health2"}
            className={`chip${rail === "health2" ? " selected" : ""}`}
            onClick={() => setRail("health2")}
            data-testid="health-rail-health2"
          >
            2.0 health2
          </button>
        </div>
        <p className="muted small" data-testid="health-dual-track-note">
          双轨说明：两条轨道是相互独立的规则管线，结论可能不同；各卡片标注规则来源与条数（P2-4）。
        </p>
      </div>
      {rail === "classic" ? <ClassicHealthPanel /> : <Health2Panel />}
      <EvolutionPanel />
    </section>
  );
}
