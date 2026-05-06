use crate::dom::{Element, Node};
use crate::html::tokenizer::{tokenize, Token};
use std::collections::HashMap;

pub fn parse(tokens: Vec<Token>) -> Node {
    let mut tokens = tokens.into_iter().peekable();
    parse_nodes(&mut tokens)
}

fn parse_nodes(tokens: &mut std::iter::Peekable<impl Iterator<Item = Token>>) -> Node {
    let mut children = Vec::new();

    while let Some(token) = tokens.peek() {
        match token {
            Token::EndTag { .. } => break,
            _ => {
                let node = parse_node(tokens);
                children.push(node);
            }
        }
    }

    if children.len() == 1 {
        children.into_iter().next().unwrap()
    } else {
        Node::element("html", HashMap::new(), children)
    }
}

fn parse_node(tokens: &mut std::iter::Peekable<impl Iterator<Item = Token>>) -> Node {
    match tokens.next() {
        Some(Token::Text(text)) => Node::text(&text),
        Some(Token::StartTag { tag, attrs, self_closing }) => {
            if self_closing || is_void_element(&tag) {
                Node::Element(Element::new(&tag, attrs, vec![]))
            } else {
                let mut children = Vec::new();
                while let Some(token) = tokens.peek() {
                    match token {
                        Token::EndTag { tag: end_tag } if end_tag == &tag => {
                            tokens.next();
                            break;
                        }
                        _ => {
                            children.push(parse_node(tokens));
                        }
                    }
                }
                Node::Element(Element::new(&tag, attrs, children))
            }
        }
        Some(Token::EndTag { .. }) => Node::text(""),
        None => Node::text(""),
    }
}

fn is_void_element(tag: &str) -> bool {
    matches!(tag, "br" | "hr" | "img" | "input" | "meta" | "link" | "area" | "base" | "col" | "embed" | "param" | "source" | "track" | "wbr")
}

pub fn parse_html(input: &str) -> Node {
    let tokens = tokenize(input);
    parse(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_text() {
        let node = parse_html("Hello");
        match node {
            Node::Text(s) => assert_eq!(s, "Hello"),
            _ => panic!("Expected text node"),
        }
    }

    #[test]
    fn test_parse_single_element() {
        let node = parse_html("<p>Hello</p>");
        match node {
            Node::Element(el) => {
                assert_eq!(el.tag, "p");
                assert_eq!(el.children.len(), 1);
                match &el.children[0] {
                    Node::Text(s) => assert_eq!(s, "Hello"),
                    _ => panic!("Expected text child"),
                }
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_parse_nested() {
        let node = parse_html("<div><p>Hello</p></div>");
        match node {
            Node::Element(el) => {
                assert_eq!(el.tag, "div");
                assert_eq!(el.children.len(), 1);
                match &el.children[0] {
                    Node::Element(inner) => {
                        assert_eq!(inner.tag, "p");
                        assert_eq!(inner.children.len(), 1);
                    }
                    _ => panic!("Expected inner element"),
                }
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_parse_siblings() {
        let node = parse_html("<div><p>A</p><p>B</p></div>");
        match node {
            Node::Element(el) => {
                assert_eq!(el.tag, "div");
                assert_eq!(el.children.len(), 2);
                match &el.children[0] {
                    Node::Element(p1) => assert_eq!(p1.tag, "p"),
                    _ => panic!("Expected p"),
                }
                match &el.children[1] {
                    Node::Element(p2) => assert_eq!(p2.tag, "p"),
                    _ => panic!("Expected p"),
                }
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_parse_self_closing() {
        let node = parse_html("<p>Hello<br>World</p>");
        match node {
            Node::Element(el) => {
                assert_eq!(el.tag, "p");
                assert_eq!(el.children.len(), 3);
                match &el.children[1] {
                    Node::Element(br) => assert_eq!(br.tag, "br"),
                    _ => panic!("Expected br element"),
                }
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_parse_with_attrs() {
        let node = parse_html("<div id=\"main\" class=\"container\">Hello</div>");
        match node {
            Node::Element(el) => {
                assert_eq!(el.tag, "div");
                assert_eq!(el.attrs.get("id"), Some(&"main".to_string()));
                assert_eq!(el.attrs.get("class"), Some(&"container".to_string()));
                assert_eq!(el.children.len(), 1);
            }
            _ => panic!("Expected element"),
        }
    }
}
