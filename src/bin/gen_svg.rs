use mini_browser::html::parser::parse_html;
use mini_browser::css::parser::parse_css;
use mini_browser::style::{style_tree, extract_inline_styles};
use mini_browser::layout::{build_layout_tree, layout as layout_fn, Dimensions, Rect};
use mini_browser::paint::build_display_list;
use std::fs;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} <html_file> [svg_output] [html_output]", args[0]);
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
    let html_path = args.get(3).cloned().unwrap_or_else(|| "output.html".to_string());

    // Export as SVG
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
    println!("Commands: {}", display_list.len());

    generate_html_report(&html, &html_path);
    println!("✅ {} generated", html_path);
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
}

fn generate_html_report(html_src: &str, path: &str) {
    let dom = parse_html(html_src);
    let stylesheet = parse_css("");
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

    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\"><title>Mini Browser Layout Report</title>\n");
    html.push_str("<style>\n");
    html.push_str("body { font-family: monospace; margin: 20px; }\n");
    html.push_str("table { border-collapse: collapse; margin: 20px 0; }\n");
    html.push_str("th, td { border: 1px solid #ccc; padding: 8px; text-align: left; }\n");
    html.push_str("th { background: #f0f0f0; }\n");
    html.push_str(".canvas { position: relative; width: 800px; height: 600px; border: 2px solid #333; background: white; margin: 20px 0; }\n");
    html.push_str(".box { position: absolute; box-sizing: border-box; }\n");
    html.push_str(".text { position: absolute; white-space: nowrap; overflow: hidden; }\n");
    html.push_str("</style></head><body>\n");
    html.push_str("<h1>Mini Browser Layout Report</h1>\n");
    html.push_str("<h2>Source HTML</h2>\n<pre>");
    html.push_str(&html_escape(html_src));
    html.push_str("</pre>\n");
    html.push_str("<h2>Rendered Output</h2>\n<div class=\"canvas\">\n");

    for cmd in &display_list {
        match cmd {
            mini_browser::paint::DisplayCommand::SolidColor(rect, color, _radius) => {
                html.push_str(&format!(
                    "<div class=\"box\" style=\"left:{}px;top:{}px;width:{}px;height:{}px;background:rgba({},{},{},{});\">\n",
                    rect.x, rect.y, rect.width, rect.height,
                    (color.r * 255.0) as u8,
                    (color.g * 255.0) as u8,
                    (color.b * 255.0) as u8,
                    color.a
                ));
            }
            mini_browser::paint::DisplayCommand::Text(text, rect, color) => {
                if rect.width < 1.0 || rect.height < 1.0 { continue; }
                html.push_str(&format!(
                    "<div class=\"text\" style=\"left:{}px;top:{}px;width:{}px;height:{}px;color:rgb({},{},{});font-size:{}px;display:flex;align-items:center;\">\n",
                    rect.x, rect.y, rect.width, rect.height,
                    (color.r * 255.0) as u8,
                    (color.g * 255.0) as u8,
                    (color.b * 255.0) as u8,
                    rect.height
                ));
            }
            mini_browser::paint::DisplayCommand::Border(rect, width, color, _radius) => {
                html.push_str(&format!(
                    "<div class=\"box\" style=\"left:{}px;top:{}px;width:{}px;height:{}px;border:{}px solid rgb({},{},{});\">\n",
                    rect.x, rect.y, rect.width, rect.height, width,
                    (color.r * 255.0) as u8,
                    (color.g * 255.0) as u8,
                    (color.b * 255.0) as u8
                ));
            }
            mini_browser::paint::DisplayCommand::Image(_, rect, alt) => {
                let alt_text = alt.as_ref().map(|s| s.as_str()).unwrap_or("[img]");
                html.push_str(&format!(
                    "<div class=\"box\" style=\"left:{}px;top:{}px;width:{}px;height:{}px;background:#ddd;border:1px solid #999;display:flex;align-items:center;justify-content:center;font-size:12px;color:#666;\">{}\n",
                    rect.x, rect.y, rect.width, rect.height,
                    html_escape(alt_text)
                ));
            }
        }
    }

    html.push_str("</div>\n");
    html.push_str("<h2>Display List (first 50)</h2>\n");
    html.push_str("<table><tr><th>#</th><th>Type</th><th>Rect</th><th>Details</th></tr>\n");

    for (i, cmd) in display_list.iter().take(50).enumerate() {
        let (type_name, rect, detail) = match cmd {
            mini_browser::paint::DisplayCommand::SolidColor(r, c, _) => (
                "SolidColor",
                r,
                format!("rgb({:.0},{:.0},{:.0})", c.r * 255.0, c.g * 255.0, c.b * 255.0)
            ),
            mini_browser::paint::DisplayCommand::Text(t, r, c) => (
                "Text",
                r,
                format!("'{}' rgb({:.0},{:.0},{:.0})", t.chars().take(20).collect::<String>(), c.r * 255.0, c.g * 255.0, c.b * 255.0)
            ),
            mini_browser::paint::DisplayCommand::Border(r, w, c, _) => (
                "Border",
                r,
                format!("{}px rgb({:.0},{:.0},{:.0})", w, c.r * 255.0, c.g * 255.0, c.b * 255.0)
            ),
            mini_browser::paint::DisplayCommand::Image(_, r, a) => (
                "Image",
                r,
                format!("alt='{}'", a.as_ref().map(|s| s.as_str()).unwrap_or(""))
            ),
        };
        html.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{:.1},{:.1} {:.1}x{:.1}</td><td>{}</td></tr>\n",
            i, type_name, rect.x, rect.y, rect.width, rect.height, detail
        ));
    }

    html.push_str("</table></body></html>");
    fs::write(path, html).expect("Failed to write HTML report");
}
