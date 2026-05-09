use mini_browser::html::parser::parse_html;
use mini_browser::css::parser::parse_css;
use mini_browser::style::style_tree;
use mini_browser::layout::{build_layout_tree, layout as layout_fn, Dimensions, Rect};
use mini_browser::paint::build_display_list;
use std::fs;

fn render_html_to_report(name: &str, html: &str, css: &str) {
    let dom = parse_html(html);
    let stylesheet = parse_css(css);
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

    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\">");
    out.push_str(&format!("<title>{} - Mini Browser Render</title>\n", name));
    out.push_str("<style>\n");
    out.push_str("body { font-family: monospace; margin: 20px; background: #f5f5f5; }\n");
    out.push_str(".container { background: white; padding: 20px; border-radius: 8px; margin: 20px 0; }\n");
    out.push_str(".canvas { position: relative; width: 800px; height: 600px; border: 2px solid #333; background: white; margin: 20px 0; overflow: hidden; }\n");
    out.push_str(".box { position: absolute; box-sizing: border-box; }\n");
    out.push_str(".text { position: absolute; white-space: nowrap; overflow: hidden; line-height: 1; }\n");
    out.push_str("pre { background: #f0f0f0; padding: 15px; border-radius: 4px; overflow-x: auto; }\n");
    out.push_str("table { border-collapse: collapse; margin: 20px 0; width: 100%; }\n");
    out.push_str("th, td { border: 1px solid #ccc; padding: 8px; text-align: left; font-size: 12px; }\n");
    out.push_str("th { background: #e0e0e0; }\n");
    out.push_str("td:nth-child(1) { width: 40px; text-align: center; }\n");
    out.push_str("td:nth-child(2) { width: 100px; }\n");
    out.push_str("td:nth-child(3) { width: 200px; font-family: monospace; }\n");
    out.push_str("</style></head><body>\n");
    
    out.push_str(&format!("<h1>{} - Mini Browser Render</h1>\n", name));
    
    out.push_str("<div class=\"container\">\n");
    out.push_str("<h2>Rendered Output (800x600)</h2>\n");
    out.push_str("<div class=\"canvas\">\n");

    for cmd in &display_list {
        match cmd {
            mini_browser::paint::DisplayCommand::SolidColor(rect, color) => {
                out.push_str(&format!(
                    "<div class=\"box\" style=\"left:{}px;top:{}px;width:{}px;height:{}px;background:rgba({},{},{},{});\"></div>\n",
                    rect.x, rect.y, rect.width, rect.height,
                    (color.r * 255.0) as u8,
                    (color.g * 255.0) as u8,
                    (color.b * 255.0) as u8,
                    color.a
                ));
            }
            mini_browser::paint::DisplayCommand::Text(text, rect, color) => {
                if rect.width < 0.5 || rect.height < 0.5 { continue; }
                let font_size = rect.height.max(8.0).min(72.0);
                out.push_str(&format!(
                    "<div class=\"text\" style=\"left:{}px;top:{}px;width:{}px;height:{}px;color:rgba({},{},{},{});font-size:{}px;\">{}</div>\n",
                    rect.x, rect.y, rect.width + 2.0, rect.height,
                    (color.r * 255.0) as u8,
                    (color.g * 255.0) as u8,
                    (color.b * 255.0) as u8,
                    color.a,
                    font_size,
                    html_escape(text)
                ));
            }
            mini_browser::paint::DisplayCommand::Border(rect, width, color) => {
                out.push_str(&format!(
                    "<div class=\"box\" style=\"left:{}px;top:{}px;width:{}px;height:{}px;border:{}px solid rgba({},{},{},{});\"></div>\n",
                    rect.x, rect.y, rect.width, rect.height, width,
                    (color.r * 255.0) as u8,
                    (color.g * 255.0) as u8,
                    (color.b * 255.0) as u8,
                    color.a
                ));
            }
            mini_browser::paint::DisplayCommand::Image(_, rect, alt) => {
                let alt_text = alt.as_ref().map(|s| s.as_str()).unwrap_or("[img]");
                out.push_str(&format!(
                    "<div class=\"box\" style=\"left:{}px;top:{}px;width:{}px;height:{}px;background:#ddd;border:1px solid #999;display:flex;align-items:center;justify-content:center;font-size:12px;color:#666;\">{}</div>\n",
                    rect.x, rect.y, rect.width, rect.height,
                    html_escape(alt_text)
                ));
            }
        }
    }

    out.push_str("</div>\n");
    out.push_str("</div>\n");

    out.push_str("<div class=\"container\">\n");
    out.push_str("<h2>Source HTML</h2>\n<pre>");
    out.push_str(&html_escape(html));
    out.push_str("</pre>\n");
    out.push_str("</div>\n");

    out.push_str("<div class=\"container\">\n");
    out.push_str("<h2>Display List Commands</h2>\n");
    out.push_str("<table><tr><th>#</th><th>Type</th><th>Rect</th><th>Details</th></tr>\n");

    for (i, cmd) in display_list.iter().enumerate() {
        let (type_name, rect, detail) = match cmd {
            mini_browser::paint::DisplayCommand::SolidColor(r, c) => (
                "SolidColor",
                r,
                format!("rgba({:.0},{:.0},{:.0},{:.2})", c.r * 255.0, c.g * 255.0, c.b * 255.0, c.a)
            ),
            mini_browser::paint::DisplayCommand::Text(t, r, c) => (
                "Text",
                r,
                format!("'{}' rgba({:.0},{:.0},{:.0},{:.2})", &t[..t.len().min(30)], c.r * 255.0, c.g * 255.0, c.b * 255.0, c.a)
            ),
            mini_browser::paint::DisplayCommand::Border(r, w, c) => (
                "Border",
                r,
                format!("{}px rgba({:.0},{:.0},{:.0},{:.2})", w, c.r * 255.0, c.g * 255.0, c.b * 255.0, c.a)
            ),
            mini_browser::paint::DisplayCommand::Image(_, r, a) => (
                "Image",
                r,
                format!("alt='{}'", a.as_ref().map(|s| s.as_str()).unwrap_or(""))
            ),
        };
        out.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{:.1},{:.1} {:.1}x{:.1}</td><td>{}</td></tr>\n",
            i, type_name, rect.x, rect.y, rect.width, rect.height, detail
        ));
    }

    out.push_str("</table>\n");
    out.push_str("</div>\n");
    out.push_str("</body></html>");
    
    let filename = format!("render_{}.html", name.to_lowercase().replace(" ", "_"));
    fs::write(&filename, out).expect(&format!("Failed to write {}", filename));
    println!("✅ {} generated ({} commands)", filename, display_list.len());
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
}

