use mini_browser::network::url::Url;
use mini_browser::network::fetch;
use mini_browser::html::parser::parse_html;
use mini_browser::css::parser::{parse_css, Stylesheet};
use mini_browser::style::{style_tree, print_style_tree};
use mini_browser::dom::Node;
use mini_browser::layout::{build_layout_tree, layout, print_layout_box, Dimensions, Rect};
use mini_browser::paint::{build_display_list, render_to_terminal, render_to_ppm};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let mut render_mode = false;
    let mut ppm_path = None;
    let mut url = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--render" => render_mode = true,
            "--ppm" => {
                i += 1;
                if i < args.len() {
                    ppm_path = Some(args[i].clone());
                } else {
                    eprintln!("Usage: mini-browser <url> [--render] [--ppm <file>]");
                    std::process::exit(1);
                }
            }
            _ => {
                if url.is_none() {
                    url = Some(args[i].clone());
                } else {
                    eprintln!("Usage: mini-browser <url> [--render] [--ppm <file>]");
                    std::process::exit(1);
                }
            }
        }
        i += 1;
    }

    let url = match url {
        Some(u) => Url::parse(&u)?,
        None => {
            eprintln!("Usage: mini-browser <url> [--render] [--ppm <file>]");
            std::process::exit(1);
        }
    };

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

    if render_mode {
        let display_list = build_display_list(&layout_root);
        let output = render_to_terminal(&display_list, 80, 24);
        println!("\nTerminal Render:");
        println!("{}", output);
    } else if let Some(path) = ppm_path {
        let display_list = build_display_list(&layout_root);
        let ppm = render_to_ppm(&display_list, 800, 600);
        std::fs::write(&path, ppm)?;
        println!("\nExported PPM to {}", path);
    } else {
        // Default: print layout tree
        println!("\nLayout Tree:");
        print_layout_box(&layout_root, 0);
    }

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
