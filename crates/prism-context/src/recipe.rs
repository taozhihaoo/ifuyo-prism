//! ContextRecipe（规格 M10b §117–§125/§228–§234/§290–§291）。
//!
//! 按 TaskKind 调整 group 顺序（§116 group order 是 policy）；第一版
//! auto/review/debug 三 profile（§291 避免 profile 爆炸）。

use crate::model::{ContextFocus, ContextGroup, TaskKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextRecipe {
    /// §83/§234：按 task 自动（Unknown ⇒ Generic overview）。
    Auto,
    /// §124/§290：review profile。
    Review,
    /// §118：bug investigation。
    Debug,
}

impl ContextRecipe {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "auto" => Some(ContextRecipe::Auto),
            "review" => Some(ContextRecipe::Review),
            "debug" => Some(ContextRecipe::Debug),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            ContextRecipe::Auto => "auto",
            ContextRecipe::Review => "review",
            ContextRecipe::Debug => "debug",
        }
    }

    /// §117–§125：按 task kind 推导 recipe（显式 profile 覆盖）。
    pub fn for_task(kind: TaskKind) -> Self {
        match kind {
            TaskKind::Review => ContextRecipe::Review,
            TaskKind::BugInvestigation | TaskKind::Test => ContextRecipe::Debug,
            _ => ContextRecipe::Auto,
        }
    }

    /// 组顺序（§114/§115/§116：policy 而非硬编码；按 recipe 调整）。
    pub fn group_order(&self, focus: &[ContextFocus]) -> Vec<ContextGroup> {
        let mut order = match self {
            ContextRecipe::Auto => {
                if focus.contains(&ContextFocus::Ci) {
                    // §121 CI recipe：CI/验证优先（§228）。
                    vec![
                        ContextGroup::Verification,
                        ContextGroup::Environment,
                        ContextGroup::Project,
                        ContextGroup::CurrentChange,
                        ContextGroup::TargetCode,
                        ContextGroup::Dependencies,
                        ContextGroup::Impact,
                        ContextGroup::Tests,
                        ContextGroup::Health,
                        ContextGroup::Documentation,
                    ]
                } else if focus.contains(&ContextFocus::Documentation) {
                    // §123 Documentation recipe：文档 + 结构优先。
                    vec![
                        ContextGroup::Documentation,
                        ContextGroup::Project,
                        ContextGroup::TargetCode,
                        ContextGroup::CurrentChange,
                        ContextGroup::Dependencies,
                        ContextGroup::Impact,
                        ContextGroup::Tests,
                        ContextGroup::Verification,
                        ContextGroup::Environment,
                        ContextGroup::Health,
                    ]
                } else {
                    // §115 默认顺序。
                    vec![
                        ContextGroup::Project,
                        ContextGroup::CurrentChange,
                        ContextGroup::TargetCode,
                        ContextGroup::Dependencies,
                        ContextGroup::Impact,
                        ContextGroup::Tests,
                        ContextGroup::Health,
                        ContextGroup::Verification,
                        ContextGroup::Environment,
                        ContextGroup::Documentation,
                    ]
                }
            }
            ContextRecipe::Review => {
                // §124 Review recipe：ChangeSet 优先。
                vec![
                    ContextGroup::CurrentChange,
                    ContextGroup::Impact,
                    ContextGroup::Health,
                    ContextGroup::Verification,
                    ContextGroup::Dependencies,
                    ContextGroup::Tests,
                    ContextGroup::TargetCode,
                    ContextGroup::Project,
                    ContextGroup::Environment,
                    ContextGroup::Documentation,
                ]
            }
            ContextRecipe::Debug => {
                // §118 Bug Investigation recipe：失败验证 + 错误诊断优先。
                vec![
                    ContextGroup::Verification,
                    ContextGroup::TargetCode,
                    ContextGroup::CurrentChange,
                    ContextGroup::Impact,
                    ContextGroup::Tests,
                    ContextGroup::Environment,
                    ContextGroup::Health,
                    ContextGroup::Dependencies,
                    ContextGroup::Project,
                    ContextGroup::Documentation,
                ]
            }
        };
        order.dedup();
        order
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // §119/§124：recipe 按 task kind 调整组序。
    #[test]
    fn review_recipe_puts_changes_first() {
        let order = ContextRecipe::Review.group_order(&[]);
        assert_eq!(order[0], ContextGroup::CurrentChange);
    }

    #[test]
    fn debug_recipe_puts_verification_first() {
        let order = ContextRecipe::Debug.group_order(&[]);
        assert_eq!(order[0], ContextGroup::Verification);
    }

    #[test]
    fn auto_recipe_ci_focus_boosts_verification() {
        let order = ContextRecipe::Auto.group_order(&[ContextFocus::Ci]);
        assert_eq!(order[0], ContextGroup::Verification);
    }

    // §291：profile 集合有界。
    #[test]
    fn bounded_profiles() {
        assert!(ContextRecipe::parse("auto").is_some());
        assert!(ContextRecipe::parse("review").is_some());
        assert!(ContextRecipe::parse("debug").is_some());
        assert!(ContextRecipe::parse("everything").is_none());
    }
}
