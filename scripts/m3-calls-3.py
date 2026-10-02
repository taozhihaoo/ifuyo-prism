# -*- coding: utf-8 -*-
"""M3 call-graph part 3: fix remaining import lists and top-level walks."""

import io

# web.rs: CallInfo import + top-level walk adds None caller
p = "crates/prism-parser/src/languages/web.rs"
s = io.open(p, encoding="utf-8", newline="\n").read()
s = s.replace(
    """use crate::parse::{
    ImportInfo, ImportKind, ParseDiagnostic, ParseRequest, ParseResult, ParseStatus,
};""",
    """use crate::parse::{
    CallInfo, ImportInfo, ImportKind, ParseDiagnostic, ParseRequest, ParseResult,
    ParseStatus,
};""",
)
s = s.replace(
    """        collector.walk_children(
            tree.root_node(),
            Some(&file_symbol_id),
            &qualified_root,
            true,
        );""",
    """        collector.walk_children(
            tree.root_node(),
            Some(&file_symbol_id),
            &qualified_root,
            true,
            None,
        );""",
)
io.open(p, "w", encoding="utf-8", newline="\n").write(s)
print("web fixed")

# csharp.rs: top-level walk adds None caller
p = "crates/prism-parser/src/languages/csharp.rs"
s = io.open(p, encoding="utf-8", newline="\n").read()
s = s.replace(
    """        collector.walk_children(
            tree.root_node(),
            Some(&file_symbol_id),
            &qualified_root,
            false,
        );""",
    """        collector.walk_children(
            tree.root_node(),
            Some(&file_symbol_id),
            &qualified_root,
            false,
            None,
        );""",
)
io.open(p, "w", encoding="utf-8", newline="\n").write(s)
print("csharp fixed")