fn main() {
    // Test 1: Block & Inline Layout
    let html1 = r#"
<!DOCTYPE html>
<html>
<head><title>Block & Inline Test</title></head>
<body style="margin: 20px; font-family: sans-serif;">
    <h1 style="color: #333; margin-bottom: 20px;">Block & Inline Layout</h1>
    <div style="background: #e0e0e0; padding: 20px; margin: 10px 0;">
        <p style="margin: 0;">Gray box with padding.</p>
    </div>
    <div style="width: 300px; height: 100px; background: #4CAF50; margin: 20px auto; color: white;">
        Fixed 300x100 block
    </div>
    <div style="display: inline-block; background: #2196F3; padding: 10px 20px; margin: 5px; color: white;">Blue</div>
    <div style="display: inline-block; background: #FF9800; padding: 10px 20px; margin: 5px; color: white;">Orange</div>
    <div style="display: inline-block; background: #9C27B0; padding: 10px 20px; margin: 5px; color: white;">Purple</div>
    <p style="margin-top: 30px;">
        <span style="background: yellow;">Highlighted</span> inline text.
    </p>
</body>
</html>
"#;
    render_html_to_report("Block Inline", html1, "");

    // Test 2: Margin Collapsing
    let html2 = r#"
<!DOCTYPE html>
<html>
<body style="margin: 0;">
    <div style="margin: 20px; background: #f0f0f0;">
        <div style="margin-top: 30px; margin-bottom: 20px; background: #e74c3c; padding: 10px; color: white;">
            Box 1: margin-top 30, margin-bottom 20
        </div>
        <div style="margin-top: 10px; margin-bottom: 40px; background: #3498db; padding: 10px; color: white;">
            Box 2: margin-top 10, margin-bottom 40
        </div>
        <div style="margin-top: 25px; background: #2ecc71; padding: 10px; color: white;">
            Box 3: margin-top 25
        </div>
    </div>
</body>
</html>
"#;
    render_html_to_report("Margin Collapsing", html2, "");

    // Test 3: Box Sizing
    let html3 = r#"
<!DOCTYPE html>
<html>
<body style="margin: 20px;">
    <div style="width: 300px; height: 100px; background: #e74c3c; padding: 20px; border: 5px solid #333; margin: 10px; color: white;">
        content-box (default): 300x100 content
    </div>
    <div style="width: 300px; height: 100px; background: #3498db; padding: 20px; border: 5px solid #333; margin: 10px; box-sizing: border-box; color: white;">
        border-box: total 300x100
    </div>
</body>
</html>
"#;
    render_html_to_report("Box Sizing", html3, "");

    // Test 4: Baidu Snapshot
    let baidu_html = fs::read_to_string("baidu.html").unwrap_or_default();
    if !baidu_html.is_empty() {
        render_html_to_report("Baidu", &baidu_html, "");
    }

    println!("\n✅ All reports generated! Open the .html files in a browser to view.");
}
