# -*- coding: utf-8 -*-
"""Register calls module in lib.rs (idempotent)."""

import io

p = "crates/prism-graph/src/lib.rs"
s = io.open(p, encoding="utf-8", newline="\n").read()
if "pub mod calls;" not in s:
    s = s.replace("pub mod builder;", "pub mod builder;\npub mod calls;")
if "CallEdge," not in s:
    s = s.replace("pub use model::{\n    Cycle,", "pub use model::{\n    CallEdge, Cycle,")
if "pub use calls::resolve_calls;" not in s:
    s = s.replace("pub use resolve::{resolve_import, ResolutionContext};",
                  "pub use calls::resolve_calls;\npub use resolve::{resolve_import, ResolutionContext};")
io.open(p, "w", encoding="utf-8", newline="\n").write(s)
print("ok")
