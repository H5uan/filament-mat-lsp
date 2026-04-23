use crate::token::Token;

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    let mut line = 1;
    let mut column = 1;

    while let Some(&c) = chars.peek() {
        match c {
            // Whitespace
            ' ' | '\t' => {
                chars.next();
                column += 1;
            }
            '\n' => {
                chars.next();
                line += 1;
                column = 1;
            }
            '\r' => {
                chars.next();
                if let Some('\n') = chars.peek() {
                    chars.next();
                }
                line += 1;
                column = 1;
            }
            // Comments
            '/' => {
                chars.next();
                column += 1;
                if let Some('/') = chars.peek() {
                    let start_col = column;
                    chars.next();
                    column += 1;
                    let mut comment = String::new();
                    while let Some(&c) = chars.peek() {
                        if c == '\n' {
                            break;
                        }
                        comment.push(c);
                        chars.next();
                        column += 1;
                    }
                    tokens.push(Token::new("Comment", &comment, line, start_col));
                } else if let Some('*') = chars.peek() {
                    let start_col = column;
                    chars.next();
                    column += 1;
                    let mut comment = String::new();
                    while let Some(&c) = chars.peek() {
                        if c == '*' {
                            comment.push(c);
                            chars.next();
                            column += 1;
                            if let Some('/') = chars.peek() {
                                chars.next();
                                column += 1;
                                break;
                            }
                        } else {
                            if c == '\n' {
                                line += 1;
                                column = 1;
                            } else {
                                column += 1;
                            }
                            comment.push(c);
                            chars.next();
                        }
                    }
                    tokens.push(Token::new("Comment", &comment, line, start_col));
                } else {
                    tokens.push(Token::new("Punctuation", "/", line, column));
                    column += 1;
                }
            }
            // Strings
            '"' => {
                let start_col = column;
                chars.next();
                column += 1;
                let mut s = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '"' {
                        chars.next();
                        column += 1;
                        break;
                    } else if c == '\\' {
                        chars.next();
                        column += 1;
                        if let Some(&escaped) = chars.peek() {
                            s.push(match escaped {
                                'n' => '\n',
                                't' => '\t',
                                'r' => '\r',
                                '\\' => '\\',
                                '"' => '"',
                                _ => escaped,
                            });
                            chars.next();
                            column += 1;
                        }
                    } else {
                        if c == '\n' {
                            line += 1;
                            column = 1;
                        } else {
                            column += 1;
                        }
                        s.push(c);
                        chars.next();
                    }
                }
                tokens.push(Token::new("String", &s, line, start_col));
            }
            // Numbers
            '0'..='9' | '-' => {
                let start_col = column;
                let mut num = String::new();
                if c == '-' {
                    num.push(c);
                    chars.next();
                    column += 1;
                }
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_digit() || c == '.' {
                        num.push(c);
                        chars.next();
                        column += 1;
                    } else {
                        break;
                    }
                }
                tokens.push(Token::new("Number", &num, line, start_col));
            }
            // Identifiers & keywords
            'a'..='z' | 'A'..='Z' | '_' => {
                let start_col = column;
                let mut ident = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_alphanumeric() || c == '_' {
                        ident.push(c);
                        chars.next();
                        column += 1;
                    } else {
                        break;
                    }
                }
                let token_type = match ident.as_str() {
                    "material" | "vertex" | "fragment" | "compute" => "Keyword",
                    "true" | "false" => "Boolean",
                    "null" => "Null",
                    _ => "Identifier",
                };
                tokens.push(Token::new(token_type, &ident, line, start_col));
            }
            // Punctuation
            '{' | '}' | '[' | ']' | ':' | ',' => {
                tokens.push(Token::new("Punctuation", &c.to_string(), line, column));
                chars.next();
                column += 1;
            }
            // Unknown
            _ => {
                tokens.push(Token::new("Unknown", &c.to_string(), line, column));
                chars.next();
                column += 1;
            }
        }
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tokenization() {
        let input = r#"material { name: "Test" }"#;
        let tokens = tokenize(input);
        assert_eq!(tokens.len(), 7);
    }
}
