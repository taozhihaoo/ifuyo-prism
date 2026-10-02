# -*- coding: utf-8 -*-
"""M3 call-graph part 2: capture direct calls in the three adapters."""

import io
import re

CallInfo = "CallInfo"


def add_callinfo_import(s, crate_line):
    if "CallInfo" not in s.split("\n\n")[0] and "use crate::parse::" in s:
        pass
    return s


# ---------------- web.rs ----------------
p = "crates/prism-parser/src/languages/web.rs"
s = io.open(p, encoding="utf-8", newline="\n").read()

s = s.replace(
    "use crate::parse::{ImportInfo, ImportKind, ParseDiagnostic, ParseRequest, ParseResult, ParseStatus};",
    "use crate::parse::{\n    CallInfo, ImportInfo, ImportKind, ParseDiagnostic, ParseRequest, ParseResult,\n    ParseStatus,\n};",
)

# Collector field
s = s.replace(
    "    imports: Vec<ImportInfo>,\n    cancel: &'a CancellationToken,\n}",
    "    imports: Vec<ImportInfo>,\n    calls: Vec<CallInfo>,\n    cancel: &'a CancellationToken,\n}",
)

# signatures
s = s.replace(
    "    fn walk(&mut self, node: Node<'a>, parent_id: Option<&SymbolId>, prefix: &str, in_class: bool) {",
    "    fn walk(\n        &mut self,\n        node: Node<'a>,\n        parent_id: Option<&SymbolId>,\n        prefix: &str,\n        in_class: bool,\n        caller: Option<&str>,\n    ) {",
)
s = s.replace(
    "    fn walk_error(\n        &mut self,\n        node: Node<'a>,\n        parent_id: Option<&SymbolId>,\n        prefix: &str,\n        in_class: bool,\n    ) {",
    "    fn walk_error(\n        &mut self,\n        node: Node<'a>,\n        parent_id: Option<&SymbolId>,\n        prefix: &str,\n        in_class: bool,\n        caller: Option<&str>,\n    ) {",
)
s = s.replace(
    "    fn walk_children(\n        &mut self,\n        node: Node<'a>,\n        parent_id: Option<&SymbolId>,\n        prefix: &str,\n        in_class: bool,\n    ) {",
    "    fn walk_children(\n        &mut self,\n        node: Node<'a>,\n        parent_id: Option<&SymbolId>,\n        prefix: &str,\n        in_class: bool,\n        caller: Option<&str>,\n    ) {",
)
s = s.replace(
    "    fn walk_plain_children(\n        &mut self,\n        node: Node<'a>,\n        parent_id: Option<&SymbolId>,\n        prefix: &str,\n        in_class: bool,\n        kind: SymbolKind,\n    ) {",
    "    fn walk_plain_children(\n        &mut self,\n        node: Node<'a>,\n        parent_id: Option<&SymbolId>,\n        prefix: &str,\n        in_class: bool,\n        caller: Option<&str>,\n        kind: SymbolKind,\n    ) {",
)

# call sites: function/method bodies get caller = Some(&qualified); others propagate
s = s.replace("self.walk_children(node, Some(&symbol.id), &qualified, false);",
              "self.walk_children(node, Some(&symbol.id), &qualified, false, Some(&qualified));")
s = s.replace("self.walk_children(node, Some(&symbol.id), &qualified, true);",
              "self.walk_children(node, Some(&symbol.id), &qualified, true, Some(&qualified));")
s = s.replace("self.walk_plain_children(node, parent_id, prefix, in_class, SymbolKind::Constant);",
              "self.walk_plain_children(node, parent_id, prefix, in_class, caller, SymbolKind::Constant);")
s = s.replace("self.walk_children(node, parent_id, prefix, in_class);",
              "self.walk_children(node, parent_id, prefix, in_class, caller);")
s = s.replace('"ERROR" => self.walk_error(node, parent_id, prefix, in_class),',
              '"ERROR" => self.walk_error(node, parent_id, prefix, in_class, caller),')
s = s.replace("_ => self.walk_children(node, parent_id, prefix, in_class),",
              "_ => self.walk_children(node, parent_id, prefix, in_class, caller),")
s = s.replace("self.walk(current, owner_id, owner_prefix, true);",
              "self.walk(current, owner_id, owner_prefix, true, None);")
s = s.replace("self.walk(current, parent_id, prefix, in_class);",
              "self.walk(current, parent_id, prefix, in_class, caller);")

# walk_children internals: propagate caller to self.walk
s = s.replace("""        let mut child = node.child(0);
        while let Some(current) = child {
            self.walk(current, parent_id, prefix, in_class, caller);
            child = current.next_sibling();
        }
    }
}""", """        let mut child = node.child(0);
        while let Some(current) = child {
            self.walk(current, parent_id, prefix, in_class, caller);
            child = current.next_sibling();
        }
    }
}""")

