# fixture-multi-language

A synthetic multi-language project (Node.js + TypeScript + Rust/Tauri + Python)
used by Prism's discovery tests.

Expected facts (asserted by tests):
- Project types: Tauri, Rust, Node.js, TypeScript (evidence-based candidates)
- Languages: TypeScript, Rust, Python, JSON, Markdown
- Entry candidates: `src/main.ts` (frontend), `src-tauri/src/main.rs` (Tauri backend)
- Configs: package.json, tsconfig.json, Cargo.toml, tauri.conf.json, README.md
- One test file (`tests/app.test.ts`) classified as Test
