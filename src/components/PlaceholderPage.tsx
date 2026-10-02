interface PlaceholderPageProps {
  /** 主里程碑标记，如 "M1"。 */
  milestone: string;
  /** 该领域交付什么（来自 roadmap 的简述）。 */
  description: string;
}

/**
 * 未实现领域的诚实占位页。
 * 禁止伪造数据（纲领 §38）：只说明“该能力在哪个里程碑到来”。
 */
export default function PlaceholderPage({ milestone, description }: PlaceholderPageProps) {
  return (
    <section className="placeholder" data-testid="placeholder-page">
      <p className="placeholder-milestone">Coming in {milestone}</p>
      <p className="placeholder-description">{description}</p>
      <p className="placeholder-note">
        Prism does not show fake data. This area will become real once the milestone
        lands.
      </p>
    </section>
  );
}
