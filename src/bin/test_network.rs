use mini_browser::html::parser::parse_html;
use mini_browser::css::parser::parse_css;
use mini_browser::style::style_tree;
use mini_browser::layout::{build_layout_tree, layout, Dimensions, Rect};
use mini_browser::paint::build_display_list;
use mini_browser::network::url::Url;

fn main() {
    let urls = [
        "https://www.baidu.com",
        "http://www.baidu.com",
    ];
    for url in urls {
        eprintln!("\n========== {} ==========", url);
        match Url::parse(url) {
            Ok(parsed) => {
                match mini_browser::network::fetch(&parsed) {
                    Ok(html) => {
                        eprintln!("[test] Fetched {} bytes", html.len());
                        let dom = parse_html(&html);
                        let mut stylesheet = parse_css("");
                        mini_browser::style::merge_default_styles(&mut stylesheet);
                        let styled = style_tree(&dom, &stylesheet);
                        let mut layout_root = build_layout_tree(&styled);
                        let viewport = Dimensions {
                            content: Rect { x: 0.0, y: 0.0, width: 800.0, height: 600.0 },
                            ..Dimensions::default()
                        };
                        layout(&mut layout_root, viewport);
                        eprintln!("[test] Layout tree children: {}", layout_root.children.len());
                        let display_list = build_display_list(&layout_root);
                        eprintln!("[test] Display list: {} items", display_list.len());
                    }
                    Err(e) => eprintln!("[test] Fetch error: {}", e),
                }
            }
            Err(e) => eprintln!("[test] Parse error: {}", e),
        }
    }
}
