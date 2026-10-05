//! # Prism L1 C# Symbol Engine（2.0-M23）
//!
//! 语言无关的 Symbol 契约由 `prism-intel` 承载（Entity kind=symbol +
//! SourceRange Evidence + contains/defines 关系）；本 crate 是第一个
//! 语言实现：tree-sitter-c-sharp（MIT、编译期内嵌、无运行时依赖、
//! 错误恢复 + 精确 range）。
//!
//! 管线（§20 分层）：source → tree-sitter tree → declaration walk →
//! RawSymbol → SymbolEngine（逐文件 content_hash 缓存 + 增量）→
//! SymbolIndex（逻辑聚合：partial/overload/parent）→ IntelBundle。
//!
//! 边界（§三/§76）：只回答"代码声明了什么"——references/calls/类型解析
//! 归 M25；`[Export]` 等特性只记录名称，Godot 语义归 M24；不执行源码、
//! 无 shell/网络；Identity 与位置分离（行号变化不换 id）。

pub mod change_mapping;
pub mod extract;
pub mod identity;
pub mod index;
pub mod intel;
pub mod model;
pub mod semantic;

pub use change_mapping::{map_file_change, ChangedSymbol, FileChangeKind};
pub use extract::extract;
pub use identity::symbol_id;
pub use index::{SymbolEngine, SymbolIndex};
pub use model::{
    AttributeInfo, DeclarationOccurrence, FileExtraction, ParamInfo, ParseDiagnostic, RawSymbol,
    SourceRangeCs, SymbolKindCs,
};
pub use semantic::{
    analyze_file_with_resolver, Dispatch, Resolution, SemanticDiagnostic, SemanticEngine,
    SemanticRelation, SemanticRelationKind, SemanticTarget,
};

/// M23 输出契约版本（§61：与 parser/extraction 版本分离）。
pub const SYMBOL_MODEL_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::SymbolEngine;
    use crate::model::SymbolKindCs;
    use prism_core::ids::ProjectId;
    use prism_intel::identity::ProjectIdentity;

    const PLAYER_CS: &str = r#"using System;

namespace Game.Entities
{
    /// <summary>Player doc.</summary>
    [Serializable]
    public partial class Player
    {
        private int hp;
        public int HP { get; set; }
        public event Action Died;

        public Player() { }

        public void Attack(int damage) { }
        public void Attack(string weapon) { }

        protected virtual void OnDied() { }
    }
}
"#;

    const INVENTORY_CS: &str = r#"namespace Game.Inventory;

