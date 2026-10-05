//! Endpoint Discovery（M213 §15/§16）：从实际项目代码发现 API Endpoint。
//!
//! 第一优先（§15 按真实框架）：ASP.NET Core attribute 路由
//! （`[HttpGet("...")]` / `[Route]` + Controller 类名前缀）与
//! Express/FastAPI 风格（`app.get("/x")` / `@app.get("/x")`）。
//! 证据 = 文件:行（§4 Evidence-first）；发现结果是 FACT，
//! handler 关联是 INFERENCE。

use crate::model::{ApiEndpoint, EndpointSource, HttpMethod};
use prism_intel::identity::fnv1a_hex16;
use std::path::Path;

/// 单个发现候选（源文件相对路径 + 行 + 属性文本）。
pub struct DiscoverInput<'a> {
    /// (项目相对路径, 文本内容)——调用方有界读取后注入。
    pub files: &'a [(String, String)],
}

/// 从注入的源文件发现端点（确定性：按 (method, route) 排序）。
///
/// ASP.NET 组合语义：类级 `[Route("api/[controller]")]` 是前缀，
/// 方法级 `[HttpGet]`（空）= 前缀本身，`[HttpPost("x")]` = 前缀 + x。
pub fn discover_endpoints(input: &DiscoverInput) -> Vec<ApiEndpoint> {
    let mut out = Vec::new();
    for (path, content) in input.files {
        let is_cs = path.ends_with(".cs");
        let mut route_prefix = String::new();
        for (line_number, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            // C#: `[HttpGet("route")]` / `[HttpPost]` / `[Route("api/x")]`。
            if is_cs {
                if let Some(route) = parse_cs_route_attribute(trimmed) {
                    // 类级前缀：`api/[controller]` ⇒ controller 名替换。
                    let base = route.replace(
                        "[controller]",
                        &controller_name(content).unwrap_or_default(),
                    );
                    route_prefix = normalize_route(&base);
                    continue;
                }
                if let Some((method, route)) = parse_cs_http_attribute(trimmed) {
                    let combined = if route.is_empty() {
                        route_prefix.clone()
                    } else if route.starts_with('/') {
                        // 以 / 开头的绝对路由覆盖前缀（ASP.NET 语义）。
                        normalize_route(&route)
                    } else {
                        normalize_route(&format!("{route_prefix}/{route}"))
                    };
                    if !combined.is_empty() && combined != "/" {
                        out.push(make_endpoint(method, &combined, path, line_number + 1));
                    }
                    continue;
                }
            }
            // Express: `app.get("/x"` / `router.post("/y"`。
            if let Some((method, route)) = parse_express(trimmed) {
                out.push(make_endpoint(method, &route, path, line_number + 1));
                continue;
            }
            // FastAPI: `@app.get("/x")`。
            if let Some((method, route)) = parse_fastapi(trimmed) {
                out.push(make_endpoint(method, &route, path, line_number + 1));
            }
        }
    }
    out.sort_by(|a, b| {
        a.method
            .cmp(&b.method)
            .then_with(|| a.route_template.cmp(&b.route_template))
    });
    out.dedup_by(|a, b| a.endpoint_id == b.endpoint_id);
    out
}

fn make_endpoint(method: HttpMethod, route: &str, path: &str, line: usize) -> ApiEndpoint {
    let normalized = normalize_route(route);
    ApiEndpoint {
        endpoint_id: format!(
            "api:{}",
            fnv1a_hex16(&format!("{} {}", method.as_str(), normalized))
        ),
        method,
        route_template: normalized,
        source: EndpointSource::SourceCode,
        evidence: format!("{path}:{line}"),
        handler_symbol: None,
    }
}

/// 归一化路由模板（§7）：路径参数 `{id}` / `:id` / `<id>` 统一为 `{id}`。
pub fn normalize_route(route: &str) -> String {
    let trimmed = route.trim().trim_matches('"').trim_matches('\'');
    let mut segments: Vec<String> = Vec::new();
    for segment in trimmed.split('/') {
        if segment.is_empty() {
            continue;
        }
        let parameterized = segment.starts_with('{')
            || segment.starts_with(':')
            || segment.starts_with('<')
            || segment.starts_with('*');
        segments.push(if parameterized {
            "{id}".to_string()
        } else {
            segment.to_string()
        });
    }
    let joined = segments.join("/");
    format!("/{joined}")
}

/// C# `[HttpXxx("route")]` / `[HttpXxx]`。
fn parse_cs_http_attribute(line: &str) -> Option<(HttpMethod, String)> {
    for method in [
        HttpMethod::Get,
        HttpMethod::Post,
        HttpMethod::Put,
        HttpMethod::Patch,
        HttpMethod::Delete,
    ] {
        let name = format!("[{}", method.as_str());
        let verb = format!("Http{}", method.as_str());
        let lower = line.to_ascii_lowercase();
        if lower.starts_with(&format!("[{}", verb.to_ascii_lowercase())) {
            // 带路由：[HttpGet("api/x")]。
            if let Some(open) = line.find('(') {
                if let Some(close) = line[open..].find(')') {
                    let inner = line[open + 1..open + close].trim();
                    if inner.is_empty() {
                        return Some((method, String::new()));
                    }
                    let route = inner
                        .split(',')
                        .next()
                        .unwrap_or(inner)
                        .trim()
                        .trim_matches('"')
                        .to_string();
                    return Some((method, route));
                }
            }
            return Some((method, String::new()));
        }
        let _ = name;
    }
    None
}

