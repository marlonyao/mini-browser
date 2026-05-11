use mini_browser::html::parser::parse_html;
use mini_browser::css::parser::parse_css;
use mini_browser::style::{style_tree, extract_inline_styles};
use mini_browser::layout::{build_layout_tree, layout as layout_fn, Dimensions, Rect};
use mini_browser::paint::build_display_list;
use std::fs;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} <html_file> [svg_output]", args[0]);
        return;
    }

    let html = std::fs::read_to_string(&args[1]).expect("Failed to read HTML file");
    let dom = parse_html(&html);
    let mut stylesheet = parse_css("");
    let inline_rules = extract_inline_styles(&dom);
    stylesheet.rules.extend(inline_rules);
    let styled = style_tree(&dom, &stylesheet);
    let mut layout_box = build_layout_tree(&styled);
    let viewport = Dimensions {
        content: Rect { x: 0.0, y: 0.0, width: 800.0, height: 600.0 },
        padding: Default::default(),
        border: Default::default(),
        margin: Default::default(),
    };
    layout_fn(&mut layout_box, viewport);
    let display_list = build_display_list(&layout_box);

    let svg_path = args.get(2).cloned().unwrap_or_else(|| "output.svg".to_string());

    let mut svg = String::new();
    svg.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    svg.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"800\" height=\"600\" viewBox=\"0 0 800 600\">\n");
    svg.push_str("  <rect width=\"800\" height=\"600\" fill=\"white\"/>\n");

    for cmd in &display_list {
        match cmd {
            mini_browser::paint::DisplayCommand::SolidColor(rect, color, radius) => {
                let r = (color.r * 255.0) as u8;
                let g = (color.g * 255.0) as u8;
                let b = (color.b * 255.0) as u8;
                let a = color.a.clamp(0.0, 1.0);
                let radius_attr = if *radius > 0.0 {
                    format!(" rx=\"{:.1}\" ry=\"{:.1}\"", radius, radius)
                } else {
                    String::new()
                };
                let line = if a < 1.0 {
                    format!(
                        "  <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\"{} fill=\"rgb({},{},{})\" opacity=\"{:.2}\"/>\n",
                        rect.x, rect.y, rect.width, rect.height, radius_attr, r, g, b, a
                    )
                } else {
                    format!(
                        "  <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\"{} fill=\"rgb({},{},{})\"/>\n",
                        rect.x, rect.y, rect.width, rect.height, radius_attr, r, g, b
                    )
                };
                svg.push_str(&line);
            }
            mini_browser::paint::DisplayCommand::Text(text, rect, color) => {
                if rect.width < 1.0 || rect.height < 1.0 { continue; }
                let r = (color.r * 255.0) as u8;
                let g = (color.g * 255.0) as u8;
                let b = (color.b * 255.0) as u8;
                let font_size = rect.height.max(8.0).min(72.0);
                let line = format!(
                    "  <text x=\"{:.1}\" y=\"{:.1}\" font-size=\"{:.1}\" fill=\"rgb({},{},{})\">{}</text>\n",
                    rect.x, rect.y + font_size * 0.85, font_size, r, g, b,
                    html_escape(text)
                );
                svg.push_str(&line);
            }
            mini_browser::paint::DisplayCommand::Border(rect, width, color, radius) => {
                let r = (color.r * 255.0) as u8;
                let g = (color.g * 255.0) as u8;
                let b = (color.b * 255.0) as u8;
                let a = color.a.clamp(0.0, 1.0);
                let radius_attr = if *radius > 0.0 {
                    format!(" rx=\"{:.1}\" ry=\"{:.1}\"", radius, radius)
                } else {
                    String::new()
                };
                let line = if a < 1.0 {
                    format!(
                        "  <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\"{} fill=\"none\" stroke=\"rgb({},{},{})\" stroke-width=\"{:.1}\" opacity=\"{:.2}\"/>\n",
                        rect.x, rect.y, rect.width, rect.height, radius_attr, r, g, b, width, a
                    )
                } else {
                    format!(
                        "  <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\"{} fill=\"none\" stroke=\"rgb({},{},{})\" stroke-width=\"{:.1}\"/>\n",
                        rect.x, rect.y, rect.width, rect.height, radius_attr, r, g, b, width
                    )
                };
                svg.push_str(&line);
            }
            mini_browser::paint::DisplayCommand::Image(_, rect, _) => {
                let line1 = format!(
                    "  <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"#ddd\" stroke=\"#999\"/>\n",
                    rect.x, rect.y, rect.width, rect.height
                );
                let line2 = format!(
                    "  <text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\" fill=\"#666\">[img]</text>\n",
                    rect.x + 5.0, rect.y + rect.height / 2.0
                );
                svg.push_str(&line1);
                svg.push_str(&line2);
            }
        }
    }

    svg.push_str("</svg>\n");
    fs::write(&svg_path, svg).expect("Failed to write SVG");
    println!("✅ {} generated (800x600)", svg_path);
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
}
