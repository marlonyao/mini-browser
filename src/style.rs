use std::collections::HashMap;

use crate::dom::{Element, Node};
use crate::css::parser::{parse_css, Rule, Stylesheet};
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
input[type="hidden"] {
    display: none;
}

input {
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
    style_tree_with_parents(root, stylesheet, &[], &HashMap::new())
}

/// Extract CSS rules from inline `<style>` tags in the DOM.
pub fn extract_inline_styles(root: &Node) -> Vec<Rule> {
    let mut rules = Vec::new();
    match root {
        Node::Element(el) => {
            if el.tag == "style" {
                // Collect text content from all child text nodes
                let mut css_text = String::new();
                for child in &el.children {
                    if let Node::Text(text) = child {
                        css_text.push_str(text);
                    }
                }
                if !css_text.is_empty() {
                    let sheet = crate::css::parser::parse_css(&css_text);
                    rules.extend(sheet.rules);
                }
            } else {
                for child in &el.children {
                    rules.extend(extract_inline_styles(child));
                }
            }
        }
        _ => {}
    }
    rules
}

/// Expand common CSS shorthand properties into their longhand equivalents.
/// Returns a Vec of (property, value) pairs.
fn expand_inline_shorthand(prop: &str, val: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    match prop {
        "margin" | "padding" => {
            let parts: Vec<&str> = val.split_whitespace().collect();
            match parts.len() {
                1 => {
                    result.push((format!("{}-top", prop), parts[0].to_string()));
                    result.push((format!("{}-right", prop), parts[0].to_string()));
                    result.push((format!("{}-bottom", prop), parts[0].to_string()));
                    result.push((format!("{}-left", prop), parts[0].to_string()));
                }
                2 => {
                    result.push((format!("{}-top", prop), parts[0].to_string()));
                    result.push((format!("{}-right", prop), parts[1].to_string()));
                    result.push((format!("{}-bottom", prop), parts[0].to_string()));
                    result.push((format!("{}-left", prop), parts[1].to_string()));
                }
                3 => {
                    result.push((format!("{}-top", prop), parts[0].to_string()));
                    result.push((format!("{}-right", prop), parts[1].to_string()));
                    result.push((format!("{}-bottom", prop), parts[2].to_string()));
                    result.push((format!("{}-left", prop), parts[1].to_string()));
                }
                4 => {
                    result.push((format!("{}-top", prop), parts[0].to_string()));
                    result.push((format!("{}-right", prop), parts[1].to_string()));
                    result.push((format!("{}-bottom", prop), parts[2].to_string()));
                    result.push((format!("{}-left", prop), parts[3].to_string()));
                }
                _ => {
                    // Fallback: use original
                    result.push((prop.to_string(), val.to_string()));
                }
            }
        }
        "border" => {
            result.push(("border-top".to_string(), val.to_string()));
            result.push(("border-right".to_string(), val.to_string()));
            result.push(("border-bottom".to_string(), val.to_string()));
            result.push(("border-left".to_string(), val.to_string()));
        }
        "border-width" => {
            let parts: Vec<&str> = val.split_whitespace().collect();
            match parts.len() {
                1 => {
                    result.push(("border-top-width".to_string(), parts[0].to_string()));
                    result.push(("border-right-width".to_string(), parts[0].to_string()));
                    result.push(("border-bottom-width".to_string(), parts[0].to_string()));
                    result.push(("border-left-width".to_string(), parts[0].to_string()));
                }
                2 => {
                    result.push(("border-top-width".to_string(), parts[0].to_string()));
                    result.push(("border-right-width".to_string(), parts[1].to_string()));
                    result.push(("border-bottom-width".to_string(), parts[0].to_string()));
                    result.push(("border-left-width".to_string(), parts[1].to_string()));
                }
                3 => {
                    result.push(("border-top-width".to_string(), parts[0].to_string()));
                    result.push(("border-right-width".to_string(), parts[1].to_string()));
                    result.push(("border-bottom-width".to_string(), parts[2].to_string()));
                    result.push(("border-left-width".to_string(), parts[1].to_string()));
                }
                4 => {
                    result.push(("border-top-width".to_string(), parts[0].to_string()));
                    result.push(("border-right-width".to_string(), parts[1].to_string()));
                    result.push(("border-bottom-width".to_string(), parts[2].to_string()));
                    result.push(("border-left-width".to_string(), parts[3].to_string()));
                }
                _ => {
                    result.push((prop.to_string(), val.to_string()));
                }
            }
        }
        "border-color" => {
            let parts: Vec<&str> = val.split_whitespace().collect();
            match parts.len() {
                1 => {
                    result.push(("border-top-color".to_string(), parts[0].to_string()));
                    result.push(("border-right-color".to_string(), parts[0].to_string()));
                    result.push(("border-bottom-color".to_string(), parts[0].to_string()));
                    result.push(("border-left-color".to_string(), parts[0].to_string()));
                }
                2 => {
                    result.push(("border-top-color".to_string(), parts[0].to_string()));
                    result.push(("border-right-color".to_string(), parts[1].to_string()));
                    result.push(("border-bottom-color".to_string(), parts[0].to_string()));
                    result.push(("border-left-color".to_string(), parts[1].to_string()));
                }
                3 => {
                    result.push(("border-top-color".to_string(), parts[0].to_string()));
                    result.push(("border-right-color".to_string(), parts[1].to_string()));
                    result.push(("border-bottom-color".to_string(), parts[2].to_string()));
                    result.push(("border-left-color".to_string(), parts[1].to_string()));
                }
                4 => {
                    result.push(("border-top-color".to_string(), parts[0].to_string()));
                    result.push(("border-right-color".to_string(), parts[1].to_string()));
                    result.push(("border-bottom-color".to_string(), parts[2].to_string()));
                    result.push(("border-left-color".to_string(), parts[3].to_string()));
                }
                _ => {
                    result.push((prop.to_string(), val.to_string()));
                }
            }
        }
        "border-style" => {
            let parts: Vec<&str> = val.split_whitespace().collect();
            match parts.len() {
                1 => {
                    result.push(("border-top-style".to_string(), parts[0].to_string()));
                    result.push(("border-right-style".to_string(), parts[0].to_string()));
                    result.push(("border-bottom-style".to_string(), parts[0].to_string()));
                    result.push(("border-left-style".to_string(), parts[0].to_string()));
                }
                2 => {
                    result.push(("border-top-style".to_string(), parts[0].to_string()));
                    result.push(("border-right-style".to_string(), parts[1].to_string()));
                    result.push(("border-bottom-style".to_string(), parts[0].to_string()));
                    result.push(("border-left-style".to_string(), parts[1].to_string()));
                }
                3 => {
                    result.push(("border-top-style".to_string(), parts[0].to_string()));
                    result.push(("border-right-style".to_string(), parts[1].to_string()));
                    result.push(("border-bottom-style".to_string(), parts[2].to_string()));
                    result.push(("border-left-style".to_string(), parts[1].to_string()));
                }
                4 => {
                    result.push(("border-top-style".to_string(), parts[0].to_string()));
                    result.push(("border-right-style".to_string(), parts[1].to_string()));
                    result.push(("border-bottom-style".to_string(), parts[2].to_string()));
                    result.push(("border-left-style".to_string(), parts[3].to_string()));
                }
                _ => {
                    result.push((prop.to_string(), val.to_string()));
                }
            }
        }
        "background" => {
            // Simplified: keep as-is for now
            result.push((prop.to_string(), val.to_string()));
        }
        "font" => {
            // Simplified: keep as-is for now
            result.push((prop.to_string(), val.to_string()));
        }
        "flex" => {
            let parts: Vec<&str> = val.split_whitespace().collect();
            match parts.len() {
                1 => {
                    if parts[0] == "none" {
                        result.push(("flex-grow".to_string(), "0".to_string()));
                        result.push(("flex-shrink".to_string(), "0".to_string()));
                        result.push(("flex-basis".to_string(), "auto".to_string()));
                    } else {
                        result.push(("flex-grow".to_string(), parts[0].to_string()));
                        result.push(("flex-shrink".to_string(), "1".to_string()));
                        result.push(("flex-basis".to_string(), "0%".to_string()));
                    }
                }
                2 => {
                    result.push(("flex-grow".to_string(), parts[0].to_string()));
                    result.push(("flex-shrink".to_string(), parts[1].to_string()));
                    result.push(("flex-basis".to_string(), "0%".to_string()));
                }
                3 => {
                    result.push(("flex-grow".to_string(), parts[0].to_string()));
                    result.push(("flex-shrink".to_string(), parts[1].to_string()));
                    result.push(("flex-basis".to_string(), parts[2].to_string()));
                }
                _ => {
                    result.push((prop.to_string(), val.to_string()));
                }
            }
        }
        _ => {
            // Not a shorthand we handle, pass through
            result.push((prop.to_string(), val.to_string()));
        }
    }
    result
}

