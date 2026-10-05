//! csproj / props 解析（M29 §12/§13；Godot+C# 第一优先 §83）。
//!
//! 手写标签扫描（项目文件是**不可信输入** §113：只解析数据、不执行、
//! 无外部 XML 依赖；malformed ⇒ 尽力而为 + 诊断，不 panic）。

use prism_intel::evidence::EvidenceId;
use serde::{Deserialize, Serialize};

/// 单个 csproj 解析产物（声明层）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CsprojProject {
    pub manifest_path: String,
    pub sdk: Option<String>,
    pub target_framework: Option<String>,
    /// `<PackageReference Include Version />`（Version 缺省 ⇒ CPM/无版本）。
    pub package_references: Vec<CsprojReference>,
    /// `<ProjectReference Include="..\B\B.csproj" />`（原样路径）。
    pub project_references: Vec<CsprojReference>,
    /// 解析诊断（未闭合标签/属性等，诚实降级 §56）。
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CsprojReference {
    pub include: String,
    pub version: Option<String>,
    /// 1-based 行号（证据定位）。
    pub line: usize,
    pub evidence: Vec<EvidenceId>,
}

/// Directory.Packages.props 的中央版本项（§13）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageVersion {
    pub include: String,
    pub version: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CentralPackages {
    pub manifest_path: String,
    pub manage_centrally: bool,
    pub package_versions: Vec<PackageVersion>,
    pub diagnostics: Vec<String>,
}

/// 有界读取上限（与仓库其它分析路径一致的 1 MiB manifest 预算）。
pub const CSPROJ_MAX_BYTES: usize = 1024 * 1024;

/// 解析 csproj（.NET SDK 风格工程的 PackageReference/ProjectReference/
/// TargetFramework/Root Sdk 属性）。
pub fn parse_csproj(content: &str, manifest_path: &str) -> CsprojProject {
    let mut project = CsprojProject {
        manifest_path: manifest_path.to_string(),
        ..Default::default()
    };
    let bytes = content.as_bytes();
    if bytes.len() > CSPROJ_MAX_BYTES {
        project.diagnostics.push(format!(
            "{manifest_path} exceeds {CSPROJ_MAX_BYTES} bytes; parsed truncated"
        ));
    }
    let mut tags = scan_tags(content, &mut project.diagnostics);
    // <Project Sdk="..."> 是首标签（存在才记录）。
    if let Some(first) = tags.first() {
        if first.name == "Project" {
            project.sdk = first.attribute("Sdk").map(str::to_string);
        }
    }
    for tag in tags.drain(..) {
        match tag.name.as_str() {
            "PackageReference" => {
                // Include= 声明新引用；Update= 调整既有引用（props/targets 常见）。
                if let Some(include) = tag.attribute("Include").or_else(|| tag.attribute("Update"))
                {
                    project.package_references.push(CsprojReference {
                        include: include.to_string(),
                        version: tag.attribute("Version").map(str::to_string),
                        line: tag.line,
                        evidence: Vec::new(),
                    });
                }
            }
            "ProjectReference" => {
                if let Some(include) = tag.attribute("Include") {
                    project.project_references.push(CsprojReference {
                        include: include.to_string(),
                        version: None,
                        line: tag.line,
                        evidence: Vec::new(),
                    });
                }
            }
            _ => {}
        }
    }
    if let Some(tf) = element_text(content, "TargetFramework") {
        project.target_framework = Some(tf.trim().to_string());
    }
    project
}

