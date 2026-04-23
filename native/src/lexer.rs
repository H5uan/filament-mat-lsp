use crate::token::{Token, TokenType};
use std::iter::Peekable;
use std::str::Chars;

pub struct MaterialLexer<'a> {
    input: &'a str,
    chars: Peekable<Chars<'a>>,
    line: u32,
    column: u32,
}

impl<'a> MaterialLexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.chars().peekable(),
            line: 1,
            column: 1,
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while let Some(&ch) = self.chars.peek() {
            match ch {
                ' ' | '\t' => {
                    self.chars.next();
                    self.column += 1;
                }
                '\n' => {
                    self.chars.next();
                    self.line += 1;
                    self.column = 1;
                }
                '\r' => {
                    self.chars.next();
                    if let Some('\n') = self.chars.peek() {
                        self.chars.next();
                    }
                    self.line += 1;
                    self.column = 1;
                }
                '/' => {
                    self.skip_comment();
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    if let Some(token) = self.read_keyword() {
                        tokens.push(token);
                    }
                }
                '{' => {
                    let token = Token::new(TokenType::LCurly, "{", self.line, self.column);
                    self.chars.next();
                    self.column += 1;
                    tokens.push(token);
                }
                '}' => {
                    let token = Token::new(TokenType::RCurly, "}", self.line, self.column);
                    self.chars.next();
                    self.column += 1;
                    tokens.push(token);
                }
                _ => {
                    self.chars.next();
                    self.column += 1;
                }
            }
        }
        tokens
    }

    fn skip_comment(&mut self) {
        self.chars.next();
        self.column += 1;
        if let Some(&'/') = self.chars.peek() {
            self.chars.next();
            self.column += 1;
            while let Some(&ch) = self.chars.peek() {
                if ch == '\n' {
                    break;
                }
                self.chars.next();
                self.column += 1;
            }
        } else if let Some(&'*') = self.chars.peek() {
            self.chars.next();
            self.column += 1;
            while let Some(&ch) = self.chars.peek() {
                if ch == '*' {
                    self.chars.next();
                    self.column += 1;
                    if let Some(&'/') = self.chars.peek() {
                        self.chars.next();
                        self.column += 1;
                        break;
                    }
                } else if ch == '\n' {
                    self.chars.next();
                    self.line += 1;
                    self.column = 1;
                } else {
                    self.chars.next();
                    self.column += 1;
                }
            }
        }
    }

    fn read_keyword(&mut self) -> Option<Token> {
        let start_column = self.column;
        let mut ident = String::new();
        while let Some(&ch) = self.chars.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.chars.next();
                self.column += 1;
            } else {
                break;
            }
        }
        match ident.as_str() {
            "material" => Some(Token::new(TokenType::Material, "material", self.line, start_column)),
            "vertex" => Some(Token::new(TokenType::Vertex, "vertex", self.line, start_column)),
            "fragment" => Some(Token::new(TokenType::Fragment, "fragment", self.line, start_column)),
            "compute" => Some(Token::new(TokenType::Compute, "compute", self.line, start_column)),
            _ => None,
        }
    }
}

pub struct JsonishLexer<'a> {
    input: &'a str,
    chars: Peekable<Chars<'a>>,
    line: u32,
    column: u32,
}

