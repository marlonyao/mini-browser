use mini_browser::html::parser::parse_html;
use mini_browser::css::parser::{parse_css, Stylesheet};
use mini_browser::style::style_tree;
use mini_browser::dom::Node;
use mini_browser::layout::{build_layout_tree, layout, Dimensions, Rect};
use mini_browser::paint::build_display_list;

fn collect_styles(node: &Node, stylesheet: &mut Stylesheet) {
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
        }
        for child in &element.children {
            collect_styles(child, stylesheet);
        }
    }
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or("/tmp/test-page.html".to_string());
    let html = std::fs::read_to_string(&path).unwrap();
    let dom = parse_html(&html);

    let mut stylesheet = Stylesheet { rules: Vec::new() };
    collect_styles(&dom, &mut stylesheet);

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
}