/// 解析 Directory.Packages.props（§13 Central Package Management）。
pub fn parse_directory_packages_props(content: &str, manifest_path: &str) -> CentralPackages {
    let mut central = CentralPackages {
        manifest_path: manifest_path.to_string(),
        ..Default::default()
    };
    let mut tags = scan_tags(content, &mut central.diagnostics);
    for tag in tags.drain(..) {
        if tag.name == "PackageVersion" {
            if let (Some(include), Some(version)) =
                (tag.attribute("Include"), tag.attribute("Version"))
            {
                central.package_versions.push(PackageVersion {
                    include: include.to_string(),
                    version: version.to_string(),
                    line: tag.line,
                });
            }
        }
    }
    central.manage_centrally = element_text(content, "ManagePackageVersionsCentrally")
        .map(|text| text.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    central
        .package_versions
        .sort_by(|a, b| a.include.cmp(&b.include));
    central
}

/// 扫描出的开标签（含属性；文本元素由 `element_text` 单独取）。
struct RawTag {
    name: String,
    line: usize,
    attributes: Vec<(String, String)>,
}

impl RawTag {
    fn attribute(&self, key: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }
}

/// 手写扫描：`<Name key="value" />` 与 `<Name key="value">`（开标签）。
/// 闭合标签/注释/declaration 跳过；文本元素**不**整体消费——容器内的
/// 内层标签必须继续扫描（否则 `<Project>` 根元素会把整个文件当文本
/// 吞掉）。未闭合 ⇒ 截止 + 诊断（不 panic、不吞，§56）。
fn scan_tags(content: &str, diagnostics: &mut Vec<String>) -> Vec<RawTag> {
    let mut tags = Vec::new();
    let bytes = content.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] != b'<' {
            index += 1;
            continue;
        }
        let start = index;
        let line = content[..start].bytes().filter(|b| *b == b'\n').count() + 1;
        let mut cursor = index + 1;
        let name_start = cursor;
        while cursor < bytes.len()
            && !bytes[cursor].is_ascii_whitespace()
            && bytes[cursor] != b'>'
            && bytes[cursor] != b'/'
        {
            cursor += 1;
        }
        let name: String = content[name_start..cursor].to_string();
        if name.is_empty()
            || name.starts_with('!')
            || name.starts_with('?')
            || name.starts_with('/')
        {
            index = match content[start..].find('>') {
                Some(offset) => start + offset + 1,
                None => bytes.len(),
            };
            continue;
        }
        // 属性区直到 '>'（引号内的 '>' 属于值）。
        let mut attributes = Vec::new();
        let mut closed = false;
        while cursor < bytes.len() {
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor >= bytes.len() {
                break;
            }
            if bytes[cursor] == b'>' {
                closed = true;
                cursor += 1;
                break;
            }
            if bytes[cursor] == b'/' {
                match content[cursor..].find('>') {
                    Some(offset) if offset <= 2 => {
                        closed = true;
                        cursor += offset + 1;
                    }
                    _ => {}
                }
                break;
            }
            let key_start = cursor;
            while cursor < bytes.len()
                && bytes[cursor] != b'='
                && bytes[cursor] != b'>'
                && !bytes[cursor].is_ascii_whitespace()
            {
                cursor += 1;
            }
            let key = content[key_start..cursor].to_string();
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor < bytes.len() && bytes[cursor] == b'=' {
                cursor += 1;
                while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                let quote = bytes[cursor];
                if quote == b'"' || quote == b'\'' {
                    cursor += 1;
                    let value_start = cursor;
                    while cursor < bytes.len() && bytes[cursor] != quote {
                        cursor += 1;
                    }
                    if cursor >= bytes.len() {
                        diagnostics
                            .push(format!("{line}: unterminated attribute value for `{key}`"));
                        return tags;
                    }
                    attributes.push((key, content[value_start..cursor].to_string()));
                    cursor += 1;
                } else {
                    let value_start = cursor;
                    while cursor < bytes.len()
                        && !bytes[cursor].is_ascii_whitespace()
                        && bytes[cursor] != b'>'
                    {
                        cursor += 1;
                    }
                    attributes.push((key, content[value_start..cursor].to_string()));
                }
            } else if !key.is_empty() {
                attributes.push((key, String::new()));
            }
        }
        if !closed {
            diagnostics.push(format!("{line}: unterminated tag `<{name}`"));
            return tags;
        }
        index = cursor;
        tags.push(RawTag {
            name,
            line,
            attributes,
        });
    }
    tags
}

/// `<Name>text</Name>` 的直接文本捕获（无嵌套；找不到为 None）。
fn element_text(content: &str, name: &str) -> Option<String> {
    let open = format!("<{name}>");
    let open_at = content.find(&open)?;
    let rest = &content[open_at + open.len()..];
    let close = format!("</{name}>");
    let close_at = rest.find(&close)?;
    Some(rest[..close_at].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CSPROJ: &str = r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
  </PropertyGroup>
  <ItemGroup>
    <PackageReference Include="Newtonsoft.Json" Version="13.0.3" />
    <PackageReference Include="GodotSharp" />
    <ProjectReference Include="..\Lib\Lib.csproj" />
  </ItemGroup>
</Project>
"#;

    #[test]
    fn csproj_declarations_and_framework() {
        let project = parse_csproj(CSPROJ, "src/App/App.csproj");
        assert_eq!(project.sdk.as_deref(), Some("Microsoft.NET.Sdk"));
        assert_eq!(project.target_framework.as_deref(), Some("net8.0"));
        assert_eq!(project.package_references.len(), 2);
        assert_eq!(project.package_references[0].include, "Newtonsoft.Json");
        assert_eq!(
            project.package_references[0].version.as_deref(),
            Some("13.0.3")
        );
        assert!(
            project.package_references[1].version.is_none(),
            "CPM 场景无内联版本"
        );
        assert_eq!(project.project_references.len(), 1);
        assert_eq!(project.project_references[0].include, "..\\Lib\\Lib.csproj");
        assert_eq!(project.package_references[0].line, 6);
        assert!(project.diagnostics.is_empty());
    }

    #[test]
    fn malformed_csproj_reports_diagnostic_not_panic() {
        let project = parse_csproj(
            "<Project Sdk=\"A\n  <PackageReference Include=\"X",
            "broken.csproj",
        );
        assert!(!project.diagnostics.is_empty(), "未闭合属性 ⇒ 诊断");
    }

    #[test]
    fn central_packages_props() {
        let props = r#"<Project>
  <PropertyGroup>
    <ManagePackageVersionsCentrally>true</ManagePackageVersionsCentrally>
  </PropertyGroup>
  <ItemGroup>
    <PackageVersion Include="Newtonsoft.Json" Version="13.0.3" />
    <PackageVersion Include="Serilog" Version="4.0.0" />
  </ItemGroup>
</Project>
"#;
        let central = parse_directory_packages_props(props, "Directory.Packages.props");
        assert!(central.manage_centrally);
        assert_eq!(central.package_versions.len(), 2);
        assert_eq!(central.package_versions[0].include, "Newtonsoft.Json");
        assert_eq!(central.package_versions[0].version, "13.0.3");
    }
}
