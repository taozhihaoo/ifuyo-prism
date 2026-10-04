//! Symbol 身份（2.0-M23 §7-§9/§59）。
//!
//! SymbolId = `cs:{qualified}#{kind}#{sig8}`——project scope 由 IntelBundle
//! 的 EntityId 派生承担（与 M21/M22 同构）；qualified 含容器链但不含签名，
//! overload 用 signature 分量区分；行号/文件不参与身份（§7 禁止 file+line）。

use crate::model::{ParamInfo, RawSymbol, SymbolKindCs};

pub(crate) fn hash16(input: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

/// 可调用成员的规范签名分量：`(type1,type2)`（TypeSyntax 原样，不猜测语义，§8）。
pub(crate) fn signature_of(parameters: &[ParamInfo]) -> String {
    let types: Vec<String> = parameters.iter().map(|p| p.type_syntax.clone()).collect();
    format!("({})", types.join(","))
}

/// 逻辑符号稳定 id（跨运行、与位置无关；partial 的所有声明共享同一 id）。
pub fn symbol_id(kind: SymbolKindCs, qualified_name: &str, parameters: &[ParamInfo]) -> String {
    let signature = signature_of(parameters);
    let needs_signature = matches!(
        kind,
        SymbolKindCs::Method
            | SymbolKindCs::Constructor
            | SymbolKindCs::Operator
            | SymbolKindCs::Delegate
            | SymbolKindCs::LocalFunction
    );
    let suffix = if needs_signature && signature != "()" {
        hash16(&signature)
    } else {
        String::new()
    };
    if suffix.is_empty() {
        format!("cs:{qualified_name}#{}", kind.as_str())
    } else {
        format!("cs:{qualified_name}#{}#{suffix}", kind.as_str())
    }
}

/// Display 契约（§59）：`Ns.Type.Member(int,string)` 与身份字段分离。
pub fn display_name(symbol: &RawSymbol) -> String {
    let signature = signature_of(&symbol.parameters);
    if matches!(
        symbol.kind,
        SymbolKindCs::Method
            | SymbolKindCs::Constructor
            | SymbolKindCs::Delegate
            | SymbolKindCs::LocalFunction
            | SymbolKindCs::Operator
    ) {
        format!("{}{}", symbol.qualified_name, signature)
    } else {
        symbol.qualified_name.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(types: &[&str]) -> Vec<ParamInfo> {
        types
            .iter()
            .enumerate()
            .map(|(index, t)| ParamInfo {
                name: format!("p{index}"),
                type_syntax: (*t).to_string(),
            })
            .collect()
    }

    #[test]
    fn overloads_get_distinct_ids() {
        let a = symbol_id(
            SymbolKindCs::Method,
            "Game.Player.Attack",
            &params(&["int"]),
        );
        let b = symbol_id(
            SymbolKindCs::Method,
            "Game.Player.Attack",
            &params(&["string"]),
        );
        let c = symbol_id(
            SymbolKindCs::Method,
            "Game.Player.Attack",
            &params(&["int"]),
        );
        assert_ne!(a, b, "Foo(int) and Foo(string) must not collide");
        assert_eq!(a, c, "same overload must be identical");
    }

    #[test]
    fn id_is_line_and_file_independent() {
        // §7：位置不参与身份——同声明移动位置/改名文件，id 不变。
        let a = symbol_id(SymbolKindCs::Class, "Game.Entities.Player", &[]);
        let b = symbol_id(SymbolKindCs::Class, "Game.Entities.Player", &[]);
        assert_eq!(a, b);
        assert!(a.starts_with("cs:Game.Entities.Player#class"));
        assert!(!a.contains("line"));
    }

    #[test]
    fn non_callables_share_qualified_identity() {
        let field_a = symbol_id(SymbolKindCs::Field, "Game.Player.hp", &[]);
        let field_b = symbol_id(SymbolKindCs::Field, "Game.Player.hp", &params(&["int"]));
        assert_eq!(
            field_a, field_b,
            "fields have no overload semantics — signature ignored"
        );
    }
}
