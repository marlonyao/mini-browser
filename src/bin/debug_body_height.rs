use mini_browser::html::parser::parse_html;
use mini_browser::css::parser::{parse_css, Stylesheet};
use mini_browser::style::{style_tree, merge_default_styles};

fn main() {
    let html = std::fs::read_to_string("baidu_real.html").unwrap();
    let css = std::fs::read_to_string("baidu.css").unwrap();
    
    let dom = parse_html(&html);
    let parsed = parse_css(&css);
    let mut stylesheet = Stylesheet { rules: parsed.rules };
    merge_default_styles(&mut stylesheet);
    
    let styled = style_tree(&dom, &stylesheet);
    
    // Find body
    for child in &styled.children {
        if let mini_browser::dom::Node::Element(el) = &child.node {
            if el.tag == "body" {
                println!("body specified_values:");
                for (k, v) in &child.specified_values {
                    println!("  {} = '{}'", k, v);
                }
                
                let height_val = child.specified_values.get("height");
                println!("\n  height value: {:?}", height_val);
                break;
            }
        }
    }
}