# insert call_expression arm before ERROR arm
old_error = '            "ERROR" => self.walk_error(node, parent_id, prefix, in_class, caller),'
new_arm = '''            "call_expression" => {
                // 规格 §19 可靠子集：直接调用（callee 文本原样记录）；
                // 只有存在宿主函数/方法时才捕获（顶层初始化代码不属于调用图）。
                if let Some(caller) = caller {
                    if let Some(function) = node.child_by_field_name("function") {
                        self.calls.push(CallInfo {
                            caller_qualified: caller.to_string(),
                            callee_text: WebParser::text(function, self.content).to_string(),
                            location: Some(WebParser::node_location(node, false)),
                        });
                    }
                }
                self.walk_children(node, parent_id, prefix, in_class, caller);
            }
''' + old_error
assert old_error in s
s = s.replace(old_error, new_arm, 1)

# walk_error internals: propagate caller into structured routing
s = s.replace("                    self.walk(current, owner_id, owner_prefix, true);",
              "                    self.walk(current, owner_id, owner_prefix, true, caller);")

# Collector construction already has calls: Vec::new() (blanket). ParseResult assembly:
s = s.replace("""            imports: collector.imports,
        })
    }
}""", """            imports: collector.imports,
            calls: collector.calls,
        })
    }
}""")

io.open(p, "w", encoding="utf-8", newline="\n").write(s)
print("web done")

# ---------------- python.rs ----------------
p = "crates/prism-parser/src/languages/python.rs"
s = io.open(p, encoding="utf-8", newline="\n").read()
s = s.replace(
    "use crate::parse::{ImportInfo, ImportKind, ParseDiagnostic, ParseRequest, ParseResult, ParseStatus};",
    "use crate::parse::{\n    CallInfo, ImportInfo, ImportKind, ParseDiagnostic, ParseRequest, ParseResult,\n    ParseStatus,\n};",
)
s = s.replace(
    "    imports: Vec<ImportInfo>,\n    cancel: &'a CancellationToken,\n}",
    "    imports: Vec<ImportInfo>,\n    calls: Vec<CallInfo>,\n    cancel: &'a CancellationToken,\n}",
)
s = s.replace(
    "    fn walk(&mut self, node: Node<'a>, parent_id: Option<&SymbolId>, prefix: &str, in_class: bool) {",
    "    fn walk(\n        &mut self,\n        node: Node<'a>,\n        parent_id: Option<&SymbolId>,\n        prefix: &str,\n        in_class: bool,\n        caller: Option<&str>,\n    ) {",
)
s = s.replace(
    "    fn walk_children(\n        &mut self,\n        node: Node<'a>,\n        parent_id: Option<&SymbolId>,\n        prefix: &str,\n        in_class: bool,\n    ) {",
    "    fn walk_children(\n        &mut self,\n        node: Node<'a>,\n        parent_id: Option<&SymbolId>,\n        prefix: &str,\n        in_class: bool,\n        caller: Option<&str>,\n    ) {",
)
s = s.replace("self.walk_children(node, Some(&symbol.id), &qualified, false);",
              "self.walk_children(node, Some(&symbol.id), &qualified, false, Some(&qualified));")
s = s.replace("self.walk_children(node, Some(&symbol.id), &qualified, true);",
              "self.walk_children(node, Some(&symbol.id), &qualified, true, Some(&qualified));")
s = s.replace("self.walk_children(node, parent_id, prefix, in_class);",
              "self.walk_children(node, parent_id, prefix, in_class, caller);")
s = s.replace('"ERROR" => self.walk_children(node, parent_id, prefix, in_class),',
              '"ERROR" => self.walk_children(node, parent_id, prefix, in_class, caller),')
s = s.replace("_ => self.walk_children(node, parent_id, prefix, in_class),",
              "_ => self.walk_children(node, parent_id, prefix, in_class, caller),")
s = s.replace('"block" => self.walk_children(current, parent_id, prefix, in_class),',
              '"block" => self.walk_children(current, parent_id, prefix, in_class, caller),')
s = s.replace("_ => self.walk(current, parent_id, prefix, in_class),",
              "_ => self.walk(current, parent_id, prefix, in_class, caller),")

# call arm before assignment arm
old_assign = '            "assignment" => {'
new_call = '''            "call" => {
                // 规格 §19 可靠子集：直接调用，callee = function 字段文本。
                if let Some(caller) = caller {
                    if let Some(function) = node.child_by_field_name("function") {
                        self.calls.push(CallInfo {
                            caller_qualified: caller.to_string(),
                            callee_text: PythonParser::text(function, self.content).to_string(),
                            location: Some(PythonParser::node_location(node, false)),
                        });
                    }
                }
                self.walk_children(node, parent_id, prefix, in_class, caller);
            }
            "assignment" => {'''
