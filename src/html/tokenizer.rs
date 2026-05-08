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
                let decoded = decode_html_entities(&current_text);
                tokens.push(Token::Text(decoded));
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
        let decoded = decode_html_entities(&current_text);
        tokens.push(Token::Text(decoded));
    }

    tokens
}

/// Decode common HTML entities in text content.
fn decode_html_entities(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        if ch == '&' {
            chars.next(); // consume '&'
            let mut entity = String::new();
            let mut is_numeric = false;
            let mut is_hex = false;

            if let Some(&'#') = chars.peek() {
                chars.next(); // consume '#'
                is_numeric = true;
                if let Some(&'x') = chars.peek() {
                    chars.next(); // consume 'x'
                    is_hex = true;
                }
            }

            while let Some(&c) = chars.peek() {
                if c == ';' {
                    chars.next(); // consume ';'
                    break;
                }
                if c.is_whitespace() || c == '<' || c == '>' || c == '&' {
                    // Not a valid entity — push back what we've consumed
                    result.push('&');
                    if is_numeric {
                        result.push('#');
                        if is_hex {
                            result.push('x');
                        }
                    }
                    result.push_str(&entity);
                    result.push(c);
                    chars.next();
                    break;
                }
                entity.push(c);
                chars.next();
            }

            if let Some(decoded) = resolve_entity(&entity, is_numeric, is_hex) {
                result.push_str(&decoded);
            } else {
                // Unknown entity — keep as-is (including the & and ;)
                result.push('&');
                if is_numeric {
                    result.push('#');
                    if is_hex {
                        result.push('x');
                    }
                }
                result.push_str(&entity);
                result.push(';');
            }
        } else {
            result.push(ch);
            chars.next();
        }
    }

    result
}

