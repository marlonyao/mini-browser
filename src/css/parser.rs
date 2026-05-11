use crate::css::tokenizer::{CssToken, tokenize};
use crate::css::selector::{Selector, SimpleSelector};

#[derive(Debug, PartialEq, Clone)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Declaration {
    pub property: String,
    pub value: String,
}

pub fn parse_css(input: &str) -> Stylesheet {
    let tokens = tokenize(input);
    parse_stylesheet(&tokens)
}

fn parse_stylesheet(tokens: &[CssToken]) -> Stylesheet {
    let mut rules = Vec::new();
    let mut pos = 0;

    while pos < tokens.len() {
        if let Some((rule, new_pos)) = parse_rule(tokens, pos) {
            rules.push(rule);
            pos = new_pos;
        } else {
            pos += 1;
        }
    }

    Stylesheet { rules }
}

fn parse_rule(tokens: &[CssToken], pos: usize) -> Option<(Rule, usize)> {
    let (selectors, mut pos) = parse_selectors(tokens, pos)?;
    if pos >= tokens.len() || tokens[pos] != CssToken::LBrace {
        return None;
    }
    pos += 1; // skip {

    let (declarations, new_pos) = parse_declarations(tokens, pos)?;
    pos = new_pos;

    Some((Rule { selectors, declarations }, pos))
}

fn parse_selectors(tokens: &[CssToken], pos: usize) -> Option<(Vec<Selector>, usize)> {
    let mut selectors = Vec::new();
    let mut pos = pos;

    loop {
        // Skip leading whitespace
        while pos < tokens.len() && tokens[pos] == CssToken::Whitespace {
            pos += 1;
        }
        if pos >= tokens.len() {
            return None;
        }

        // Parse a chain of simple selectors (descendant combinator via space)
        let mut chain = Vec::new();
        let (first, new_pos) = parse_simple_selector(tokens, pos)?;
        chain.push(first);
        pos = new_pos;

        // Keep parsing additional simple selectors if whitespace separates them
        loop {
            if pos >= tokens.len() {
                break;
            }
            // Whitespace might indicate descendant combinator
            if tokens[pos] == CssToken::Whitespace {
                pos += 1;
                if pos >= tokens.len() {
                    break;
                }
                // If the next token can start a new simple selector, it's a descendant
                if let Some((next_sel, next_pos)) = parse_simple_selector(tokens, pos) {
                    chain.push(next_sel);
                    pos = next_pos;
                    continue;
                } else {
                    break;
                }
            }
            // Comma or LBrace means the chain ends
            if tokens[pos] == CssToken::Comma || tokens[pos] == CssToken::LBrace {
                break;
            }
            // Non-whitespace, non-comma, non-lbrace without a parseable simple selector = end
            break;
        }

        if chain.len() == 1 {
            selectors.push(Selector::Simple(chain.into_iter().next().unwrap()));
        } else {
            selectors.push(Selector::Descendant(chain));
        }

        if pos < tokens.len() && tokens[pos] == CssToken::Comma {
            pos += 1; // skip comma
            continue;
        }

        break;
    }

    Some((selectors, pos))
}

fn parse_simple_selector(tokens: &[CssToken], pos: usize) -> Option<(SimpleSelector, usize)> {
    let mut selector = SimpleSelector {
        tag: None,
        id: None,
        classes: Vec::new(),
    };
    let mut pos = pos;

    while pos < tokens.len() {
        match &tokens[pos] {
            CssToken::Ident(name) => {
                if selector.tag.is_none() {
                    selector.tag = Some(name.clone());
                } else {
                    break;
                }
                pos += 1;
            }
            CssToken::Hash(id) => {
                selector.id = Some(id.clone());
                pos += 1;
            }
            CssToken::DotClass(class) => {
                selector.classes.push(class.clone());
                pos += 1;
            }
            CssToken::Colon => {
                // Skip pseudo-class or pseudo-element (:hover, ::after, :nth-child(3n+1))
                pos += 1;
                // Skip optional second colon for ::pseudo-element
                if pos < tokens.len() && tokens[pos] == CssToken::Colon {
                    pos += 1;
                }
                // Skip the pseudo-class name (Ident)
                if pos < tokens.len() && matches!(tokens[pos], CssToken::Ident(_)) {
                    pos += 1;
                }
                // Skip parentheses: :nth-child(3n+1)
                if pos < tokens.len() && tokens[pos] == CssToken::LParen {
                    pos += 1;
                    while pos < tokens.len() && tokens[pos] != CssToken::RParen {
                        pos += 1;
                    }
                    if pos < tokens.len() {
                        pos += 1; // skip )
                    }
                }
            }
            _ => break,
        }
    }

    if selector.tag.is_none() && selector.id.is_none() && selector.classes.is_empty() {
        return None;
    }

    Some((selector, pos))
}

