//! HealthRule trait 与注册表（规格 M7 §7/§16/§17/§18/§73/§180/§181）。
//!
//! - 规则 = Rust trait 实现 + 注册表，不搞宏/插件/脚本运行时（§180）
//! - 规则 id 稳定且不绑实现文件（§17）
//! - 规则只读上下文、返回 [`RuleResult`]（§18/§201/§202），无全局状态（§204）
//! - 全部 built-in 随版本发布，无远程更新（§181/§182）

use crate::model::{FindingCategory, RuleResult};
use prism_core::severity::Severity;
use prism_core::task::CancellationToken;

/// 规则元数据（§73：GUI/CLI 可检视）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthRuleMeta {
    /// 稳定规则 id（"MAINT001" 等；§16/§17）。
    pub id: &'static str,
    pub name: &'static str,
    pub category: FindingCategory,
    pub description: &'static str,
    pub default_severity: Severity,
    pub default_enabled: bool,
    /// 规则声明的 threshold 语义；None = 该规则不接受 threshold
    /// （收到配置 threshold ⇒ 引擎期 unknown-option 诊断，§72）。
    pub threshold_meaning: Option<&'static str>,
    /// 规则是否需要派生事实（文件内容/代码度量/fan 统计）；
    /// 引擎据此决定是否构建一次供全部规则共享（§91-§94）。
    pub needs_derived: bool,
}

/// 一条健康规则（§18：Inputs → Evaluation → Findings → Evidence）。
///
/// 实现必须：纯确定性、只依赖传入上下文、不改共享状态（§202）、
/// 支持通过 cancel 协作取消（§101/§102，不自带线程终止）。
pub trait HealthRule: Send + Sync {
    fn meta(&self) -> &'static HealthRuleMeta;

    /// 只读评估。错误 = 该规则失败（非致命：诊断 + 其余规则继续，§99/§163）。
    fn evaluate(
        &self,
        context: &crate::engine::HealthContext,
        config: &prism_core::config::HealthRuleConfig,
        cancel: &CancellationToken,
    ) -> prism_core::PrismResult<RuleResult>;
}

/// 内建规则注册表（确定性：按 id 排序遍历，§95/§97）。
#[derive(Default)]
pub struct HealthRuleRegistry {
    rules: Vec<Box<dyn HealthRule>>,
}

impl HealthRuleRegistry {
    /// 内建规则集（M7a：MAINT001 作为引擎验证载体；完整目录在 M7b）。
    pub fn with_builtins() -> Self {
        let mut registry = HealthRuleRegistry::default();
        registry.register(Box::new(crate::rules::maintenance::MaintenanceMarkerRule));
        registry.register(Box::new(crate::rules::architecture::CircularDependencyRule));
        registry.register(Box::new(crate::rules::architecture::FanOutRule));
        registry.register(Box::new(crate::rules::architecture::FanInRule));
        registry.register(Box::new(
            crate::rules::forbidden_dependency::ForbiddenDependencyRule,
        ));
        registry.register(Box::new(crate::rules::dependency::UnresolvedDependencyRule));
        registry.register(Box::new(crate::rules::dependency::AmbiguousDependencyRule));
        registry.register(Box::new(crate::rules::code::HighComplexityRule));
        registry.register(Box::new(crate::rules::code::LargeFunctionRule));
        registry.register(Box::new(crate::rules::code::LargeTypeRule));
        registry.register(Box::new(crate::rules::code::ExcessiveNestingRule));
        registry.register(Box::new(crate::rules::security::HardcodedSecretRule));
        registry.register(Box::new(crate::rules::security::PrivateKeyRule));
        registry.register(Box::new(crate::rules::documentation::CommandDriftRule));
        registry.register(Box::new(crate::rules::documentation::PathDriftRule));
        registry
    }

    /// 注册规则（测试可注入；重复 id 以先注册者为准，保持确定性）。
    pub fn register(&mut self, rule: Box<dyn HealthRule>) {
        if self.get(rule.meta().id).is_none() {
            self.rules.push(rule);
            self.rules.sort_by_key(|rule| rule.meta().id);
        }
    }

    /// 按 id 排序的全部规则（§95 确定性遍历）。
    pub fn rules(&self) -> impl Iterator<Item = &dyn HealthRule> {
        self.rules.iter().map(|rule| rule.as_ref())
    }

    pub fn get(&self, rule_id: &str) -> Option<&dyn HealthRule> {
        self.rules
            .iter()
            .map(|rule| rule.as_ref())
            .find(|rule| rule.meta().id == rule_id)
    }

    pub fn len(&self) -> usize {
        self.rules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

impl std::fmt::Debug for HealthRuleRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HealthRuleRegistry")
            .field(
                "rule_ids",
                &self
                    .rules
                    .iter()
                    .map(|rule| rule.meta().id)
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}