fn resolve_entity(entity: &str, is_numeric: bool, is_hex: bool) -> Option<String> {
    if is_numeric {
        let code = if is_hex {
            u32::from_str_radix(entity, 16).ok()
        } else {
            entity.parse::<u32>().ok()
        };
        return code.and_then(|c| char::from_u32(c)).map(|c| c.to_string());
    }

    let ch = match entity {
        "nbsp" | "NBSP" => '\u{00A0}',
        "copy" | "COPY" => '\u{00A9}',
        "reg" | "REG" => '\u{00AE}',
        "trade" | "TRADE" => '\u{2122}',
        "lt" | "LT" => '<',
        "gt" | "GT" => '>',
        "amp" | "AMP" => '&',
        "quot" | "QUOT" => '"',
        "apos" => '\'',
        "mdash" => '\u{2014}',
        "ndash" => '\u{2013}',
        "hellip" => '\u{2026}',
        "laquo" => '\u{00AB}',
        "raquo" => '\u{00BB}',
        "ldquo" => '\u{201C}',
        "rdquo" => '\u{201D}',
        "lsquo" => '\u{2018}',
        "rsquo" => '\u{2019}',
        "euro" => '\u{20AC}',
        "yen" => '\u{00A5}',
        "pound" => '\u{00A3}',
        "cent" => '\u{00A2}',
        "sect" => '\u{00A7}',
        "para" => '\u{00B6}',
        "middot" => '\u{00B7}',
        "bull" => '\u{2022}',
        "ensp" => '\u{2002}',
        "emsp" => '\u{2003}',
        "thinsp" => '\u{2009}',
        "zwj" => '\u{200D}',
        "zwnj" => '\u{200C}',
        "shy" => '\u{00AD}',
        "crarr" => '\u{21B5}',
        "uarr" => '\u{2191}',
        "darr" => '\u{2193}',
        "larr" => '\u{2190}',
        "rarr" => '\u{2192}',
        "harr" => '\u{2194}',
        "times" => '\u{00D7}',
        "divide" => '\u{00F7}',
        "plusmn" => '\u{00B1}',
        "sup2" => '\u{00B2}',
        "sup3" => '\u{00B3}',
        "frac14" => '\u{00BC}',
        "frac12" => '\u{00BD}',
        "frac34" => '\u{00BE}',
        "deg" => '\u{00B0}',
        "micro" => '\u{00B5}',
        "not" => '\u{00AC}',
        "macr" => '\u{00AF}',
        "acute" => '\u{00B4}',
        "cedil" => '\u{00B8}',
        "ordf" => '\u{00AA}',
        "ordm" => '\u{00BA}',
        "iquest" => '\u{00BF}',
        "iexcl" => '\u{00A1}',
        "brvbar" => '\u{00A6}',
        "uml" => '\u{00A8}',
        "oslash" => '\u{00F8}',
        "oelig" => '\u{0153}',
        "scaron" => '\u{0161}',
        "Yuml" => '\u{0178}',
        "fnof" => '\u{0192}',
        "Alpha" => '\u{0391}',
        "Beta" => '\u{0392}',
        "Gamma" => '\u{0393}',
        "Delta" => '\u{0394}',
        "Epsilon" => '\u{0395}',
        "Zeta" => '\u{0396}',
        "Eta" => '\u{0397}',
        "Theta" => '\u{0398}',
        "Iota" => '\u{0399}',
        "Kappa" => '\u{039A}',
        "Lambda" => '\u{039B}',
        "Mu" => '\u{039C}',
        "Nu" => '\u{039D}',
        "Xi" => '\u{039E}',
        "Omicron" => '\u{039F}',
        "Pi" => '\u{03A0}',
        "Rho" => '\u{03A1}',
        "Sigma" => '\u{03A3}',
        "Tau" => '\u{03A4}',
        "Upsilon" => '\u{03A5}',
        "Phi" => '\u{03A6}',
        "Chi" => '\u{03A7}',
        "Psi" => '\u{03A8}',
        "Omega" => '\u{03A9}',
        "alpha" => '\u{03B1}',
        "beta" => '\u{03B2}',
        "gamma" => '\u{03B3}',
        "delta" => '\u{03B4}',
        "epsilon" => '\u{03B5}',
        "zeta" => '\u{03B6}',
        "eta" => '\u{03B7}',
        "theta" => '\u{03B8}',
        "iota" => '\u{03B9}',
        "kappa" => '\u{03BA}',
        "lambda" => '\u{03BB}',
        "mu" => '\u{03BC}',
        "nu" => '\u{03BD}',
        "xi" => '\u{03BE}',
        "omicron" => '\u{03BF}',
        "pi" => '\u{03C0}',
        "rho" => '\u{03C1}',
        "sigmaf" => '\u{03C2}',
        "sigma" => '\u{03C3}',
        "tau" => '\u{03C4}',
        "upsilon" => '\u{03C5}',
        "phi" => '\u{03C6}',
        "chi" => '\u{03C7}',
        "psi" => '\u{03C8}',
        "omega" => '\u{03C9}',
        "thetasym" => '\u{03D1}',
        "upsih" => '\u{03D2}',
        "piv" => '\u{03D6}',
        "forall" => '\u{2200}',
        "part" => '\u{2202}',
        "exist" => '\u{2203}',
        "empty" => '\u{2205}',
        "nabla" => '\u{2207}',
        "isin" => '\u{2208}',
        "notin" => '\u{2209}',
        "ni" => '\u{220B}',
        "prod" => '\u{220F}',
        "sum" => '\u{2211}',
        "minus" => '\u{2212}',
        "lowast" => '\u{2217}',
        "radic" => '\u{221A}',
        "prop" => '\u{221D}',
        "infin" => '\u{221E}',
        "ang" => '\u{2220}',
        "and" => '\u{2227}',
        "or" => '\u{2228}',
        "cap" => '\u{2229}',
        "cup" => '\u{222A}',
        "int" => '\u{222B}',
        "there4" => '\u{2234}',
        "sim" => '\u{223C}',
        "cong" => '\u{2245}',
        "asymp" => '\u{2248}',
        "ne" => '\u{2260}',
        "equiv" => '\u{2261}',
        "le" => '\u{2264}',
        "ge" => '\u{2265}',
        "sub" => '\u{2282}',
        "sup" => '\u{2283}',
        "nsub" => '\u{2284}',
        "sube" => '\u{2286}',
        "supe" => '\u{2287}',
        "oplus" => '\u{2295}',
        "otimes" => '\u{2297}',
        "perp" => '\u{22A5}',
        "sdot" => '\u{22C5}',
        "lceil" => '\u{2308}',
        "rceil" => '\u{2309}',
        "lfloor" => '\u{230A}',
        "rfloor" => '\u{230B}',
        "lang" => '\u{2329}',
        "rang" => '\u{232A}',
        "loz" => '\u{25CA}',
        "spades" => '\u{2660}',
        "clubs" => '\u{2663}',
        "hearts" => '\u{2665}',
        "diams" => '\u{2666}',
        "OElig" => '\u{0152}',
        "Scaron" => '\u{0160}',
        _ => return None,
    };
    Some(ch.to_string())
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