fn selector_matches(selector: &Selector, element: &Element, parents: &[&Element]) -> bool {
    match selector {
        Selector::Simple(simple) => matches(simple, element),
        Selector::Descendant(chain) => {
            if chain.is_empty() {
                return false;
            }
            let last = chain.len() - 1;
            if !matches(&chain[last], element) {
                return false;
            }
            // Walk up parent chain, matching earlier selectors
            let mut parent_idx = parents.len();
            for sel_idx in (0..last).rev() {
                let mut found = false;
                while parent_idx > 0 {
                    parent_idx -= 1;
                    if matches(&chain[sel_idx], parents[parent_idx]) {
                        found = true;
                        break;
                    }
                }
                if !found {
                    return false;
                }
            }
            true
        }
    }
}

fn style_tree_with_parents(root: &Node, stylesheet: &Stylesheet, parents: &[&Element], inherited_values: &HashMap<String, String>) -> StyledNode {
    match root {
        Node::Element(element) => {
            let mut specified_values = HashMap::new();
            let mut matched = Vec::new();

            for rule in &stylesheet.rules {
                for selector in &rule.selectors {
                    if selector_matches(selector, element, parents) {
                        matched.push((selector.specificity(), &rule.declarations));
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

            // Inline style has highest specificity (overrides everything)
            if let Some(inline_style) = element.attrs.get("style") {
                for decl in inline_style.split(';') {
                    let decl = decl.trim();
                    if decl.is_empty() { continue; }
                    if let Some((prop, val)) = decl.split_once(':') {
                        let prop = prop.trim();
                        let val = val.trim();
                        // Expand shorthand properties for inline styles
                        let expanded = expand_inline_shorthand(prop, val);
                        for (p, v) in expanded {
                            specified_values.insert(p, v);
                        }
                    }
                }
            }

            // Inherit values from parent for properties that should inherit
            let inherited_properties = [
                "font-size", "color", "font-family", "font-weight", "font-style",
                "line-height", "text-align", "text-decoration", "letter-spacing",
                "word-spacing", "white-space", "visibility",
            ];
            for prop in &inherited_properties {
                if !specified_values.contains_key(*prop) {
                    if let Some(val) = inherited_values.get(*prop) {
                        specified_values.insert(prop.to_string(), val.clone());
                    }
                }
            }

            // Build new inherited values map for children
            let mut child_inherited = inherited_values.clone();
            for prop in &inherited_properties {
                if let Some(val) = specified_values.get(*prop) {
                    child_inherited.insert(prop.to_string(), val.clone());
                }
            }

            // Build parent chain for children
            let mut child_parents: Vec<&Element> = parents.to_vec();
            child_parents.push(element);

            let children: Vec<StyledNode> = element
                .children
                .iter()
                .map(|child| style_tree_with_parents(child, stylesheet, &child_parents, &child_inherited))
                .collect();

            StyledNode {
                node: root.clone(),
                specified_values,
                children,
            }
        }
        Node::Text(_) => StyledNode {
            node: root.clone(),
            specified_values: inherited_values.clone(),
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
        // Text nodes now inherit values from parent
        assert_eq!(styled.children[0].specified_values.get("color"), Some(&"black".to_string()));
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
