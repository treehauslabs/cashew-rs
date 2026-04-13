use crate::error::{CashewError, Result};
use crate::query::expression::CashewExpression;

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Word(String),
    StringLit(String),
    Number(i64),
    Equals,
    Pipe,
}

/// Parses a cashew query string into a list of expressions.
///
/// # Examples
/// ```
/// use cashew::query::CashewParser;
///
/// let exprs = CashewParser::parse(r#"get "alice""#).unwrap();
/// ```
pub struct CashewParser;

impl CashewParser {
    pub fn parse(input: &str) -> Result<Vec<CashewExpression>> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(CashewError::EmptyExpression);
        }
        let tokens = tokenize(trimmed)?;
        let segments = split_by_pipe(&tokens);
        let mut expressions = Vec::new();
        for segment in segments {
            if segment.is_empty() {
                continue;
            }
            expressions.push(parse_segment(&segment)?);
        }
        if expressions.is_empty() {
            return Err(CashewError::EmptyExpression);
        }
        Ok(expressions)
    }
}

fn tokenize(input: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if ch.is_whitespace() {
            i += 1;
            continue;
        }

        if ch == '|' {
            tokens.push(Token::Pipe);
            i += 1;
            continue;
        }

        if ch == '=' {
            tokens.push(Token::Equals);
            i += 1;
            continue;
        }

        if ch == '"' || ch == '\'' {
            let quote = ch;
            i += 1;
            let mut s = String::new();
            while i < chars.len() && chars[i] != quote {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1;
                    s.push(chars[i]);
                } else {
                    s.push(chars[i]);
                }
                i += 1;
            }
            if i < chars.len() {
                i += 1; // skip closing quote
            }
            tokens.push(Token::StringLit(s));
            continue;
        }

        if ch == '-' || ch.is_ascii_digit() {
            let start = i;
            if ch == '-' {
                i += 1;
            }
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let num_str: String = chars[start..i].iter().collect();
            if let Ok(n) = num_str.parse::<i64>() {
                tokens.push(Token::Number(n));
            } else {
                tokens.push(Token::Word(num_str.to_lowercase()));
            }
            continue;
        }

        if ch.is_alphanumeric() || ch == '_' {
            let start = i;
            while i < chars.len()
                && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '-')
            {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            tokens.push(Token::Word(word.to_lowercase()));
            continue;
        }

        i += 1;
    }

    Ok(tokens)
}

fn split_by_pipe(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();
    for token in tokens {
        if *token == Token::Pipe {
            if !current.is_empty() {
                segments.push(current);
                current = Vec::new();
            }
        } else {
            current.push(token.clone());
        }
    }
    if !current.is_empty() {
        segments.push(current);
    }
    segments
}

fn parse_segment(tokens: &[Token]) -> Result<CashewExpression> {
    if tokens.is_empty() {
        return Err(CashewError::EmptyExpression);
    }

    let command = match &tokens[0] {
        Token::Word(w) => w.as_str(),
        _ => return Err(CashewError::ParseError("expected command word".into())),
    };

    match command {
        "get" => {
            if tokens.len() < 2 {
                return Err(CashewError::ParseError(
                    "get requires a key argument".into(),
                ));
            }
            match &tokens[1] {
                Token::StringLit(s) => Ok(CashewExpression::Get(s.clone())),
                Token::Number(n) => Ok(CashewExpression::GetAt(*n as usize)),
                Token::Word(w) => Ok(CashewExpression::Get(w.clone())),
                _ => Err(CashewError::ParseError("invalid get argument".into())),
            }
        }
        "keys" => {
            if tokens.len() > 1 && matches!(&tokens[1], Token::Word(w) if w == "sorted") {
                let (limit, after) = parse_pagination(&tokens[2..])?;
                Ok(CashewExpression::SortedKeys { limit, after })
            } else {
                Ok(CashewExpression::Keys)
            }
        }
        "values" => {
            if tokens.len() > 1 && matches!(&tokens[1], Token::Word(w) if w == "sorted") {
                let (limit, after) = parse_pagination(&tokens[2..])?;
                Ok(CashewExpression::SortedValues { limit, after })
            } else {
                Ok(CashewExpression::Values)
            }
        }
        "count" | "size" => Ok(CashewExpression::Count),
        "contains" | "has" => {
            if tokens.len() < 2 {
                return Err(CashewError::ParseError("contains requires a key".into()));
            }
            let key = extract_string(&tokens[1])?;
            Ok(CashewExpression::Contains(key))
        }
        "first" => Ok(CashewExpression::First),
        "last" => Ok(CashewExpression::Last),
        "insert" | "add" => {
            let (key, value) = parse_key_value(&tokens[1..])?;
            Ok(CashewExpression::Insert { key, value })
        }
        "update" => {
            let (key, value) = parse_key_value(&tokens[1..])?;
            Ok(CashewExpression::Update { key, value })
        }
        "set" | "put" => {
            let (key, value) = parse_key_value(&tokens[1..])?;
            Ok(CashewExpression::Set { key, value })
        }
        "delete" | "remove" => {
            if tokens.len() < 2 {
                return Err(CashewError::ParseError("delete requires a key".into()));
            }
            let key = extract_string(&tokens[1])?;
            Ok(CashewExpression::Delete(key))
        }
        "append" => {
            if tokens.len() < 2 {
                return Err(CashewError::ParseError("append requires a value".into()));
            }
            let val = extract_string(&tokens[1])?;
            Ok(CashewExpression::Append(val))
        }
        _ => Err(CashewError::UnsupportedOperation(command.to_string())),
    }
}

