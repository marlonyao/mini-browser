/// JS 词法分析器 — 将源码解析为 Token 流

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Number(f64),
    String(String),
    Ident(String),
    Boolean(bool),
    Null,
    Undefined,

    // Arithmetic operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,

    // Assignment operators
    Assign,
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,

    // Comparison operators
    Eq,
    NotEq,
    StrictEq,
    StrictNotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,

    // Logical operators
    And,
    Or,
    Not,

    // Punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semicolon,
    Comma,
    Dot,
    Colon,
    Arrow,

    // Keywords
    Let,
    Const,
    Var,
    Function,
    Return,
    If,
    Else,
    While,
    For,
    Break,
    Continue,
    New,
    This,
    Typeof,

    // Special
    Eof,
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Token::Number(n) => write!(f, "{}", n),
            Token::String(s) => write!(f, "\"{}\"", s),
            Token::Ident(s) => write!(f, "{}", s),
            Token::Boolean(b) => write!(f, "{}", b),
            Token::Null => write!(f, "null"),
            Token::Undefined => write!(f, "undefined"),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            Token::Assign => write!(f, "="),
            Token::PlusAssign => write!(f, "+="),
            Token::MinusAssign => write!(f, "-="),
            Token::StarAssign => write!(f, "*="),
            Token::SlashAssign => write!(f, "/="),
            Token::Eq => write!(f, "=="),
            Token:: NotEq => write!(f, "!="),
            Token::StrictEq => write!(f, "==="),
            Token::StrictNotEq => write!(f, "!=="),
            Token::Lt => write!(f, "<"),
            Token::Gt => write!(f, ">"),
            Token::LtEq => write!(f, "<="),
            Token::GtEq => write!(f, ">="),
            Token::And => write!(f, "&&"),
            Token::Or => write!(f, "||"),
            Token::Not => write!(f, "!"),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::Semicolon => write!(f, ";"),
            Token::Comma => write!(f, ","),
            Token::Dot => write!(f, "."),
            Token::Colon => write!(f, ":"),
            Token::Arrow => write!(f, "=>"),
            Token::Let => write!(f, "let"),
            Token::Const => write!(f, "const"),
            Token::Var => write!(f, "var"),
            Token::Function => write!(f, "function"),
            Token::Return => write!(f, "return"),
            Token::If => write!(f, "if"),
            Token::Else => write!(f, "else"),
            Token::While => write!(f, "while"),
            Token::For => write!(f, "for"),
            Token::Break => write!(f, "break"),
            Token::Continue => write!(f, "continue"),
            Token::New => write!(f, "new"),
            Token::This => write!(f, "this"),
            Token::Typeof => write!(f, "typeof"),
            Token::Eof => write!(f, "EOF"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Lexer {
    input: Vec<char>,
    pos: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<&char> {
        self.input.get(self.pos)
    }

    fn peek_at(&self, offset: usize) -> Option<&char> {
        self.input.get(self.pos + offset)
    }

    fn advance(&mut self) -> Option<&char> {
        let ch = self.input.get(self.pos);
        self.pos += 1;
        ch
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            // Skip whitespace
            while let Some(&ch) = self.peek() {
                if ch.is_whitespace() {
                    self.advance();
                } else {
                    break;
                }
            }
            // Skip single-line comment
            if self.peek() == Some(&'/') && self.peek_at(1) == Some(&'/') {
                while let Some(&ch) = self.peek() {
                    self.advance();
                    if ch == '\n' {
                        break;
                    }
                }
                continue;
            }
            // Skip multi-line comment
            if self.peek() == Some(&'/') && self.peek_at(1) == Some(&'*') {
                self.advance(); // /
                self.advance(); // *
                loop {
                    match self.peek() {
                        Some(&'*') if self.peek_at(1) == Some(&'/') => {
                            self.advance();
                            self.advance();
                            break;
                        }
                        Some(_) => {
                            self.advance();
                        }
                        None => break,
                    }
                }
                continue;
            }
            break;
        }
    }

    fn read_string(&mut self, quote: char) -> Token {
        let mut s = String::new();
        loop {
            match self.advance() {
                Some(&'\\') => {
                    match self.advance() {
                        Some(&'n') => s.push('\n'),
                        Some(&'t') => s.push('\t'),
                        Some(&'r') => s.push('\r'),
                        Some(&'\\') => s.push('\\'),
                        Some(&c) if c == quote => s.push(quote),
                        Some(c) => s.push(*c),
                        None => break,
                    }
                }
                Some(&c) if c == quote => break,
                Some(&c) => s.push(*c),
                None => break,
            }
        }
        Token::String(s)
    }

    fn read_number(&mut self) -> Token {
        let mut num = String::new();
        while let Some(&ch) = self.peek() {
            if ch.is_ascii_digit() || ch == '.' {
                num.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        Token::Number(num.parse::<f64>().unwrap())
    }

    fn read_ident(&mut self) -> Token {
        let mut ident = String::new();
        while let Some(&ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' || ch == '$' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        match ident.as_str() {
            "true" => Token::Boolean(true),
            "false" => Token::Boolean(false),
            "null" => Token::Null,
            "undefined" => Token::Undefined,
            "let" => Token::Let,
            "const" => Token::Const,
            "var" => Token::Var,
            "function" => Token::Function,
            "return" => Token::Return,
            "if" => Token::If,
            "else" => Token::Else,
            "while" => Token::While,
            "for" => Token::For,
            "break" => Token::Break,
            "continue" => Token::Continue,
            "new" => Token::New,
            "this" => Token::This,
            "typeof" => Token::Typeof,
            _ => Token::Ident(ident),
        }
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();

        let ch = match self.peek() {
            Some(&ch) => ch,
            None => return Token::Eof,
        };

        match ch {
            '"' | '\'' => {
                self.advance();
                self.read_string(ch)
            }
            '0'..='9' => self.read_number(),
            c if c.is_alphabetic() || c == '_' || c == '$' => self.read_ident(),
            '+' => {
                self.advance();
                if self.peek() == Some(&'=') {
                    self.advance();
                    Token::PlusAssign
                } else {
                    Token::Plus
                }
            }
            '-' => {
                self.advance();
                if self.peek() == Some(&'=') {
                    self.advance();
                    Token::MinusAssign
                } else {
                    Token::Minus
                }
            }
            '*' => {
                self.advance();
                if self.peek() == Some(&'=') {
                    self.advance();
                    Token::StarAssign
                } else {
                    Token::Star
                }
            }
            '/' => {
                self.advance();
                if self.peek() == Some(&'=') {
                    self.advance();
                    Token::SlashAssign
                } else {
                    Token::Slash
                }
            }
            '%' => {
                self.advance();
                Token::Percent
            }
            '=' => {
                self.advance();
                if self.peek() == Some(&'=') {
                    self.advance();
                    if self.peek() == Some(&'=') {
                        self.advance();
                        Token::StrictEq
                    } else {
                        Token::Eq
                    }
                } else if self.peek() == Some(&'>') {
                    self.advance();
                    Token::Arrow
                } else {
                    Token::Assign
                }
            }
            '!' => {
                self.advance();
                if self.peek() == Some(&'=') {
                    self.advance();
                    if self.peek() == Some(&'=') {
                        self.advance();
                        Token::StrictNotEq
                    } else {
                        Token::NotEq
                    }
                } else {
                    Token::Not
                }
            }
            '<' => {
                self.advance();
                if self.peek() == Some(&'=') {
                    self.advance();
                    Token::LtEq
                } else {
                    Token::Lt
                }
            }
            '>' => {
                self.advance();
                if self.peek() == Some(&'=') {
                    self.advance();
                    Token::GtEq
                } else {
                    Token::Gt
                }
            }
            '&' => {
                self.advance();
                if self.peek() == Some(&'&') {
                    self.advance();
                    Token::And
                } else {
                    Token::Ident("&".to_string()) // bitwise and — simplified
                }
            }
            '|' => {
                self.advance();
                if self.peek() == Some(&'|') {
                    self.advance();
                    Token::Or
                } else {
                    Token::Ident("|".to_string()) // bitwise or — simplified
                }
            }
            '(' => { self.advance(); Token::LParen }
            ')' => { self.advance(); Token::RParen }
            '{' => { self.advance(); Token::LBrace }
            '}' => { self.advance(); Token::RBrace }
            '[' => { self.advance(); Token::LBracket }
            ']' => { self.advance(); Token::RBracket }
            ';' => { self.advance(); Token::Semicolon }
            ',' => { self.advance(); Token::Comma }
            '.' => { self.advance(); Token::Dot }
            ':' => { self.advance(); Token::Colon }
            _ => {
                self.advance();
                Token::Ident(ch.to_string())
            }
        }
    }
}

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token();
        let is_eof = token == Token::Eof;
        tokens.push(token);
        if is_eof {
            break;
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_number() {
        let tokens = tokenize("42 3.14");
        assert_eq!(tokens[0], Token::Number(42.0));
        assert_eq!(tokens[1], Token::Number(3.14));
        assert_eq!(tokens[2], Token::Eof);
    }

    #[test]
    fn test_tokenize_string() {
        let tokens = tokenize(r#""hello" 'world'"#);
        assert_eq!(tokens[0], Token::String("hello".to_string()));
        assert_eq!(tokens[1], Token::String("world".to_string()));
    }

    #[test]
    fn test_tokenize_ident() {
        let tokens = tokenize("foo bar _baz $qux");
        assert_eq!(tokens[0], Token::Ident("foo".to_string()));
        assert_eq!(tokens[1], Token::Ident("bar".to_string()));
        assert_eq!(tokens[2], Token::Ident("_baz".to_string()));
        assert_eq!(tokens[3], Token::Ident("$qux".to_string()));
    }

    #[test]
    fn test_tokenize_operators() {
        let tokens = tokenize("+ - * / = == === != !==");
        assert_eq!(tokens[0], Token::Plus);
        assert_eq!(tokens[1], Token::Minus);
        assert_eq!(tokens[2], Token::Star);
        assert_eq!(tokens[3], Token::Slash);
        assert_eq!(tokens[4], Token::Assign);
        assert_eq!(tokens[5], Token::Eq);
        assert_eq!(tokens[6], Token::StrictEq);
        assert_eq!(tokens[7], Token::NotEq);
        assert_eq!(tokens[8], Token::StrictNotEq);
    }

    #[test]
    fn test_tokenize_keywords() {
        let tokens = tokenize("let const var function return if else while for");
        assert_eq!(tokens[0], Token::Let);
        assert_eq!(tokens[1], Token::Const);
        assert_eq!(tokens[2], Token::Var);
        assert_eq!(tokens[3], Token::Function);
        assert_eq!(tokens[4], Token::Return);
        assert_eq!(tokens[5], Token::If);
        assert_eq!(tokens[6], Token::Else);
        assert_eq!(tokens[7], Token::While);
        assert_eq!(tokens[8], Token::For);
    }

    #[test]
    fn test_tokenize_punctuation() {
        let tokens = tokenize("( ) { } [ ] ; , . :");
        assert_eq!(tokens[0], Token::LParen);
        assert_eq!(tokens[1], Token::RParen);
        assert_eq!(tokens[2], Token::LBrace);
        assert_eq!(tokens[3], Token::RBrace);
        assert_eq!(tokens[4], Token::LBracket);
        assert_eq!(tokens[5], Token::RBracket);
        assert_eq!(tokens[6], Token::Semicolon);
        assert_eq!(tokens[7], Token::Comma);
        assert_eq!(tokens[8], Token::Dot);
        assert_eq!(tokens[9], Token::Colon);
    }

    #[test]
    fn test_tokenize_comparison() {
        let tokens = tokenize("< > <= >=");
        assert_eq!(tokens[0], Token::Lt);
        assert_eq!(tokens[1], Token::Gt);
        assert_eq!(tokens[2], Token::LtEq);
        assert_eq!(tokens[3], Token::GtEq);
    }

    #[test]
    fn test_tokenize_logical() {
        let tokens = tokenize("&& || !");
        assert_eq!(tokens[0], Token::And);
        assert_eq!(tokens[1], Token::Or);
        assert_eq!(tokens[2], Token::Not);
    }

    #[test]
    fn test_tokenize_boolean_null() {
        let tokens = tokenize("true false null undefined");
        assert_eq!(tokens[0], Token::Boolean(true));
        assert_eq!(tokens[1], Token::Boolean(false));
        assert_eq!(tokens[2], Token::Null);
        assert_eq!(tokens[3], Token::Undefined);
    }

    #[test]
    fn test_tokenize_arrow() {
        let tokens = tokenize("=>");
        assert_eq!(tokens[0], Token::Arrow);
    }

    #[test]
    fn test_tokenize_comments() {
        let tokens = tokenize("42 // comment\n43 /* block */ 44");
        assert_eq!(tokens[0], Token::Number(42.0));
        assert_eq!(tokens[1], Token::Number(43.0));
        assert_eq!(tokens[2], Token::Number(44.0));
    }

    #[test]
    fn test_tokenize_assignment_ops() {
        let tokens = tokenize("+= -= *= /=");
        assert_eq!(tokens[0], Token::PlusAssign);
        assert_eq!(tokens[1], Token::MinusAssign);
        assert_eq!(tokens[2], Token::StarAssign);
        assert_eq!(tokens[3], Token::SlashAssign);
    }

    #[test]
    fn test_tokenize_typeof() {
        let tokens = tokenize("typeof x");
        assert_eq!(tokens[0], Token::Typeof);
        assert_eq!(tokens[1], Token::Ident("x".to_string()));
    }

    #[test]
    fn test_tokenize_string_escapes() {
        let tokens = tokenize(r#""hello\nworld""#);
        assert_eq!(tokens[0], Token::String("hello\nworld".to_string()));
    }
}