/// C# `[Route("...")]`（类级前缀；[controller] 占位）。
fn parse_cs_route_attribute(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    if !lower.starts_with("[route") {
        return None;
    }
    let open = line.find('(')?;
    let close = line[open..].find(')')?;
    let inner = line[open + 1..open + close].trim();
    let route = inner
        .split(',')
        .next()
        .unwrap_or(inner)
        .trim()
        .trim_matches('"');
    if route.is_empty() {
        None
    } else {
        Some(route.to_string())
    }
}

/// 文件中第一个 `class XxxController` 的 `Xxx`（[controller] 占位补全）。
fn controller_name(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(at) = trimmed.find("class ") {
            let rest = trimmed[at + 6..].trim();
            let name = rest
                .split(|c: char| !c.is_ascii_alphanumeric())
                .next()
                .unwrap_or("");
            if let Some(base) = name.strip_suffix("Controller") {
                return Some(base.to_string());
            }
            return Some(name.to_string());
        }
    }
    None
}

/// Express `app.get("/x"` / `router.delete("/y"`。
fn parse_express(line: &str) -> Option<(HttpMethod, String)> {
    for method in [
        HttpMethod::Get,
        HttpMethod::Post,
        HttpMethod::Put,
        HttpMethod::Patch,
        HttpMethod::Delete,
    ] {
        let verb = method.as_str().to_ascii_lowercase();
        for receiver in ["app.", "router.", "server."] {
            let prefix = format!("{receiver}{verb}(\"");
            if line.trim_start().starts_with(&prefix) {
                let rest = line.trim_start()[prefix.len()..].to_string();
                let route = rest.split('"').next().unwrap_or_default().to_string();
                return Some((method, route));
            }
        }
    }
    None
}

/// FastAPI `@app.get("/x")` / `@router.post("/y")`。
fn parse_fastapi(line: &str) -> Option<(HttpMethod, String)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('@') {
        return None;
    }
    for method in [
        HttpMethod::Get,
        HttpMethod::Post,
        HttpMethod::Put,
        HttpMethod::Patch,
        HttpMethod::Delete,
    ] {
        let verb = method.as_str().to_ascii_lowercase();
        let marker = format!(".{verb}(\"");
        if let Some(at) = trimmed.find(&marker) {
            let rest = &trimmed[at + marker.len()..];
            let route = rest.split('"').next().unwrap_or_default().to_string();
            return Some((method, route));
        }
    }
    None
}

/// 便捷：目录相对路径构造（M213 §73 project isolation）。
pub fn input_from_pairs(pairs: &[(String, String)]) -> DiscoverInput<'_> {
    DiscoverInput { files: pairs }
}

/// 路径判断便捷（供调用方过滤 .cs/.js/.py）。
pub fn is_discoverable_source(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".cs")
        || lower.ends_with(".js")
        || lower.ends_with(".ts")
        || lower.ends_with(".py")
}

/// 未使用的路径参数占位（防 dead-code 误报；Path 类型保留给 Tauri 层）。
pub fn _path_marker(path: &Path) -> bool {
    path.exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aspnet_attributes_discovered_with_controller_prefix() {
        let content = "[ApiController]\n[Route(\"api/[controller]\")]\npublic class CardsController : ControllerBase {\n[HttpGet]\npublic IActionResult List() => Ok();\n[HttpPost(\"{id}/draw\")]\npublic IActionResult Draw(int id) => Ok();\n}\n";
        let endpoints = discover_endpoints(&DiscoverInput {
            files: &[(
                "Controllers/CardsController.cs".to_string(),
                content.to_string(),
            )],
        });
        // [Route] 展开为 GET api/cards；[HttpGet] 空 = 同模板；[HttpPost("{id}/draw")]。
        let routes: Vec<(&str, &str)> = endpoints
            .iter()
            .map(|e| (e.method.as_str(), e.route_template.as_str()))
            .collect();
        // [controller] => 类名去 Controller 后缀（保留大小写，ASP.NET 语义）。
        assert!(routes.contains(&("GET", "/api/Cards")), "routes={routes:?}");
        assert!(routes.contains(&("POST", "/api/Cards/{id}/draw")));
        assert!(endpoints
            .iter()
            .all(|e| e.source == EndpointSource::SourceCode));
        assert!(endpoints[0].evidence.contains("CardsController.cs:"));
    }

    #[test]
    fn express_and_fastapi_routes_discovered() {
        let files = vec![
            (
                "server.js".to_string(),
                "app.get(\"/api/cards\", handler);\napp.post(\"/api/cards\", h2);\n".to_string(),
            ),
            (
                "main.py".to_string(),
                "@app.get(\"/api/decks\")\n".to_string(),
            ),
        ];
        let endpoints = discover_endpoints(&DiscoverInput { files: &files });
        assert_eq!(endpoints.len(), 3);
        let routes: Vec<(&str, &str)> = endpoints
            .iter()
            .map(|e| (e.method.as_str(), e.route_template.as_str()))
            .collect();
        assert!(routes.contains(&("GET", "/api/cards")));
        assert!(routes.contains(&("POST", "/api/cards")));
        assert!(routes.contains(&("GET", "/api/decks")));
    }

    #[test]
    fn route_parameters_normalized_for_stable_identity() {
        // §7：/api/cards/123 与 /api/cards/{id} 同一身份。
        let normalized = normalize_route("/api/cards/{id}");
        assert_eq!(normalized, "/api/cards/{id}");
        let normalized_colon = normalize_route("/api/cards/:id");
        assert_eq!(normalized_colon, "/api/cards/{id}");
        let normalized_angle = normalize_route("/api/cards/<id>");
        assert_eq!(normalized_angle, "/api/cards/{id}");
    }
}
