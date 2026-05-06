use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub enum Token {
    StartTag {
        tag: String,
        attrs: HashMap<String, String>,
        self_closing: bool,
    },
    EndTag {
        tag: String,
    },
    Text(String),
}

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    let mut current_text = String::new();

    while let Some(&ch) = chars.peek() {
        if ch == '<' {
            if !current_text.is_empty() {
                tokens.push(Token::Text(current_text.clone()));
                current_text.clear();
            }
            chars.next(); // consume '<'

            // Skip comments and doctype
            if chars.peek() == Some(&'!') {
                while let Some(c) = chars.next() {
                    if c == '>' {
                        break;
                    }
                }
                continue;
            }

            let is_end_tag = chars.peek() == Some(&'/');
            if is_end_tag {
                chars.next(); // consume '/'
            }

            let mut tag_name = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() || c == '>' || c == '/' {
                    break;
                }
                tag_name.push(c);
                chars.next();
            }

            // Skip whitespace after tag name
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    chars.next();
                } else {
                    break;
                }
            }

            let mut attrs = HashMap::new();
            let mut self_closing = false;

            if !is_end_tag {
                loop {
                    // Skip whitespace
                    while let Some(&c) = chars.peek() {
                        if c.is_whitespace() {
                            chars.next();
                        } else {
                            break;
                        }
                    }

                    if chars.peek() == Some(&'>') {
                        chars.next();
                        break;
                    }

                    if chars.peek() == Some(&'/') {
                        chars.next();
                        if chars.peek() == Some(&'>') {
                            chars.next();
                            self_closing = true;
                            break;
                        } else {
                            self_closing = true;
                            while let Some(c) = chars.next() {
                                if c == '>' {
                                    break;
                                }
                            }
                            break;
                        }
                    }

                    // Read attribute name
                    let mut attr_name = String::new();
                    while let Some(&c) = chars.peek() {
                        if c.is_whitespace() || c == '=' || c == '>' || c == '/' {
                            break;
                        }
                        attr_name.push(c);
                        chars.next();
                    }

                    if attr_name.is_empty() {
                        if chars.peek() == Some(&'>') {
                            chars.next();
                            break;
                        }
                        if chars.peek() == Some(&'/') {
                            chars.next();
                            if chars.peek() == Some(&'>') {
                                chars.next();
                                self_closing = true;
                                break;
                            }
                        }
                        chars.next();
                        continue;
                    }

                    // Skip whitespace
                    while let Some(&c) = chars.peek() {
                        if c.is_whitespace() {
                            chars.next();
                        } else {
                            break;
                        }
                    }

                    let mut attr_value = String::new();
                    if chars.peek() == Some(&'=') {
                        chars.next(); // consume '='

                        // Skip whitespace
                        while let Some(&c) = chars.peek() {
                            if c.is_whitespace() {
                                chars.next();
                            } else {
                                break;
                            }
                        }

                        let quote = chars.peek().copied();
                        if quote == Some('"') || quote == Some('\'') {
                            chars.next(); // consume quote
                            while let Some(c) = chars.next() {
                                if c == quote.unwrap() {
                                    break;
                                }
                                attr_value.push(c);
                            }
                        } else {
                            while let Some(&c) = chars.peek() {
                                if c.is_whitespace() || c == '>' || c == '/' {
                                    break;
                                }
                                attr_value.push(c);
                                chars.next();
                            }
                        }
                    }

                    attrs.insert(attr_name, attr_value);
                }
            } else {
                // End tag - skip until >
                while let Some(c) = chars.next() {
                    if c == '>' {
                        break;
                    }
                }
            }

            if is_end_tag {
                tokens.push(Token::EndTag { tag: tag_name });
            } else {
                tokens.push(Token::StartTag {
                    tag: tag_name,
                    attrs,
                    self_closing,
                });
            }
        } else {
            current_text.push(ch);
            chars.next();
        }
    }

    if !current_text.is_empty() {
        tokens.push(Token::Text(current_text));
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_text() {
        let tokens = tokenize("Hello");
        assert_eq!(tokens, vec![Token::Text("Hello".to_string())]);
    }

    #[test]
    fn test_tokenize_open_tag() {
        let tokens = tokenize("<div>");
        assert_eq!(tokens.len(), 1);
        match &tokens[0] {
            Token::StartTag { tag, attrs, self_closing } => {
                assert_eq!(tag, "div");
                assert!(attrs.is_empty());
                assert!(!self_closing);
            }
            _ => panic!("Expected start tag"),
        }
    }

    #[test]
    fn test_tokenize_tag_with_attr() {
        let tokens = tokenize("<div class=\"foo\">");
        assert_eq!(tokens.len(), 1);
        match &tokens[0] {
            Token::StartTag { tag, attrs, .. } => {
                assert_eq!(tag, "div");
                assert_eq!(attrs.get("class"), Some(&"foo".to_string()));
            }
            _ => panic!("Expected start tag"),
        }
    }

    #[test]
    fn test_tokenize_self_closing() {
        let tokens = tokenize("<br />");
        assert_eq!(tokens.len(), 1);
        match &tokens[0] {
            Token::StartTag { tag, self_closing, .. } => {
                assert_eq!(tag, "br");
                assert!(self_closing);
            }
            _ => panic!("Expected start tag"),
        }
    }

    #[test]
    fn test_tokenize_end_tag() {
        let tokens = tokenize("</div>");
        assert_eq!(tokens, vec![Token::EndTag { tag: "div".to_string() }]);
    }

    #[test]
    fn test_tokenize_mixed() {
        let tokens = tokenize("<p>Hello <b>world</b></p>");
        assert_eq!(tokens.len(), 6);
        match &tokens[0] { Token::StartTag { tag, .. } => assert_eq!(tag, "p"), _ => panic!() }
        match &tokens[1] { Token::Text(t) => assert_eq!(t, "Hello "), _ => panic!() }
        match &tokens[2] { Token::StartTag { tag, .. } => assert_eq!(tag, "b"), _ => panic!() }
        match &tokens[3] { Token::Text(t) => assert_eq!(t, "world"), _ => panic!() }
        match &tokens[4] { Token::EndTag { tag } => assert_eq!(tag, "b"), _ => panic!() }
        match &tokens[5] { Token::EndTag { tag } => assert_eq!(tag, "p"), _ => panic!() }
    }

    #[test]
    fn test_tokenize_with_id() {
        let tokens = tokenize("<div id=\"main\">");
        match &tokens[0] {
            Token::StartTag { tag, attrs, .. } => {
                assert_eq!(tag, "div");
                assert_eq!(attrs.get("id"), Some(&"main".to_string()));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_tokenize_multiple_attrs() {
        let tokens = tokenize("<a href=\"url\" class=\"link\">");
        match &tokens[0] {
            Token::StartTag { tag, attrs, .. } => {
                assert_eq!(tag, "a");
                assert_eq!(attrs.get("href"), Some(&"url".to_string()));
                assert_eq!(attrs.get("class"), Some(&"link".to_string()));
            }
            _ => panic!(),
        }
    }
}
