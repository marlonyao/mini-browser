use mini_browser::html::parser::parse_html;
use mini_browser::css::parser::{parse_css, Stylesheet};
use mini_browser::style::style_tree;
use mini_browser::dom::Node;
use mini_browser::layout::{build_layout_tree, layout, Dimensions, Rect};
use mini_browser::paint::build_display_list;

fn collect_styles(node: &Node, stylesheet: &mut Stylesheet, base_url: &str) {
    if let Node::Element(element) = node {
        if element.tag == "style" {
            let mut css_text = String::new();
            for child in &element.children {
                if let Node::Text(text) = child {
                    css_text.push_str(text);
                }
            }
            let parsed = parse_css(&css_text);
            stylesheet.rules.extend(parsed.rules);
        } else if element.tag == "link" {
            if let Some(rel) = element.attrs.get("rel") {
                if rel == "stylesheet" {
                    if let Some(href) = element.attrs.get("href") {
                        let css_url = if href.starts_with('/') {
                            format!("file://{}", href)
                        } else if href.starts_with("file://") || href.starts_with("http://") || href.starts_with("https://") {
                            href.clone()
                        } else {
                            format!("file://{}/{}", std::env::current_dir().unwrap_or_default().display(), href)
                        };
                        match fetch_css(&css_url) {
                            Ok(css_text) => {
                                let parsed = parse_css(&css_text);
                                stylesheet.rules.extend(parsed.rules);
                            }
                            Err(e) => {
                                eprintln!("Failed to load stylesheet {}: {}", css_url, e);
                            }
                        }
                    }
                }
            }
        }
        for child in &element.children {
            collect_styles(child, stylesheet, base_url);
        }
    }
}

fn fetch_css(url: &str) -> Result<String, String> {
    if url.starts_with("file://") {
        let path = &url[7..];
        std::fs::read_to_string(path).map_err(|e| e.to_string())
    } else if url.starts_with("http://") || url.starts_with("https://") {
        match mini_browser::network::url::Url::parse(url) {
            Ok(parsed) => mini_browser::network::fetch(&parsed).map_err(|e| e.to_string()),
            Err(e) => Err(e.to_string()),
        }
    } else {
        std::fs::read_to_string(url).map_err(|e| e.to_string())
    }
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or("/tmp/test-page.html".to_string());
    let html = std::fs::read_to_string(&path).unwrap();
    let dom = parse_html(&html);

    let mut stylesheet = Stylesheet { rules: Vec::new() };
    collect_styles(&dom, &mut stylesheet, &format!("file://{}", path));

    // Merge UA default styles (lowest specificity)
    mini_browser::style::merge_default_styles(&mut stylesheet);

    let styled = style_tree(&dom, &stylesheet);
    let mut layout_root = build_layout_tree(&styled);
    let viewport = Dimensions {
        content: Rect { x: 0.0, y: 0.0, width: 800.0, height: 600.0 },
        ..Dimensions::default()
    };
    layout(&mut layout_root, viewport);

    let display_list = build_display_list(&layout_root);

    println!("Display list ({} commands):", display_list.len());
    for cmd in &display_list {
        println!("  {:?}", cmd);
    }

    // Also write JSON for screenshot rendering
    let json_path = std::env::args().nth(2).unwrap_or_else(|| "/tmp/display_list.json".to_string());
    let json = serde_json::to_string_pretty(&display_list).unwrap();
    std::fs::write(&json_path, json).unwrap();
    println!("JSON written to {}", json_path);
}
