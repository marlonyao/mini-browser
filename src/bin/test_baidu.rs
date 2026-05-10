use mini_browser::html::parser::parse_html;
use mini_browser::css::parser::{parse_css, Stylesheet};
use mini_browser::style::{style_tree, merge_default_styles};
use mini_browser::layout::{build_layout_tree, layout, Dimensions, Rect};
use mini_browser::paint::build_display_list;
use std::fs;

fn main() {
    let html = fs::read_to_string("baidu_real.html").expect("Failed to read baidu_real.html");
    println!("=== HTML size: {} bytes ===", html.len());

    // 1. Parse HTML
    let dom = parse_html(&html);
    println!("✅ DOM parsed");

    // 2. Load external CSS if available
    let mut stylesheet = Stylesheet { rules: Vec::new() };
    if let Ok(css) = fs::read_to_string("baidu.css") {
        let parsed = parse_css(&css);
        let rule_count = parsed.rules.len();
        stylesheet.rules.extend(parsed.rules);
        println!("✅ External CSS loaded ({} rules)", rule_count);
    } else {
        println!("⚠️  External CSS not found, using UA defaults only");
    }
    merge_default_styles(&mut stylesheet);
    println!("✅ Stylesheet ready ({} rules total)", stylesheet.rules.len());

    // 3. Build styled tree
    let styled = style_tree(&dom, &stylesheet);
    println!("✅ Styled tree built");

    // Print styled tree structure
    println!("\n=== Styled Tree Structure ===");
    mini_browser::style::print_style_tree(&styled, 0);

    // 4. Build layout tree
    let mut layout_root = build_layout_tree(&styled);
    println!("✅ Layout tree built");

    // 5. Layout
    let viewport = Dimensions {
        content: Rect { x: 0.0, y: 0.0, width: 800.0, height: 600.0 },
        ..Dimensions::default()
    };
    layout(&mut layout_root, viewport);
    println!("✅ Layout computed");

    // Print layout tree structure
    println!("\n=== Layout Tree Structure ===");
    mini_browser::layout::print_layout_box(&layout_root, 0);

    // 6. Build display list
    let display_list = build_display_list(&layout_root);
    println!("✅ Display list: {} commands", display_list.len());

    // 7. Stats
    let mut max_y = 0.0f32;
    let mut text_count = 0;
    let mut bg_count = 0;
    let mut border_count = 0;
    let mut img_count = 0;

    for cmd in &display_list {
        use mini_browser::paint::DisplayCommand::*;
        match cmd {
            SolidColor(rect, _) => { max_y = max_y.max(rect.y + rect.height); bg_count += 1; }
            Text(_, rect, _) => { max_y = max_y.max(rect.y + rect.height); text_count += 1; }
            Border(rect, _, _) => { max_y = max_y.max(rect.y + rect.height); border_count += 1; }
            Image(_, rect, _) => { max_y = max_y.max(rect.y + rect.height); img_count += 1; }
        }
    }

    println!("\n=== Display List Stats ===");
    println!("Backgrounds: {}", bg_count);
    println!("Text:        {}", text_count);
    println!("Borders:     {}", border_count);
    println!("Images:      {}", img_count);
    println!("Page height: {:.1}px", max_y);

    // Print first 30 text elements for inspection
    println!("\n=== First 30 text elements ===");
    let mut printed = 0;
    for cmd in &display_list {
        if let mini_browser::paint::DisplayCommand::Text(text, rect, _) = cmd {
            if printed < 30 && !text.trim().is_empty() {
                println!("  [{:3}] '{}' at ({:.0}, {:.0}) w={:.0} h={:.0}",
                    printed, text, rect.x, rect.y, rect.width, rect.height);
                printed += 1;
            }
        }
    }

    println!("\n=== SUCCESS ===");
}