public record Item(string Name, int Count);
public readonly record struct Pixel(byte A);
public interface IEquippable { void Equip(); }
public enum Slot { Head, Hands = 2 }
public delegate void SlotChanged(Slot oldSlot);
public struct Weight { public float Value; }
"#;

    fn engine_with(files: &[(&str, &str)]) -> SymbolEngine {
        let mut engine = SymbolEngine::new();
        for (path, source) in files {
            engine.update_file(path, source);
        }
        engine
    }

    #[test]
    fn extracts_types_members_and_hierarchy() {
        let engine = engine_with(&[("Player.cs", PLAYER_CS)]);
        let index = engine.build_index();

        let player = index
            .by_qualified("Game.Entities.Player")
            .first()
            .copied()
            .unwrap_or_else(|| panic!("Player class missing"));
        assert_eq!(player.kind, SymbolKindCs::Class);
        assert!(player.is_partial);
        assert_eq!(player.attributes, vec!["Serializable"]);
        assert_eq!(player.doc_summary.as_deref(), Some("Player doc."));

        // 成员层级（§15）。
        let children = index.children(player.id.as_str());
        let names: Vec<&str> = children.iter().map(|s| s.name.as_str()).collect();
        for expected in ["hp", "HP", "Died", "Player", "Attack", "OnDied"] {
            assert!(names.contains(&expected), "missing {expected} in {names:?}");
        }
        // 每个成员的 parent 回指（§49 hierarchy invariant）。
        for child in &children {
            assert_eq!(child.parent_id.as_deref(), Some(player.id.as_str()));
        }
    }

    #[test]
    fn overloads_distinguished_and_parameters_extracted() {
        let engine = engine_with(&[("Player.cs", PLAYER_CS)]);
        let index = engine.build_index();
        let attacks = index.by_qualified("Game.Entities.Player.Attack");
        assert_eq!(attacks.len(), 2, "overloads must both exist");
        let mut param_sets: Vec<Vec<String>> = attacks
            .iter()
            .map(|s| s.parameters.iter().map(|p| p.type_syntax.clone()).collect())
            .collect();
        param_sets.sort();
        assert_eq!(
            param_sets,
            vec![vec!["int".to_string()], vec!["string".to_string()]]
        );
        assert_ne!(
            attacks[0].id, attacks[1].id,
            "overload ids must differ (§8)"
        );
    }

    #[test]
    fn modern_syntax_coverage() {
        let engine = engine_with(&[("Inventory.cs", INVENTORY_CS)]);
        let index = engine.build_index();
        let has = |qualified: &str, kind: SymbolKindCs| {
            index.by_qualified(qualified).iter().any(|s| s.kind == kind)
        };
        assert!(has("Game.Inventory.Item", SymbolKindCs::Record));
        assert!(has("Game.Inventory.Pixel", SymbolKindCs::RecordStruct));
        assert!(has("Game.Inventory.IEquippable", SymbolKindCs::Interface));
        assert!(has("Game.Inventory.Slot", SymbolKindCs::Enum));
        assert!(has("Game.Inventory.SlotChanged", SymbolKindCs::Delegate));
        assert!(has("Game.Inventory.Weight", SymbolKindCs::Struct));
        assert!(has("Game.Inventory.Slot.Head", SymbolKindCs::EnumMember));
        // file-scoped namespace（§11/§12）。
        assert!(has("Game.Inventory", SymbolKindCs::Namespace));
        // enum member 值面/序号不做语义断言——只验证存在。
    }

    #[test]
    fn syntax_error_does_not_kill_extraction() {
        let broken = r#"namespace Broken {
    class Ok { void Fine() { } }
    class Broken { void Oops( { }
"#;
        let engine = engine_with(&[("broken.cs", broken)]);
        let index = engine.build_index();
        // §30：恢复出的声明必须存活；qualified 可能因 ERROR 吞掉
        // namespace 头而降级（诚实限制——记录诊断而非伪造前缀）。
        assert!(
            !index.search("ok", Some(SymbolKindCs::Class)).is_empty(),
            "recovered declarations must survive syntax errors (§30); got: {:?}",
            index
                .by_id
                .values()
                .map(|s| (&s.kind, &s.qualified_name))
                .collect::<Vec<_>>()
        );
        let extraction = engine.extraction("broken.cs").unwrap();
        assert!(
            !extraction.diagnostics.is_empty(),
            "diagnostics must be recorded"
        );
    }

    #[test]
    fn incremental_no_ghost_symbols_and_reuse() {
        let mut engine = SymbolEngine::new();
        engine.update_file("A.cs", "namespace N; class Alpha { }");
        engine.update_file("B.cs", "namespace N; class Beta { }");
        let before = engine.build_index();
        assert!(before.by_qualified("N.Alpha").len() == 1);
        assert!(before.by_qualified("N.Beta").len() == 1);

        // 修改 A：Alpha 符号集合重建，B 不受影响（§28）。
        engine.update_file("A.cs", "namespace N; class Alpha2 { void M() { } }");
        let after = engine.build_index();
        assert!(after.by_qualified("N.Alpha").is_empty(), "no ghost symbols");
        assert!(after.by_qualified("N.Alpha2").len() == 1);
        assert!(after.by_qualified("N.Beta").len() == 1);

        // 相同内容 update = 复用（hash 未变，抽取结果不变）。
        let extraction_before = engine.extraction("A.cs").unwrap().clone();
        engine.update_file("A.cs", "namespace N; class Alpha2 { void M() { } }");
        assert_eq!(*engine.extraction("A.cs").unwrap(), extraction_before);

        // 删除文件：符号移除（§28）。
        engine.remove_file("A.cs");
        let final_index = engine.build_index();
        assert!(
            final_index.by_qualified("N.Alpha2").is_empty(),
            "deleted file ⇒ symbols removed"
        );
        assert!(final_index.by_qualified("N.Beta").len() == 1);
    }

    #[test]
    fn partial_type_aggregates_across_files() {
        let mut engine = SymbolEngine::new();
        engine.update_file(
            "Player.cs",
            "namespace G; public partial class Player { void Move() { } }",
        );
        engine.update_file(
            "Player.Inventory.cs",
            "namespace G; public partial class Player { void Drop() { } }",
        );
        let index = engine.build_index();
        let players = index.by_qualified("G.Player");
        assert_eq!(
            players.len(),
            1,
            "partial declarations aggregate into one logical symbol"
        );
        let player = players[0];
        assert!(player.is_partial);
        assert_eq!(player.declarations.len(), 2, "two declaration occurrences");
        assert_eq!(
            index.children(player.id.as_str()).len(),
            2,
            "members from both files"
        );
    }

    #[test]
    fn identity_stable_under_code_movement() {
        let mut engine = SymbolEngine::new();
        engine.update_file(
            "P.cs",
            "namespace G; class Player { void A() { }\n\n void B() { } }",
        );
        let first = engine.build_index();
        let id_before = first.by_qualified("G.Player.A")[0].id.clone();
        // 行号移动（插入空行）后 id 不变（§7：id 与行号无关）。
        engine.update_file(
            "P.cs",
            "namespace G; class Player {\n\n\n void A() { }\n\n void B() { } }",
        );
        let second = engine.build_index();
        assert_eq!(
            id_before,
            second.by_qualified("G.Player.A")[0].id,
            "code movement must not change identity"
        );
    }

    #[test]
    fn search_is_symbol_search_not_fulltext() {
        let engine = engine_with(&[
            ("P.cs", "namespace G; class Player { }"),
            ("C.cs", "// Player appears in a comment\nclass Card { }"),
        ]);
        let index = engine.build_index();
        let hits = index.search("play", None);
        assert_eq!(hits.len(), 1, "comment mentions are not symbols (§58)");
        assert_eq!(hits[0].name, "Player");
        let classes_only = index.search("c", Some(SymbolKindCs::Class));
        assert!(classes_only.iter().all(|s| s.kind == SymbolKindCs::Class));
    }

    #[test]
    fn serialization_round_trip_and_determinism() {
        let engine = engine_with(&[("Player.cs", PLAYER_CS)]);
        let index = engine.build_index();
        let text = serde_json::to_string(&index).unwrap();
        let back: SymbolIndex = serde_json::from_str(&text).unwrap();
        assert_eq!(back, index);
        assert_eq!(
            serde_json::to_string(&back).unwrap(),
            text,
            "deterministic serialization"
        );
    }

    #[test]
    fn intel_bundle_carries_entities_relations_and_facts() {
        let engine = engine_with(&[("Player.cs", PLAYER_CS)]);
        let index = engine.build_index();
        let identity = ProjectIdentity::from_root(ProjectId::from_string("proj:m23"), "C:/proj");
        let outcome = crate::intel::symbols_to_intel(&index, &identity, "t", 0);
        let bundle = outcome.data.unwrap();

        // 符号实体 + 文件实体都在（merge 时 file 与 M22 同口径去重）。
        assert!(bundle
            .entities
            .values()
            .any(|e| e.metadata.get("kind").map(String::as_str) == Some("class")));
        assert!(bundle
            .entities
            .values()
            .any(|e| e.display_name == "Player.cs"));
        // containment：class 包含 method。
        assert!(bundle
            .relations
            .values()
            .any(|r| r.relation_type == prism_intel::RelationType::contains()));
        // 事实：symbols_total > 0。
        assert!(bundle
            .facts
            .values()
            .any(|f| f.fact_type.as_str() == "symbols_total"));
        // SourceRange 证据（§25）。
        assert!(bundle
            .evidence
            .values()
            .any(|e| matches!(e.kind, prism_intel::EvidenceKind::SourceRange)));
        // 状态：无解析错误 ⇒ Complete。
        assert_eq!(outcome.status, prism_intel::IntelStatus::Complete);
    }
}
