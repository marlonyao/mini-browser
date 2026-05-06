use std::collections::HashMap;

use crate::dom::Node;
use crate::css::parser::Stylesheet;
use crate::css::selector::{matches, Selector};

#[derive(Debug, PartialEq)]
pub struct StyledNode {
    pub node: Node,
    pub specified_values: HashMap<String, String>,
    pub children: Vec<StyledNode>,
}

pub fn style_tree(root: &Node, stylesheet: &Stylesheet) -> StyledNode {
    match root {
        Node::Element(element) => {
            let mut specified_values = HashMap::new();
            let mut matched = Vec::new();

            for rule in &stylesheet.rules {
                for selector in &rule.selectors {
                    let Selector::Simple(simple) = selector;
                    if matches(simple, element) {
                        matched.push((simple.specificity(), &rule.declarations));
                    }
                }
            }

            // Sort by specificity (ascending) so later (higher specificity) overwrites earlier
            matched.sort_by(|a, b| a.0.cmp(&b.0));

            for (_spec, declarations) in &matched {
                for declaration in *declarations {
                    specified_values.insert(
                        declaration.property.clone(),
                        declaration.value.clone(),
                    );
                }
            }

            let children: Vec<StyledNode> = element
                .children
                .iter()
                .map(|child| style_tree(child, stylesheet))
                .collect();

            StyledNode {
                node: root.clone(),
                specified_values,
                children,
            }
        }
        Node::Text(_) => StyledNode {
            node: root.clone(),
            specified_values: HashMap::new(),
            children: vec![],
        },
    }
}

pub fn print_style_tree(node: &StyledNode, indent: usize) {
    let spaces = "  ".repeat(indent);
    match &node.node {
        Node::Element(el) => {
            let mut attrs = String::new();
            if let Some(id) = el.attrs.get("id") {
                attrs.push_str(&format!(" id=\"{}\"", id));
            }
            if let Some(class) = el.attrs.get("class") {
                attrs.push_str(&format!(" class=\"{}\"", class));
            }
            let mut style_str = String::new();
            if !node.specified_values.is_empty() {
                style_str.push_str(" style={");
                for (k, v) in &node.specified_values {
                    style_str.push_str(&format!("{}: {}; ", k, v));
                }
                style_str.push('}');
            }
            println!("{}<{}{}{}>", spaces, el.tag, attrs, style_str);
            for child in &node.children {
                print_style_tree(child, indent + 1);
            }
        }
        Node::Text(text) => {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                println!("{}{}", spaces, trimmed);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::parser::parse_css;
    use std::collections::HashMap;

    #[test]
    fn test_style_simple() {
        let dom = Node::element("div", HashMap::new(), vec![]);
        let stylesheet = parse_css("div { color: red; }");
        let styled = style_tree(&dom, &stylesheet);

        assert_eq!(styled.specified_values.get("color"), Some(&"red".to_string()));
    }

    #[test]
    fn test_style_override_by_specificity() {
        let mut attrs = HashMap::new();
        attrs.insert("class".to_string(), "box".to_string());
        let dom = Node::element("div", attrs, vec![]);
        let stylesheet = parse_css("div { color: red; } .box { color: blue; }");
        let styled = style_tree(&dom, &stylesheet);

        assert_eq!(styled.specified_values.get("color"), Some(&"blue".to_string()));
    }

    #[test]
    fn test_style_no_match() {
        let dom = Node::element("p", HashMap::new(), vec![]);
        let stylesheet = parse_css("div { color: red; }");
        let styled = style_tree(&dom, &stylesheet);

        assert!(styled.specified_values.is_empty());
    }

    #[test]
    fn test_style_nested() {
        let child = Node::element("span", HashMap::new(), vec![]);
        let parent = Node::element("div", HashMap::new(), vec![child]);
        let stylesheet = parse_css("div { color: red; } span { font-size: 12px; }");
        let styled = style_tree(&parent, &stylesheet);

        assert_eq!(styled.specified_values.get("color"), Some(&"red".to_string()));
        assert_eq!(styled.children.len(), 1);
        assert_eq!(
            styled.children[0].specified_values.get("font-size"),
            Some(&"12px".to_string())
        );
    }

    #[test]
    fn test_style_text_node() {
        let dom = Node::element("p", HashMap::new(), vec![Node::text("Hello")]);
        let stylesheet = parse_css("p { color: black; }");
        let styled = style_tree(&dom, &stylesheet);

        assert_eq!(styled.specified_values.get("color"), Some(&"black".to_string()));
        assert_eq!(styled.children.len(), 1);
        assert!(styled.children[0].specified_values.is_empty());
    }

    #[test]
    fn test_style_id_override_class() {
        let mut attrs = HashMap::new();
        attrs.insert("id".to_string(), "main".to_string());
        attrs.insert("class".to_string(), "box".to_string());
        let dom = Node::element("div", attrs, vec![]);
        let stylesheet = parse_css(".box { color: blue; } #main { color: green; }");
        let styled = style_tree(&dom, &stylesheet);

        assert_eq!(styled.specified_values.get("color"), Some(&"green".to_string()));
    }
}
