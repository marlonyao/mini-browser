use std::collections::HashMap;

use crate::dom::Node;
use crate::css::parser::{parse_css, Stylesheet};
use crate::css::selector::{matches, Selector};

#[derive(Debug, PartialEq, Clone)]
pub struct StyledNode {
    pub node: Node,
    pub specified_values: HashMap<String, String>,
    pub children: Vec<StyledNode>,
}

/// Built-in User Agent (UA) default stylesheet.
/// These are the browser defaults that apply before any author CSS.
pub const DEFAULT_CSS: &str = r#"
/* === Display === */
body, div, p, h1, h2, h3, h4, h5, h6, ul, ol, li, blockquote, pre, table, form, fieldset, header, footer, nav, section, article, aside, main, figure, figcaption, details, dialog {
    display: block;
}
span, a, em, strong, b, i, u, s, strike, del, ins, sup, sub, small, mark, abbr, code, kbd, samp, var, cite, dfn, q, time, label, img, br, wbr, input, textarea, select, button, option, optgroup, datalist, progress, meter {
    display: inline;
}
head, style, script, link, meta, title, base, noscript, template {
    display: none;
}

/* === Body === */
body {
    margin: 8px;
}

/* === Paragraphs === */
p {
    margin-top: 1em;
    margin-bottom: 1em;
}

/* === Headings === */
h1 {
    font-size: 2em;
    margin-top: 0.67em;
    margin-bottom: 0.67em;
    font-weight: bold;
}
h2 {
    font-size: 1.5em;
    margin-top: 0.75em;
    margin-bottom: 0.75em;
    font-weight: bold;
}
h3 {
    font-size: 1.17em;
    margin-top: 0.83em;
    margin-bottom: 0.83em;
    font-weight: bold;
}
h4 {
    font-size: 1em;
    margin-top: 1.12em;
    margin-bottom: 1.12em;
    font-weight: bold;
}
h5 {
    font-size: 0.83em;
    margin-top: 1.5em;
    margin-bottom: 1.5em;
    font-weight: bold;
}
h6 {
    font-size: 0.67em;
    margin-top: 1.67em;
    margin-bottom: 1.67em;
    font-weight: bold;
}

/* === Lists === */
ul, ol {
    margin-top: 1em;
    margin-bottom: 1em;
    padding-left: 40px;
}
ul {
    list-style-type: disc;
}
ol {
    list-style-type: decimal;
}
li {
    display: list-item;
}

/* === Links === */
a {
    color: #0000EE;
    text-decoration: underline;
}
a:visited {
    color: #551A8B;
}

/* === Misc === */
br {
    display: none;
}
img {
    display: inline;
}
input, textarea, select, button {
    display: inline-block;
}
blockquote {
    margin: 1em 40px;
}
pre {
    margin: 1em 0;
    white-space: pre;
}
"#;

/// Merge the UA default stylesheet rules into an existing stylesheet.
/// Default rules go first (lowest specificity), so author CSS can override.
pub fn merge_default_styles(stylesheet: &mut Stylesheet) {
    let default_sheet = parse_css(DEFAULT_CSS);
    let mut merged = default_sheet.rules;
    merged.extend(stylesheet.rules.clone());
    stylesheet.rules = merged;
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
