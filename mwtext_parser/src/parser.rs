use chumsky::{
    ConfigIterParser, IterParser, Parser,
    input::ValueInput,
    prelude::{just, none_of},
    span::SimpleSpan,
};

use crate::{
    ast::Heading,
    lexer::{Token, to_string},
};

pub(crate) fn header<'a, I>() -> impl Parser<'a, I, Heading>
where
    I: ValueInput<'a, Token = Token<'a>, Span = SimpleSpan>,
{
    let equals = just(Token::Equal).repeated();
    let header_text = none_of(Token::Equal)
        .repeated()
        .at_least(1)
        .collect::<Vec<Token>>();
    equals
        .at_least(1)
        .count()
        .then_with_ctx(
            header_text.then_ignore(
                just(Token::Equal)
                    .repeated()
                    .configure(|cfg, ctx| cfg.exactly(*ctx)),
            ),
        )
        .map(|(level, text)| Heading {
            level,
            text: to_string(text).trim().to_string(),
        })
}

#[cfg(test)]
mod tests {
    use chumsky::input::Stream;
    use logos::Logos;

    use super::*;

    #[test]
    fn test_header() {
        let input = "=== Header ===";
        let tokens = Stream::from_iter(Token::lexer(input).filter_map(|result| result.ok()));
        dbg!(
            Token::lexer(input)
                .filter_map(|result| result.ok())
                .collect::<Vec<Token>>()
        );
        let result = header().parse(tokens);
        dbg!(result.unwrap());
    }
}