fn parse_declarations(tokens: &[CssToken], pos: usize) -> Option<(Vec<Declaration>, usize)> {
    let mut declarations = Vec::new();
    let mut pos = pos;

    while pos < tokens.len() && tokens[pos] != CssToken::RBrace {
        // Skip leading whitespace
        while pos < tokens.len() && tokens[pos] == CssToken::Whitespace {
            pos += 1;
        }
        if pos >= tokens.len() || tokens[pos] == CssToken::RBrace {
            break;
        }
        if let Some((decl, new_pos)) = parse_declaration(tokens, pos) {
            // Expand shorthand properties
            let expanded = expand_shorthand(decl);
            declarations.extend(expanded);
            pos = new_pos;
        } else {
            // Skip unknown tokens until semicolon or RBrace
            while pos < tokens.len() && tokens[pos] != CssToken::Semicolon && tokens[pos] != CssToken::RBrace {
                pos += 1;
            }
            if pos < tokens.len() && tokens[pos] == CssToken::Semicolon {
                pos += 1;
            }
        }
    }

    if pos < tokens.len() && tokens[pos] == CssToken::RBrace {
        pos += 1; // skip }
    }

    Some((declarations, pos))
}

/// Expand CSS shorthand properties like `padding: 10px` into individual properties.
fn expand_shorthand(decl: Declaration) -> Vec<Declaration> {
    let parts: Vec<&str> = decl.value.split_whitespace().collect();
    match decl.property.as_str() {
        "padding" => expand_box("padding", &parts),
        "margin" => expand_box("margin", &parts),
        "border" => expand_border_shorthand(&parts),
        "border-width" => expand_box("border", &parts).into_iter().map(|d| {
            Declaration { property: format!("{}-width", d.property), value: d.value }
        }).collect(),
        "border-color" => expand_box("border", &parts).into_iter().map(|d| {
            Declaration { property: format!("{}-color", d.property), value: d.value }
        }).collect(),
        "flex" => {
            // flex: none | [ <flex-grow> <flex-shrink>? || <flex-basis> ]
            // Single value: flex-grow (shrink=1, basis=0%)
            // Two values: grow shrink (basis=0%)
            // Three values: grow shrink basis
            match parts.len() {
                1 => {
                    if parts[0] == "none" {
                        vec![
                            Declaration { property: "flex-grow".to_string(), value: "0".to_string() },
                            Declaration { property: "flex-shrink".to_string(), value: "0".to_string() },
                            Declaration { property: "flex-basis".to_string(), value: "auto".to_string() },
                        ]
                    } else {
                        vec![
                            Declaration { property: "flex-grow".to_string(), value: parts[0].to_string() },
                            Declaration { property: "flex-shrink".to_string(), value: "1".to_string() },
                            Declaration { property: "flex-basis".to_string(), value: "0%".to_string() },
                        ]
                    }
                }
                2 => vec![
                    Declaration { property: "flex-grow".to_string(), value: parts[0].to_string() },
                    Declaration { property: "flex-shrink".to_string(), value: parts[1].to_string() },
                    Declaration { property: "flex-basis".to_string(), value: "0%".to_string() },
                ],
                3 => vec![
                    Declaration { property: "flex-grow".to_string(), value: parts[0].to_string() },
                    Declaration { property: "flex-shrink".to_string(), value: parts[1].to_string() },
                    Declaration { property: "flex-basis".to_string(), value: parts[2].to_string() },
                ],
                _ => vec![decl],
            }
        }
        _ => vec![decl],
    }
}

