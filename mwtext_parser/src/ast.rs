use std::ops::Deref;

use crate::lexer::Token;

#[derive(Debug, Clone, PartialEq)]
pub struct Heading {
    pub level: usize,
    pub text: String,
}

impl ToString for Heading {
    fn to_string(&self) -> String {
        let mut result = String::new();
        for _ in 0..self.level {
            result.push('=');
        }
        result.push_str(&format!(" {} ", self.text));
        for _ in 0..self.level {
            result.push('=');
        }
        result
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt<'a> {
    Clause {
        text: Vec<Token<'a>>,
        index: usize,
        changes: bool,
    },
    Preamble(Vec<Token<'a>>),
}

impl ToString for Stmt<'_> {
    fn to_string(&self) -> String {
        match self {
            Stmt::Clause {
                text,
                index,
                changes: _,
            } => {
                let mut result = String::new();
                result.push_str(format!("§{}. ", index).as_str());
                for token in text {
                    result.push_str(token.to_string().as_str());
                }
                result.push('\n');
                result.push('\n');
                result
            }
            Stmt::Preamble(text) => {
                let mut result = String::new();
                for token in text {
                    result.push_str(token.to_string().as_str());
                }
                result.push_str("<br>");
                result
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Table<'a> {
    pub style: &'a str,
    pub header: Option<Vec<Cell<'a>>>,
    pub cells: Vec<Vec<Cell<'a>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cell<'a>(Vec<Token<'a>>);

impl<'a> Deref for Cell<'a> {
    type Target = Vec<Token<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> ToString for Cell<'a> {
    fn to_string(&self) -> String {
        let mut result = String::new();
        for token in self.0.iter() {
            result.push_str(&token.to_string());
        }
        result
    }
}

impl ToString for Table<'_> {
    fn to_string(&self) -> String {
        let mut result = format!("{{| {}\n|-\n", self.style);
        if self.header.is_some() {
            for header in self.header.as_ref().unwrap().iter() {
                result.push_str(&format!("! {}\n", header.to_string()));
            }
            result.push_str("|-\n");
        }
        for row in self.cells.iter() {
            for cell in row.iter() {
                result.push_str(&format!("| {}\n", cell.to_string()));
            }
            result.push_str("|-\n");
        }
        result.push_str("|}");
        result
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct HTMLClass<'a> {
    pub name: &'a str,
    pub attributes: Vec<(&'a str, &'a str)>,
    pub text: Vec<Token<'a>>,
}
