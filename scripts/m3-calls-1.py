# -*- coding: utf-8 -*-
"""M3 call-graph part 1: CallInfo model + ParseResult.calls + literals."""

import io

# 1) parse.rs: CallInfo + ParseResult.calls
p = "crates/prism-parser/src/parse.rs"
s = io.open(p, encoding="utf-8", newline="\n").read()

anchor = "/// 解析请求：按需读取的单文件内容（解析后即丢弃，不驻留）。"
callinfo = """/// 一条直接调用语法事实（规格 §19 可靠子集：仅直接函数/方法调用）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallInfo {
    /// 调用方所在函数/方法的限定路径；顶层调用无宿主时不捕获。
    pub caller_qualified: String,
    /// 被调者的原样文本（如 "helper" / "obj.method" / "svc.fetch"）。
    pub callee_text: String,
    /// 调用表达式起始位置。
    pub location: Option<crate::model::SourceLocation>,
}

/// 解析请求：按需读取的单文件内容（解析后即丢弃，不驻留）。"""
assert anchor in s
s = s.replace(anchor, callinfo, 1)

old_field = """    /// 文件内全部 import 语法事实（M3 起；M2 已构造处补空 Vec）。
    #[serde(default)]
    pub imports: Vec<ImportInfo>,
}"""
new_field = """    /// 文件内全部 import 语法事实（M3 起；M2 已构造处补空 Vec）。
    #[serde(default)]
    pub imports: Vec<ImportInfo>,
    /// 文件内全部直接调用语法事实（M3 起）。
    #[serde(default)]
    pub calls: Vec<CallInfo>,
}"""
assert old_field in s
s = s.replace(old_field, new_field, 1)
io.open(p, "w", encoding="utf-8", newline="\n").write(s)
print("parse.rs ok")

# 2) blanket: add calls: Vec::new() after every imports line in ParseResult literals
files = [
    "crates/prism-parser/src/registry.rs",
    "crates/prism-parser/src/parse.rs",
    "crates/prism-parser/src/languages/csharp.rs",
    "crates/prism-parser/src/languages/python.rs",
    "crates/prism-parser/src/languages/web.rs",
    "crates/prism-analysis/src/index.rs",
    "crates/prism-analysis/src/service.rs",
]
for p in files:
    s = io.open(p, encoding="utf-8", newline="\n").read()
    count = 0
    out = []
    for line in s.split("\n"):
        out.append(line)
        stripped = line.strip()
        indent = line[: len(line) - len(line.lstrip())]
        if stripped == "imports: Vec::new()," :
            out.append(indent + "calls: Vec::new(),")
            count += 1
    s2 = "\n".join(out)
    # dedupe accidental doubles
    while "calls: Vec::new(),\n            calls: Vec::new()," in s2 or "calls: Vec::new(),\n        calls: Vec::new()," in s2 or "calls: Vec::new(),\n                calls: Vec::new()," in s2:
        s2 = s2.replace("calls: Vec::new(),\n            calls: Vec::new(),", "calls: Vec::new(),")
        s2 = s2.replace("calls: Vec::new(),\n        calls: Vec::new(),", "calls: Vec::new(),")
        s2 = s2.replace("calls: Vec::new(),\n                calls: Vec::new(),", "calls: Vec::new(),")
    io.open(p, "w", encoding="utf-8", newline="\n").write(s2)
    print(p, "inserted:", count)
