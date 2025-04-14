use std::fmt::Display;

use logos::Logos;

#[derive(Debug, Clone, PartialEq, Logos)]
pub enum Token<'a> {
    #[token("=")]
    Equal,
    #[token("\n")]
    #[token("\r")]
    Newline,
    #[regex(r#"[\n|\r]§?\d+"#, |lex| lex.slice().trim().trim_matches('§').parse::<usize>().ok())]
    Index(usize),
    #[token(" ")]
    Whitespace,
    #[token("\t")]
    #[token(r#"    "#)]
    Tabspace,
    #[token("<")]
    LAngle,
    #[token(">")]
    RAngle,
    #[token("</")]
    ClassTerminator,
    #[regex(r#"[[:alnum:]|\\\/\-+*%!_\.]+"#)]
    Word(&'a str),
    #[token(r#"'''"#)]
    Bold,
    #[token(r#"''"#)]
    Italic,
    #[token(r#"[["#)]
    InternalLink,
    #[token(r"]]")]
    InternalLinkEnd,
    #[token(r#"["#)]
    ExternalLink,
    #[token(r#"]"#)]
    ExternalLinkEnd,
    #[token(r#"----"#)]
    Separator,
    #[token(r#"{|"#)]
    TableStart,
    #[token(r#"|}"#)]
    TableEnd,
    #[regex("\n! ")]
    TableHeading,
    #[regex(r#"\|\s*"#, priority = 3)]
    Pipe,
    #[token(r#"|-"#)]
    TableSeparator,
}

pub fn to_string<'a>(tokens: Vec<Token<'a>>) -> String {
    let mut result = String::new();
    for token in tokens {
        result.push_str(&token.to_string());
    }
    result
}

impl<'a> Display for Token<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Equal => write!(f, "="),
            Token::Newline => write!(f, "\n"),
            Token::Index(index) => write!(f, "{}", index),
            Token::Whitespace => write!(f, " "),
            Token::Tabspace => write!(f, "\t"),
            Token::Word(word) => write!(f, "{}", word),
            Token::Bold => write!(f, "'''"),
            Token::Italic => write!(f, "''"),
            Token::InternalLink => write!(f, "[["),
            Token::InternalLinkEnd => write!(f, "]]"),
            Token::ExternalLink => write!(f, "["),
            Token::ExternalLinkEnd => write!(f, "]"),
            Token::Separator => write!(f, "----"),
            Token::TableStart => write!(f, "{{|"),
            Token::TableEnd => write!(f, "|}}"),
            Token::TableHeading => write!(f, "\n! "),
            Token::Pipe => write!(f, "|"),
            Token::TableSeparator => write!(f, "|-"),
            Token::LAngle => write!(f, "<"),
            Token::RAngle => write!(f, ">"),
            Token::ClassTerminator => write!(f, "</"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;

    #[test]
    fn test_tokens() {
        let test_cases = vec![
            ("=", Some(Token::Equal)),
            ("\n", Some(Token::Newline)),
            ("\n§123", Some(Token::Index(123))),
            ("\n123", Some(Token::Index(123))),
            (" ", Some(Token::Whitespace)),
            ("\t", Some(Token::Tabspace)),
            ("    ", Some(Token::Tabspace)),
            ("hello", Some(Token::Word("hello"))),
            ("!", Some(Token::Word("!"))),
            ("-", Some(Token::Word("-"))),
            (".", Some(Token::Word("."))),
            ("'''", Some(Token::Bold)),
            ("''", Some(Token::Italic)),
            ("[[", Some(Token::InternalLink)),
            ("]]", Some(Token::InternalLinkEnd)),
            ("[", Some(Token::ExternalLink)),
            ("]", Some(Token::ExternalLinkEnd)),
            ("----", Some(Token::Separator)),
            ("{|", Some(Token::TableStart)),
            ("|}", Some(Token::TableEnd)),
            ("\n! ", Some(Token::TableHeading)),
            ("|", Some(Token::Pipe)),
            ("|-", Some(Token::TableSeparator)),
        ];

        for (input, expected) in test_cases {
            eprintln!("Testing input: {:#?}", input);
            let mut lexer = Token::lexer(input);
            assert_eq!(lexer.next().map(Result::unwrap), expected);
            assert_eq!(lexer.next(), None);
        }
    }
    #[test]
    fn test_complex_expr() {
        let tests = vec![
            (
                "Hello World",
                vec![
                    Token::Word("Hello"),
                    Token::Whitespace,
                    Token::Word("World"),
                ],
            ),
            (
                "|-
| Date of judgment
| 19th December 2022
|-",
                vec![
                    Token::TableSeparator,
                    Token::Newline,
                    Token::Pipe,
                    Token::Word("Date"),
                    Token::Whitespace,
                    Token::Word("of"),
                    Token::Whitespace,
                    Token::Word("judgment"),
                    Token::Newline,
                    Token::Pipe,
                    Token::Word("19th"),
                    Token::Whitespace,
                    Token::Word("December"),
                    Token::Whitespace,
                    Token::Word("2022"),
                    Token::Newline,
                    Token::TableSeparator,
                ],
            ),
            ("===", vec![Token::Equal; 3])
        ];
        for (input, expected) in tests {
            let mut lexer = Token::lexer(input);
            for expected_token in expected {
                let token = lexer.next().unwrap();
                assert_eq!(token, Ok(expected_token));
            }
        }
    }
}