/// Expand 1-4 value box shorthand (top right bottom left)
/// 1 value: all sides
/// 2 values: top/bottom, left/right  
/// 3 values: top, left/right, bottom
/// 4 values: top, right, bottom, left
fn expand_box(prefix: &str, parts: &[&str]) -> Vec<Declaration> {
    match parts.len() {
        1 => vec![
            Declaration { property: format!("{}-top", prefix), value: parts[0].to_string() },
            Declaration { property: format!("{}-right", prefix), value: parts[0].to_string() },
            Declaration { property: format!("{}-bottom", prefix), value: parts[0].to_string() },
            Declaration { property: format!("{}-left", prefix), value: parts[0].to_string() },
        ],
        2 => vec![
            Declaration { property: format!("{}-top", prefix), value: parts[0].to_string() },
            Declaration { property: format!("{}-right", prefix), value: parts[1].to_string() },
            Declaration { property: format!("{}-bottom", prefix), value: parts[0].to_string() },
            Declaration { property: format!("{}-left", prefix), value: parts[1].to_string() },
        ],
        3 => vec![
            Declaration { property: format!("{}-top", prefix), value: parts[0].to_string() },
            Declaration { property: format!("{}-right", prefix), value: parts[1].to_string() },
            Declaration { property: format!("{}-bottom", prefix), value: parts[2].to_string() },
            Declaration { property: format!("{}-left", prefix), value: parts[1].to_string() },
        ],
        4 => vec![
            Declaration { property: format!("{}-top", prefix), value: parts[0].to_string() },
            Declaration { property: format!("{}-right", prefix), value: parts[1].to_string() },
            Declaration { property: format!("{}-bottom", prefix), value: parts[2].to_string() },
            Declaration { property: format!("{}-left", prefix), value: parts[3].to_string() },
        ],
        _ => vec![],
    }
}

/// Expand `border: 2px solid black` → border-width + border-color
fn expand_border_shorthand(parts: &[&str]) -> Vec<Declaration> {
    let mut width = None;
    let mut color = None;
    for part in parts {
        if part.ends_with("px") || part.parse::<f32>().is_ok() {
            width = Some(*part);
        } else if part != &"solid" && part != &"dashed" && part != &"dotted" && part != &"none" {
            color = Some(*part);
        }
    }
    let mut result = Vec::new();
    if let Some(w) = width {
        result.push(Declaration { property: "border-width".to_string(), value: w.to_string() });
    }
    if let Some(c) = color {
        result.push(Declaration { property: "border-color".to_string(), value: c.to_string() });
    }
    result
}

fn parse_declaration(tokens: &[CssToken], pos: usize) -> Option<(Declaration, usize)> {
    if pos >= tokens.len() {
        return None;
    }

    let property = match &tokens[pos] {
        CssToken::Ident(name) => name.clone(),
        _ => return None,
    };
    let mut pos = pos + 1;

    // Skip whitespace before colon
    while pos < tokens.len() && tokens[pos] == CssToken::Whitespace {
        pos += 1;
    }

    if pos >= tokens.len() || tokens[pos] != CssToken::Colon {
        return None;
    }
    pos += 1; // skip :

    // Skip whitespace after colon
    while pos < tokens.len() && tokens[pos] == CssToken::Whitespace {
        pos += 1;
    }

    let (value, new_pos) = parse_value(tokens, pos)?;
    pos = new_pos;

    if pos < tokens.len() && tokens[pos] == CssToken::Semicolon {
        pos += 1;
    }

    Some((Declaration { property, value }, pos))
}

