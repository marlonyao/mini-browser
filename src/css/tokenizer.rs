#[derive(Debug, PartialEq, Clone)]
pub enum CssToken {
    Ident(String),
    Hash(String),
    DotClass(String),
    Colon,
    Semicolon,
    LBrace,
    RBrace,
    LParen,
    RParen,
    Comma,
    String(String),
    Number(f64),
    Unit(f64, String),
    Whitespace,
}

pub fn tokenize(input: &str) -> Vec<CssToken> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            // Only emit a single Whitespace token for consecutive whitespace
            tokens.push(CssToken::Whitespace);
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    chars.next();
                } else {
                    break;
                }
            }
            continue;
        }

        if ch == '/' && chars.clone().nth(1) == Some('*') {
            // Skip comment /* ... */
            chars.next(); // '/'
            chars.next(); // '*'
            while let Some(c) = chars.next() {
                if c == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    break;
                }
            }
            continue;
        }

        match ch {
            '{' => {
                chars.next();
                tokens.push(CssToken::LBrace);
            }
            '}' => {
                chars.next();
                tokens.push(CssToken::RBrace);
            }
            '(' => {
                chars.next();
                tokens.push(CssToken::LParen);
            }
            ')' => {
                chars.next();
                tokens.push(CssToken::RParen);
            }
            ';' => {
                chars.next();
                tokens.push(CssToken::Semicolon);
            }
            ':' => {
                chars.next();
                tokens.push(CssToken::Colon);
            }
            ',' => {
                chars.next();
                tokens.push(CssToken::Comma);
            }
            '"' | '\'' => {
                tokens.push(read_string(&mut chars));
            }
            '#' => {
                chars.next();
                let name = read_name(&mut chars);
                tokens.push(CssToken::Hash(name));
            }
            '.' => {
                chars.next();
                let name = read_name(&mut chars);
                tokens.push(CssToken::DotClass(name));
            }
            _ if ch.is_ascii_digit() || (ch == '.' && chars.clone().nth(1).map_or(false, |c| c.is_ascii_digit())) => {
                tokens.push(read_number(&mut chars));
            }
            _ if is_name_start(ch) => {
                let name = read_name(&mut chars);
                tokens.push(CssToken::Ident(name));
            }
            _ => {
                // Skip unknown character
                chars.next();
            }
        }
    }

    tokens
}

fn read_string(chars: &mut std::iter::Peekable<impl Iterator<Item = char>>) -> CssToken {
    let quote = chars.next().unwrap();
    let mut value = String::new();
    while let Some(c) = chars.next() {
        if c == quote {
            break;
        }
        value.push(c);
    }
    CssToken::String(value)
}

fn read_number(chars: &mut std::iter::Peekable<impl Iterator<Item = char>>) -> CssToken {
    let mut num_str = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() || c == '.' {
            num_str.push(c);
            chars.next();
        } else {
            break;
        }
    }

    let num: f64 = num_str.parse().unwrap_or(0.0);

    // Check for unit
    let mut unit = String::new();
    while let Some(&c) = chars.peek() {
        if is_name_char(c) {
            unit.push(c);
            chars.next();
        } else {
            break;
        }
    }

    if unit.is_empty() {
        CssToken::Number(num)
    } else {
        CssToken::Unit(num, unit)
    }
}

fn read_name(chars: &mut std::iter::Peekable<impl Iterator<Item = char>>) -> String {
    let mut name = String::new();
    while let Some(&c) = chars.peek() {
        if is_name_char(c) {
            name.push(c);
            chars.next();
        } else {
            break;
        }
    }
    name
}

fn is_name_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_' || ch == '-'
}

fn is_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple_rule() {
        let tokens = tokenize("div { color: red; }");
        assert_eq!(
            tokens,
            vec![
                CssToken::Ident("div".to_string()),
                CssToken::Whitespace,
                CssToken::LBrace,
                CssToken::Whitespace,
                CssToken::Ident("color".to_string()),
                CssToken::Colon,
                CssToken::Whitespace,
                CssToken::Ident("red".to_string()),
                CssToken::Semicolon,
                CssToken::Whitespace,
                CssToken::RBrace,
            ]
        );
    }

    #[test]
    fn test_tokenize_class_selector() {
        let tokens = tokenize(".main { }");
        assert_eq!(
            tokens,
            vec![
                CssToken::DotClass("main".to_string()),
                CssToken::Whitespace,
                CssToken::LBrace,
                CssToken::Whitespace,
                CssToken::RBrace,
            ]
        );
    }

    #[test]
    fn test_tokenize_id_selector() {
        let tokens = tokenize("#header { }");
        assert_eq!(
            tokens,
            vec![
                CssToken::Hash("header".to_string()),
                CssToken::Whitespace,
                CssToken::LBrace,
                CssToken::Whitespace,
                CssToken::RBrace,
            ]
        );
    }

    #[test]
    fn test_tokenize_property_values() {
        let tokens = tokenize("color: red; font-size: 16px;");
        assert_eq!(
            tokens,
            vec![
                CssToken::Ident("color".to_string()),
                CssToken::Colon,
                CssToken::Whitespace,
                CssToken::Ident("red".to_string()),
                CssToken::Semicolon,
                CssToken::Whitespace,
                CssToken::Ident("font-size".to_string()),
                CssToken::Colon,
                CssToken::Whitespace,
                CssToken::Unit(16.0, "px".to_string()),
                CssToken::Semicolon,
            ]
        );
    }

    #[test]
    fn test_tokenize_number_and_string() {
        let tokens = tokenize("content: \"hello\"; opacity: 0.5;");
        assert_eq!(
            tokens,
            vec![
                CssToken::Ident("content".to_string()),
                CssToken::Colon,
                CssToken::Whitespace,
                CssToken::String("hello".to_string()),
                CssToken::Semicolon,
                CssToken::Whitespace,
                CssToken::Ident("opacity".to_string()),
                CssToken::Colon,
                CssToken::Whitespace,
                CssToken::Number(0.5),
                CssToken::Semicolon,
            ]
        );
    }

    #[test]
    fn test_tokenize_comma_and_multiple_selectors() {
        let tokens = tokenize("h1, h2 { color: blue; }");
        assert_eq!(
            tokens,
            vec![
                CssToken::Ident("h1".to_string()),
                CssToken::Comma,
                CssToken::Whitespace,
                CssToken::Ident("h2".to_string()),
                CssToken::Whitespace,
                CssToken::LBrace,
                CssToken::Whitespace,
                CssToken::Ident("color".to_string()),
                CssToken::Colon,
                CssToken::Whitespace,
                CssToken::Ident("blue".to_string()),
                CssToken::Semicolon,
                CssToken::Whitespace,
                CssToken::RBrace,
            ]
        );
    }

    #[test]
    fn test_tokenize_comment() {
        let tokens = tokenize("/* comment */ div { }");
        assert_eq!(
            tokens,
            vec![
                CssToken::Whitespace,
                CssToken::Ident("div".to_string()),
                CssToken::Whitespace,
                CssToken::LBrace,
                CssToken::Whitespace,
                CssToken::RBrace,
            ]
        );
    }
}
