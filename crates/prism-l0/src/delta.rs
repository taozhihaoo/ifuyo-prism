//! L0 增量正确性（2.0-M22 §52/§53/§90-§91）。
//!
//! 当前 L0 每次运行为**确定性全量再观察**（M1 扫描亚秒级；§51 警告不要
//! 造四五套互不相知的缓存——当前零持久缓存，增量缓存待有测量证据再建）。
//! 本模块提供两次 bundle 之间的**正确增量差分**：验证"修改/新增/删除只
//! 影响应影响的实体与事实"，并在观察层把旧结果标记 stale 的依据。

use prism_intel::IntelBundle;
use serde::{Deserialize, Serialize};

/// 变更实体引用（id 稳定，display_name 供人读）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityChange {
    pub id: String,
    pub display_name: String,
}

/// 两次 L0 观察的差分（内容寻址 id 直比——id 变了就是变了）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct L0Delta {
    pub added_entities: Vec<EntityChange>,
    pub removed_entities: Vec<EntityChange>,
    pub added_facts: Vec<String>,
    pub removed_facts: Vec<String>,
    /// 事实总量（供 stale 判定与测试）。
    pub facts_before: usize,
    pub facts_after: usize,
}

impl L0Delta {
    /// 差分为空 ⇒ 两次观察等价（旧结果仍 fresh 的依据）。
    pub fn is_empty(&self) -> bool {
        self.added_entities.is_empty()
            && self.removed_entities.is_empty()
            && self.added_facts.is_empty()
            && self.removed_facts.is_empty()
    }

    /// 事实是否被内容性修改（同 type+subject 不同 value ⇒ id 不同）。
    pub fn has_fact_changes(&self) -> bool {
        !self.added_facts.is_empty() || !self.removed_facts.is_empty()
    }
}

/// 比较 previous → current。
pub fn diff(previous: &IntelBundle, current: &IntelBundle) -> L0Delta {
    debug_assert_eq!(
        previous.project.stable_key, current.project.stable_key,
        "cannot diff bundles of different projects"
    );
    let mut delta = L0Delta {
        facts_before: previous.facts.len(),
        facts_after: current.facts.len(),
        ..L0Delta::default()
    };
    for (id, entity) in &current.entities {
        if !previous.entities.contains_key(id) {
            delta.added_entities.push(EntityChange {
                id: id.as_str().to_string(),
                display_name: entity.display_name.clone(),
            });
        }
    }
    for (id, entity) in &previous.entities {
        if !current.entities.contains_key(id) {
            delta.removed_entities.push(EntityChange {
                id: id.as_str().to_string(),
                display_name: entity.display_name.clone(),
            });
        }
    }
    for id in current.facts.keys() {
        if !previous.facts.contains_key(id) {
            delta.added_facts.push(id.clone());
        }
    }
    for id in previous.facts.keys() {
        if !current.facts.contains_key(id) {
            delta.removed_facts.push(id.clone());
        }
    }
    delta
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::L0Options;
    use crate::structural::structural_analysis;
    use prism_core::task::CancellationToken;
    use prism_intel::{AnalysisContext, AnalysisScope, ProjectIdentity};

    fn scan(root: &std::path::Path) -> prism_intel::IntelBundle {
        let project = prism_project::Project::from_root(root).unwrap();
        let identity = ProjectIdentity::from_project(&project);
        let context = AnalysisContext::new(
            identity,
            AnalysisScope::FullProject,
            vec!["l0_structural".into()],
            CancellationToken::new(),
        );
        structural_analysis(root, &context, &L0Options::default())
            .data
            .expect("complete l0 scan")
    }

    fn fixture() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "prism-l0-delta-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/a.txt"), "alpha").unwrap();
        std::fs::write(dir.join("src/b.txt"), "beta").unwrap();
        dir
    }

    #[test]
    fn unchanged_project_yields_empty_delta() {
        let dir = fixture();
        let first = scan(&dir);
        let second = scan(&dir);
        let delta = diff(&first, &second);
        assert!(
            delta.is_empty(),
            "same project state must not produce spurious delta: {delta:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn modify_add_delete_reflect_precisely() {
        let dir = fixture();
        let first = scan(&dir);
        // modify a.txt（内容变 ⇒ file 实体的事实变化——这里 size 变不进实体，
        // 通过 git 缺席时的内容无关性验证：加新文件 + 删旧文件 + 改内容）。
        std::fs::write(dir.join("src/a.txt"), "alpha-changed").unwrap();
        std::fs::write(dir.join("src/c.txt"), "gamma").unwrap();
        std::fs::remove_file(dir.join("src/b.txt")).unwrap();
        let second = scan(&dir);
        let delta = diff(&first, &second);
        assert!(
            delta.has_fact_changes() || !delta.added_entities.is_empty(),
            "changes must be visible: {delta:?}"
        );
        let added_names: Vec<&str> = delta
            .added_entities
            .iter()
            .map(|c| c.display_name.as_str())
            .collect();
        let removed_names: Vec<&str> = delta
            .removed_entities
            .iter()
            .map(|c| c.display_name.as_str())
            .collect();
        assert!(
            added_names.contains(&"c.txt"),
            "new file entity added: {added_names:?}"
        );
        assert!(
            removed_names.contains(&"b.txt"),
            "deleted file entity removed: {removed_names:?}"
        );
        assert!(
            !added_names.contains(&"a.txt"),
            "modified-in-place file keeps its path id"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
