use mini_browser::html::parser::parse_html;
use mini_browser::css::parser::{parse_css, Stylesheet};
use mini_browser::style::{style_tree, merge_default_styles};
use mini_browser::layout::{build_layout_tree, layout, Dimensions, Rect};
use std::fs;

fn main() {
    let html = fs::read_to_string("baidu_real.html").unwrap();
    let mut stylesheet = Stylesheet { rules: Vec::new() };
    if let Ok(css) = fs::read_to_string("baidu.css") {
        stylesheet.rules.extend(parse_css(&css).rules);
    }
    merge_default_styles(&mut stylesheet);
    let styled = style_tree(&dom, &stylesheet);
    let mut layout_root = build_layout_tree(&styled);
    let viewport = Dimensions {
        content: Rect { x: 0.0, y: 0.0, width: 800.0, height: 600.0 },
        ..Dimensions::default()
    };
    layout(&mut layout_root, viewport);
    
    // Print body and first few children
    fn print_box(box: &mini_browser::layout::LayoutBox, depth: usize) {
        let indent = "  ".repeat(depth);
        println!("{}y={:.1} h={:.1} | {:?}", 
            indent,
            box.dimensions.content.y,
            box.dimensions.content.height,
            box.box_type
        );
        for child in &box.children {
            print_box(child, depth + 1);
        }
    }
    print_box(&layout_root, 0);
}