impl<'a> JsonishLexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.chars().peekable(),
            line: 1,
            column: 1,
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while let Some(&ch) = self.chars.peek() {
            match ch {
                ' ' | '\t' => {
                    self.chars.next();
                    self.column += 1;
                }
                '\n' => {
                    self.chars.next();
                    self.line += 1;
                    self.column = 1;
                }
                '\r' => {
                    self.chars.next();
                    if let Some('\n') = self.chars.peek() {
                        self.chars.next();
                    }
                    self.line += 1;
                    self.column = 1;
                }
                '/' => {
                    tokens.push(self.read_comment());
                }
                '"' => {
                    tokens.push(self.read_string());
                }
                '0'..='9' | '-' | '.' => {
                    tokens.push(self.read_number());
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    tokens.push(self.read_identifier());
                }
                '{' => {
                    let token = Token::new(TokenType::LCurly, "{", self.line, self.column);
                    self.chars.next();
                    self.column += 1;
                    tokens.push(token);
                }
                '}' => {
                    let token = Token::new(TokenType::RCurly, "}", self.line, self.column);
                    self.chars.next();
                    self.column += 1;
                    tokens.push(token);
                }
                '[' => {
                    let token = Token::new(TokenType::LBracket, "[", self.line, self.column);
                    self.chars.next();
                    self.column += 1;
                    tokens.push(token);
                }
                ']' => {
                    let token = Token::new(TokenType::RBracket, "]", self.line, self.column);
                    self.chars.next();
                    self.column += 1;
                    tokens.push(token);
                }
                ':' => {
                    let token = Token::new(TokenType::Colon, ":", self.line, self.column);
                    self.chars.next();
                    self.column += 1;
                    tokens.push(token);
                }
                ',' => {
                    let token = Token::new(TokenType::Comma, ",", self.line, self.column);
                    self.chars.next();
                    self.column += 1;
                    tokens.push(token);
                }
                _ => {
                    let token = Token::new(TokenType::Unknown, &ch.to_string(), self.line, self.column);
                    self.chars.next();
                    self.column += 1;
                    tokens.push(token);
                }
            }
        }
        tokens
    }

    fn read_comment(&mut self) -> Token {
        let start_column = self.column;
        self.chars.next();
        self.column += 1;
        if let Some(&'/') = self.chars.peek() {
            let mut comment = String::from("//");
            self.chars.next();
            self.column += 1;
            while let Some(&ch) = self.chars.peek() {
                if ch == '\n' {
                    break;
                }
                comment.push(ch);
                self.chars.next();
                self.column += 1;
            }
            Token::new(TokenType::Comment, &comment, self.line, start_column)
        } else if let Some(&'*') = self.chars.peek() {
            let mut comment = String::from("/*");
            self.chars.next();
            self.column += 1;
            while let Some(&ch) = self.chars.peek() {
                comment.push(ch);
                self.chars.next();
                if ch == '\n' {
                    self.line += 1;
                    self.column = 1;
                } else {
                    self.column += 1;
                }
                if ch == '*' && self.chars.peek() == Some(&'/') {
                    comment.push('/');
                    self.chars.next();
                    self.column += 1;
                    break;
                }
            }
            Token::new(TokenType::Comment, &comment, self.line, start_column)
        } else {
            Token::new(TokenType::Unknown, "/", self.line, start_column)
        }
    }

    fn read_string(&mut self) -> Token {
        let start_column = self.column;
        self.chars.next();
        self.column += 1;
        let mut s = String::from("\"");
        while let Some(&ch) = self.chars.peek() {
            if ch == '"' {
                s.push('"');
                self.chars.next();
                self.column += 1;
                break;
            } else if ch == '\\' {
                s.push('\\');
                self.chars.next();
                self.column += 1;
                if let Some(&escaped) = self.chars.peek() {
                    s.push(escaped);
                    self.chars.next();
                    self.column += 1;
                }
            } else {
                if ch == '\n' {
                    self.line += 1;
                    self.column = 1;
                } else {
                    self.column += 1;
                }
                s.push(ch);
                self.chars.next();
            }
        }
        Token::new(TokenType::String, &s, self.line, start_column)
    }

    fn read_number(&mut self) -> Token {
        let start_column = self.column;
        let mut num = String::new();
        while let Some(&ch) = self.chars.peek() {
            if ch.is_ascii_digit() || ch == '.' || ch == '-' || ch == 'e' || ch == 'E' || ch == '+' {
                num.push(ch);
                self.chars.next();
                self.column += 1;
            } else {
                break;
            }
        }
        Token::new(TokenType::Number, &num, self.line, start_column)
    }

    fn read_identifier(&mut self) -> Token {
        let start_column = self.column;
        let mut ident = String::new();
        while let Some(&ch) = self.chars.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.chars.next();
                self.column += 1;
            } else {
                break;
            }
        }
        let token_type = match ident.as_str() {
            "name" => TokenType::Name,
            "shadingModel" => TokenType::ShadingModel,
            "requires" => TokenType::Requires,
            "parameters" => TokenType::Parameters,
            "type" => TokenType::Type,
            "true" => TokenType::True,
            "false" => TokenType::False,
            "lit" => TokenType::Lit,
            "unlit" => TokenType::Unlit,
            "float" => TokenType::Float,
            "sampler2d" => TokenType::Sampler2d,
            _ => TokenType::Identifier,
        };
        Token::new(token_type, &ident, self.line, start_column)
    }
}