fn parse_pagination(tokens: &[Token]) -> Result<(Option<usize>, Option<String>)> {
    let mut limit = None;
    let mut after = None;
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::Word(w) if w == "limit" => {
                i += 1;
                if i < tokens.len() {
                    if let Token::Number(n) = &tokens[i] {
                        limit = Some(*n as usize);
                    }
                }
            }
            Token::Word(w) if w == "after" => {
                i += 1;
                if i < tokens.len() {
                    after = Some(extract_string(&tokens[i])?);
                }
            }
            _ => {}
        }
        i += 1;
    }
    Ok((limit, after))
}

fn parse_key_value(tokens: &[Token]) -> Result<(String, String)> {
    if tokens.len() < 3 {
        return Err(CashewError::ParseError("expected: key = value".into()));
    }
    let key = extract_string(&tokens[0])?;
    if tokens[1] != Token::Equals {
        return Err(CashewError::ParseError("expected '=' after key".into()));
    }
    let value = extract_string(&tokens[2])?;
    Ok((key, value))
}

fn extract_string(token: &Token) -> Result<String> {
    match token {
        Token::StringLit(s) => Ok(s.clone()),
        Token::Word(w) => Ok(w.clone()),
        Token::Number(n) => Ok(n.to_string()),
        _ => Err(CashewError::ParseError("expected string or word".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_get() {
        let exprs = CashewParser::parse(r#"get "alice""#).unwrap();
        assert_eq!(exprs, vec![CashewExpression::Get("alice".to_string())]);
    }

    #[test]
    fn test_parse_keys() {
        let exprs = CashewParser::parse("keys").unwrap();
        assert_eq!(exprs, vec![CashewExpression::Keys]);
    }

    #[test]
    fn test_parse_sorted_keys() {
        let exprs = CashewParser::parse("keys sorted limit 10").unwrap();
        assert_eq!(
            exprs,
            vec![CashewExpression::SortedKeys {
                limit: Some(10),
                after: None
            }]
        );
    }

    #[test]
    fn test_parse_count() {
        let exprs = CashewParser::parse("count").unwrap();
        assert_eq!(exprs, vec![CashewExpression::Count]);
    }

    #[test]
    fn test_parse_insert() {
        let exprs = CashewParser::parse(r#"insert "name" = "alice""#).unwrap();
        assert_eq!(
            exprs,
            vec![CashewExpression::Insert {
                key: "name".to_string(),
                value: "alice".to_string()
            }]
        );
    }

    #[test]
    fn test_parse_delete() {
        let exprs = CashewParser::parse(r#"delete "name""#).unwrap();
        assert_eq!(exprs, vec![CashewExpression::Delete("name".to_string())]);
    }

    #[test]
    fn test_parse_pipe() {
        let exprs = CashewParser::parse(r#"get "users" | keys"#).unwrap();
        assert_eq!(
            exprs,
            vec![
                CashewExpression::Get("users".to_string()),
                CashewExpression::Keys,
            ]
        );
    }

    #[test]
    fn test_parse_contains() {
        let exprs = CashewParser::parse(r#"contains "alice""#).unwrap();
        assert_eq!(exprs, vec![CashewExpression::Contains("alice".to_string())]);
    }

    #[test]
    fn test_parse_empty() {
        assert!(CashewParser::parse("").is_err());
    }

    #[test]
    fn test_parse_append() {
        let exprs = CashewParser::parse(r#"append "value""#).unwrap();
        assert_eq!(exprs, vec![CashewExpression::Append("value".to_string())]);
    }
}
