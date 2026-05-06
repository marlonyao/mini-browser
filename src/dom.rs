use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Element(Element),
    Text(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub tag: String,
    pub attrs: HashMap<String, String>,
    pub children: Vec<Node>,
}

impl Element {
    pub fn new(tag: &str, attrs: HashMap<String, String>, children: Vec<Node>) -> Self {
        Element {
            tag: tag.to_string(),
            attrs,
            children,
        }
    }
}

impl Node {
    pub fn text(content: &str) -> Self {
        Node::Text(content.to_string())
    }

    pub fn element(tag: &str, attrs: HashMap<String, String>, children: Vec<Node>) -> Self {
        Node::Element(Element::new(tag, attrs, children))
    }
}

fn format_node(node: &Node, f: &mut fmt::Formatter, indent: usize) -> fmt::Result {
    let spaces = "  ".repeat(indent);
    match node {
        Node::Text(text) => {
            writeln!(f, "{}{}", spaces, text)?;
        }
        Node::Element(el) => {
            writeln!(f, "{}<{}>", spaces, el.tag)?;
            for child in &el.children {
                format_node(child, f, indent + 1)?;
            }
            writeln!(f, "{}</{}>", spaces, el.tag)?;
        }
    }
    Ok(())
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        format_node(self, f, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_node() {
        let node = Node::text("Hello");
        match node {
            Node::Text(s) => assert_eq!(s, "Hello"),
            _ => panic!("Expected text node"),
        }
    }

    #[test]
    fn test_element_node() {
        let node = Node::element("div", HashMap::new(), vec![Node::text("Hello")]);
        match node {
            Node::Element(el) => {
                assert_eq!(el.tag, "div");
                assert_eq!(el.children.len(), 1);
            }
            _ => panic!("Expected element node"),
        }
    }

    #[test]
    fn test_nested_dom() {
        let inner = Node::element("span", HashMap::new(), vec![Node::text("world")]);
        let outer = Node::element("div", HashMap::new(), vec![inner]);
        match outer {
            Node::Element(el) => {
                assert_eq!(el.tag, "div");
                assert_eq!(el.children.len(), 1);
                match &el.children[0] {
                    Node::Element(inner_el) => {
                        assert_eq!(inner_el.tag, "span");
                        assert_eq!(inner_el.children.len(), 1);
                    }
                    _ => panic!("Expected inner element"),
                }
            }
            _ => panic!("Expected element node"),
        }
    }

    #[test]
    fn test_dom_display() {
        let node = Node::element("div", HashMap::new(), vec![Node::text("Hello")]);
        let s = format!("{}", node);
        assert!(s.contains("<div>"));
        assert!(s.contains("Hello"));
        assert!(s.contains("</div>"));
    }
}
