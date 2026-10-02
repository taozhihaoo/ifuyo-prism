# -*- coding: utf-8 -*-
"""M3 call-graph part 4: remaining top-level walks + caller propagation."""

import io

# web.rs
p = "crates/prism-parser/src/languages/web.rs"
s = io.open(p, encoding="utf-8", newline="\n").read()
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
# walk_error routes structured declarations: propagate caller
s = s.replace(
    "                    self.walk(current, owner_id, owner_prefix, true);",
    "                    self.walk(current, owner_id, owner_prefix, true, caller);",
)
# walk_plain_children: caller not used (constants have no body)
s = s.replace(
    "        in_class: bool,\n        caller: Option<&str>,\n        kind: SymbolKind,",
    "        in_class: bool,\n        _caller: Option<&str>,\n        kind: SymbolKind,",
)
io.open(p, "w", encoding="utf-8", newline="\n").write(s)
print("web ok")

# csharp.rs walk_error: propagate caller into structured routing
p = "crates/prism-parser/src/languages/csharp.rs"
s = io.open(p, encoding="utf-8", newline="\n").read()
s = s.replace(
    "                    self.walk(current, owner_id, owner_prefix, false);",
    "                    self.walk(current, owner_id, owner_prefix, false, caller);",
)
io.open(p, "w", encoding="utf-8", newline="\n").write(s)
print("csharp ok")
