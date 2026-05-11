use mini_browser::html::parser::parse_html;
use mini_browser::css::parser::parse_css;
use mini_browser::style::{style_tree, merge_default_styles, StyledNode};
use mini_browser::dom::Node;
use mini_browser::layout::{build_layout_tree, layout, Dimensions, Rect};

fn find_divs(node: &StyledNode, depth: usize) {
    if let Node::Element(el) = &node.node {
        if el.tag == "div" {
            println!("{}div: display={:?}, flex={:?}, gap={:?}", 
                "  ".repeat(depth),
                node.specified_values.get("display"),
                node.specified_values.get("flex"),
                node.specified_values.get("gap"));
        }
    }
    for child in &node.children {
        find_divs(child, depth + 1);
    }
}

fn main() {
    let html = r#"
<!DOCTYPE html>
<html>
<body style="margin:0; padding:20px;">
  <div style="display:flex; gap:10px;">
    <div style="flex:1; height:80px; background:#FF5722;">A</div>
    <div style="flex:1; height:80px; background:#2196F3;">B</div>
    <div style="flex:1; height:80px; background:#4CAF50;">C</div>
  </div>
</body>
</html>
"#;

    let dom = parse_html(html);
    let mut stylesheet = parse_css("");
    merge_default_styles(&mut stylesheet);
    let styled = style_tree(&dom, &stylesheet);
    
    println!("=== Styled Tree divs ===");
    find_divs(&styled, 0);
    
    let mut layout_root = build_layout_tree(&styled);
    let viewport = Dimensions {
        content: Rect { x: 0.0, y: 0.0, width: 800.0, height: 600.0 },
        padding: Default::default(),
        border: Default::default(),
        margin: Default::default(),
    };
    layout(&mut layout_root, viewport);
    
    println!("\n=== Layout Tree ===");
    mini_browser::layout::print_layout_box(&layout_root, 0);
}