fn parse_value(tokens: &[CssToken], pos: usize) -> Option<(String, usize)> {
    let mut value = String::new();
    let mut pos = pos;

    while pos < tokens.len() {
        match &tokens[pos] {
            CssToken::Semicolon | CssToken::RBrace => break,
            CssToken::Whitespace => {
                pos += 1;
            }
            CssToken::Ident(s) => {
                if !value.is_empty() {
                    value.push(' ');
                }
                value.push_str(s);
                pos += 1;
            }
            CssToken::String(s) => {
                if !value.is_empty() {
                    value.push(' ');
                }
                value.push_str(s);
                pos += 1;
            }
            CssToken::Number(n) => {
                if !value.is_empty() {
                    value.push(' ');
                }
                value.push_str(&format!("{}", n));
                pos += 1;
            }
            CssToken::Unit(n, u) => {
                if !value.is_empty() {
                    value.push(' ');
                }
                value.push_str(&format!("{}{}", n, u));
                pos += 1;
            }
            CssToken::Hash(hash) => {
                if !value.is_empty() {
                    value.push(' ');
                }
                value.push('#');
                value.push_str(hash);
                pos += 1;
            }
            _ => {
                pos += 1;
            }
        }
    }

    if value.is_empty() {
        return None;
    }

    Some((value, pos))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_rule() {
        let sheet = parse_css("div { color: red; }");
        assert_eq!(sheet.rules.len(), 1);
        assert_eq!(sheet.rules[0].selectors.len(), 1);
        assert_eq!(sheet.rules[0].declarations.len(), 1);
        assert_eq!(sheet.rules[0].declarations[0].property, "color");
        assert_eq!(sheet.rules[0].declarations[0].value, "red");
    }

    #[test]
    fn test_parse_multiple_selectors() {
        let sheet = parse_css("h1, h2, h3 { color: blue; }");
        assert_eq!(sheet.rules.len(), 1);
        assert_eq!(sheet.rules[0].selectors.len(), 3);
        assert_eq!(sheet.rules[0].declarations.len(), 1);
        assert_eq!(sheet.rules[0].declarations[0].property, "color");
        assert_eq!(sheet.rules[0].declarations[0].value, "blue");
    }

    #[test]
    fn test_parse_multiple_declarations() {
        let sheet = parse_css("p { color: red; font-size: 16px; margin: 10px; }");
        assert_eq!(sheet.rules.len(), 1);
        // margin shorthand expands to 4 directional declarations
        assert_eq!(sheet.rules[0].declarations.len(), 6);
        assert_eq!(sheet.rules[0].declarations[0].property, "color");
        assert_eq!(sheet.rules[0].declarations[0].value, "red");
        assert_eq!(sheet.rules[0].declarations[1].property, "font-size");
        assert_eq!(sheet.rules[0].declarations[1].value, "16px");
        assert_eq!(sheet.rules[0].declarations[2].property, "margin-top");
        assert_eq!(sheet.rules[0].declarations[2].value, "10px");
    }

    #[test]
    fn test_parse_multiple_rules() {
        let sheet = parse_css("div { color: red; } p { font-size: 12px; }");
        assert_eq!(sheet.rules.len(), 2);
        assert_eq!(sheet.rules[0].declarations[0].property, "color");
        assert_eq!(sheet.rules[0].declarations[0].value, "red");
        assert_eq!(sheet.rules[1].declarations[0].property, "font-size");
        assert_eq!(sheet.rules[1].declarations[0].value, "12px");
    }

    #[test]
    fn test_parse_class_and_id_selector() {
        let sheet = parse_css("#main.container { color: red; }");
        assert_eq!(sheet.rules.len(), 1);
        match &sheet.rules[0].selectors[0] {
            Selector::Simple(s) => {
                assert_eq!(s.id, Some("main".to_string()));
                assert_eq!(s.classes, vec!["container".to_string()]);
            }
            _ => panic!("Expected Simple selector"),
        }
    }

    #[test]
    fn test_parse_combined_selector() {
        let sheet = parse_css("div#main.container { color: red; }");
        assert_eq!(sheet.rules.len(), 1);
        match &sheet.rules[0].selectors[0] {
            Selector::Simple(s) => {
                assert_eq!(s.tag, Some("div".to_string()));
                assert_eq!(s.id, Some("main".to_string()));
                assert_eq!(s.classes, vec!["container".to_string()]);
            }
            _ => panic!("Expected Simple selector"),
        }
    }

    #[test]
    fn test_parse_empty_body() {
        let sheet = parse_css(".main { }");
        assert_eq!(sheet.rules.len(), 1);
        assert!(sheet.rules[0].declarations.is_empty());
    }
}
