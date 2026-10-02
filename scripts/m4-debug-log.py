# -*- coding: utf-8 -*-
"""Add debug eprintln to parse_log skip branch (literal escape text)."""

import io

p = "crates/prism-git/src/parse.rs"
s = io.open(p, encoding="utf-8", newline="\n").read()

old = (
    "        let fields: Vec<&str> = record.split('\\u{1f}').collect();\n"
    "        if fields.len() < 10 {\n"
    "            continue;\n"
    "        }"
)
new = (
    "        let fields: Vec<&str> = record.split('\\u{1f}').collect();\n"
    "        if fields.len() < 10 {\n"
    "            eprintln!(\n"
    "                \"DEBUG parse_log skip: {} fields, first={:?}\",\n"
    "                fields.len(),\n"
    "                fields.first().map(|f| f.chars().take(24).collect::<String>())\n"
    "            );\n"
    "            continue;\n"
    "        }"
)
assert old in s, "pattern not found"
s = s.replace(old, new)
io.open(p, "w", encoding="utf-8", newline="\n").write(s)
print("ok")
