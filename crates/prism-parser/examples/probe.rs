use prism_core::ids::FileId;
use prism_core::task::CancellationToken;
use prism_parser::{LanguageParser, ParseRequest, WebParser};

fn main() {
    let src = "import { helper } from \"./helper\";\n\nexport const x = 1;\n";
    let parser = WebParser::typescript();
    let req = ParseRequest {
        file_id: FileId::from_string("file:a.ts"),
        relative_path: "a.ts".into(),
        language: prism_project::Language::TypeScript,
        content: src.to_string(),
    };
    let result = parser.parse(&req, &CancellationToken::new()).unwrap();
    println!("imports: {:?}", result.imports);
    let mut p = tree_sitter::Parser::new();
    p.set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
        .unwrap();
    let tree = p.parse(src, None).unwrap();
    let root = tree.root_node();
    let mut child = root.child(0);
    while let Some(c) = child {
        println!("child kind: {}", c.kind());
        child = c.next_sibling();
    }
}
