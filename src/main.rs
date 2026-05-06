use mini_browser::network::url::Url;
use mini_browser::network::fetch;
use mini_browser::html::parser::parse_html;
use mini_browser::css::parser::{parse_css, Stylesheet};
use mini_browser::style::{style_tree, print_style_tree};
use mini_browser::dom::Node;
use mini_browser::layout::{build_layout_tree, layout, print_layout_box, Dimensions, Rect};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: mini-browser <url>");
        std::process::exit(1);
    }

    let url = Url::parse(&args[1])?;
    println!("Fetching {}://{}{}...", url.scheme, url.host, url.path);

    let body = fetch(&url)?;
    let dom = parse_html(&body);

    // Extract inline stylesheets from <style> tags
    let mut stylesheet = Stylesheet { rules: Vec::new() };
    collect_styles(&dom, &mut stylesheet);

    // Build styled tree
    let styled = style_tree(&dom, &stylesheet);

    // Print styled tree
    println!("\nStyled Tree:");
    print_style_tree(&styled, 0);

    // Build layout tree
    let mut layout_root = build_layout_tree(&styled);

    // Layout with fixed viewport width
    let viewport = Dimensions {
        content: Rect { x: 0.0, y: 0.0, width: 800.0, height: 600.0 },
        ..Dimensions::default()
    };
    layout(&mut layout_root, viewport);

    // Print layout tree
    println!("\nLayout Tree:");
    print_layout_box(&layout_root, 0);

    Ok(())
}

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
