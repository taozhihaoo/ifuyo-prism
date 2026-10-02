# -*- coding: utf-8 -*-
"""Regex-replace mangled binary fixture lines (byte-level, DOTALL, no escape in repl)."""

import io
import re

p = "crates/prism-git/tests/git_fixture.rs"
data = io.open(p, "rb").read()

pattern = re.compile(
    rb'std::fs::write\(fixture\.root\.join\("logo\.png"\), b"(?s:.*?)"\)\.unwrap\(\);'
)
replacement = (
    b'std::fs::write(fixture.root.join("logo.png"), b"\\x89PNG\\r\\n\\x1a\\nnot-really").unwrap();'
)
matches = pattern.findall(data)
print("found:", len(matches))
data, count = pattern.subn(lambda _m: replacement, data)
io.open(p, "wb").write(data)
print("replaced:", count)