assert old_assign in s
s = s.replace(old_assign, new_call, 1)

s = s.replace("""            imports: collector.imports,
        })
    }
}""", """            imports: collector.imports,
            calls: collector.calls,
        })
    }
}""")

io.open(p, "w", encoding="utf-8", newline="\n").write(s)
print("python done")

# ---------------- csharp.rs ----------------
p = "crates/prism-parser/src/languages/csharp.rs"
s = io.open(p, encoding="utf-8", newline="\n").read()
s = s.replace(
    "use crate::parse::{ImportInfo, ParseDiagnostic, ParseRequest, ParseResult, ParseStatus};",
    "use crate::parse::{\n    CallInfo, ImportInfo, ParseDiagnostic, ParseRequest, ParseResult, ParseStatus,\n};",
)
s = s.replace(
    "    imports: Vec<ImportInfo>,\n    cancel: &'a CancellationToken,\n}",
    "    imports: Vec<ImportInfo>,\n    calls: Vec<CallInfo>,\n    cancel: &'a CancellationToken,\n}",
)
s = s.replace(
    "    fn walk(&mut self, node: Node<'a>, parent_id: Option<&SymbolId>, prefix: &str, top_level: bool) {",
    "    fn walk(\n        &mut self,\n        node: Node<'a>,\n        parent_id: Option<&SymbolId>,\n        prefix: &str,\n        top_level: bool,\n        caller: Option<&str>,\n    ) {",
)
s = s.replace(
    "    fn walk_error(\n        &mut self,\n        node: Node<'a>,\n        parent_id: Option<&SymbolId>,\n        prefix: &str,\n        top_level: bool,\n    ) {",
    "    fn walk_error(\n        &mut self,\n        node: Node<'a>,\n        parent_id: Option<&SymbolId>,\n        prefix: &str,\n        _top_level: bool,\n        caller: Option<&str>,\n    ) {",
)
s = s.replace(
    "    fn walk_children(\n        &mut self,\n        node: Node<'a>,\n        parent_id: Option<&SymbolId>,\n        prefix: &str,\n        top_level: bool,\n    ) {",
    "    fn walk_children(\n        &mut self,\n        node: Node<'a>,\n        parent_id: Option<&SymbolId>,\n        prefix: &str,\n        top_level: bool,\n        caller: Option<&str>,\n    ) {",
)
s = s.replace("self.walk_children(node, Some(&symbol.id), &qualified, false);",
              "self.walk_children(node, Some(&symbol.id), &qualified, false, Some(&qualified));")
s = s.replace('"ERROR" => self.walk_error(node, parent_id, prefix, top_level),',
              '"ERROR" => self.walk_error(node, parent_id, prefix, top_level, caller),')
s = s.replace("_ => self.walk_children(node, parent_id, prefix, top_level),",
              "_ => self.walk_children(node, parent_id, prefix, top_level, caller),")
s = s.replace("                    self.walk(current, owner_id, owner_prefix, false);",
              "                    self.walk(current, owner_id, owner_prefix, false, None);")
s = s.replace("            self.walk(current, parent_id, prefix, top_level);",
              "            self.walk(current, parent_id, prefix, top_level, caller);")

# invocation arm before using_directive
old_using = '            "using_directive" => {'
new_inv = '''            "invocation_expression" => {
                // 规格 §19 可靠子集：直接调用，callee = expression 字段文本。
                if let Some(caller) = caller {
                    if let Some(function) = node.child_by_field_name("expression") {
                        self.calls.push(CallInfo {
                            caller_qualified: caller.to_string(),
                            callee_text: CSharpParser::text(function, self.content).to_string(),
                            location: Some(CSharpParser::node_location(node, false)),
                        });
                    }
                }
                self.walk_children(node, parent_id, prefix, top_level, caller);
            }
            "using_directive" => {'''
assert old_using in s
s = s.replace(old_using, new_inv, 1)

# method_declaration / constructor_declaration body walk: caller = qualified
s = s.replace("""                    let symbol = self.emit(node, SymbolKind::Method, &qualified, parent_id, visibility, signature);
                    self.walk_children(node, Some(&symbol.id), &qualified, false, Some(&qualified));""",
"""                    let symbol = self.emit(node, SymbolKind::Method, &qualified, parent_id, visibility, signature);
                    self.walk_children(node, Some(&symbol.id), &qualified, false, Some(&qualified));""")

s = s.replace("""            parser_version: self.version.to_string(),
            reason: None,
            imports: collector.imports,
        })
    }
}""", """            parser_version: self.version.to_string(),
            reason: None,
            imports: collector.imports,
            calls: collector.calls,
        })
    }
}""")

io.open(p, "w", encoding="utf-8", newline="\n").write(s)
print("csharp done")
