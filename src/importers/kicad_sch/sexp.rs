use anyhow::{Result, bail};

#[derive(Debug, Clone)]
pub(super) enum Sexp {
    Atom(String),
    Str(String),
    List(Vec<Sexp>),
}

pub(super) fn parse_sexp_document(text: &str) -> Result<Sexp> {
    let mut parser = Parser { text, offset: 0 };
    parser.skip_ws_and_comments();
    let sexp = parser.parse_one()?;
    parser.skip_ws_and_comments();
    if parser.offset != parser.text.len() {
        bail!("Unexpected trailing data in KiCad S-expression.");
    }
    Ok(sexp)
}

struct Parser<'a> {
    text: &'a str,
    offset: usize,
}

impl Parser<'_> {
    fn parse_one(&mut self) -> Result<Sexp> {
        self.skip_ws_and_comments();
        match self.peek_char() {
            Some('(') => self.parse_list(),
            Some('"') => self.parse_string().map(Sexp::Str),
            Some(_) => self.parse_atom().map(Sexp::Atom),
            None => bail!("Unexpected end of KiCad S-expression."),
        }
    }

    fn parse_list(&mut self) -> Result<Sexp> {
        self.expect_char('(')?;
        let mut items = Vec::new();
        loop {
            self.skip_ws_and_comments();
            match self.peek_char() {
                Some(')') => {
                    self.expect_char(')')?;
                    break;
                }
                Some(_) => items.push(self.parse_one()?),
                None => bail!("Unclosed KiCad S-expression list."),
            }
        }
        Ok(Sexp::List(items))
    }

    fn parse_string(&mut self) -> Result<String> {
        self.expect_char('"')?;
        let mut output = String::new();
        loop {
            let Some(character) = self.next_char() else {
                bail!("Unclosed KiCad S-expression string.");
            };
            match character {
                '"' => break,
                '\\' => {
                    let Some(escaped) = self.next_char() else {
                        bail!("Unclosed KiCad S-expression escape.");
                    };
                    output.push(escaped);
                }
                _ => output.push(character),
            }
        }
        Ok(output)
    }

    fn parse_atom(&mut self) -> Result<String> {
        let start = self.offset;
        while let Some(character) = self.peek_char() {
            if character.is_whitespace() || character == '(' || character == ')' {
                break;
            }
            self.next_char();
        }
        if self.offset == start {
            bail!("Expected KiCad S-expression atom.");
        }
        Ok(self.text[start..self.offset].to_string())
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            while self.peek_char().is_some_and(char::is_whitespace) {
                self.next_char();
            }
            if self.peek_char() == Some(';') {
                while let Some(character) = self.next_char() {
                    if character == '\n' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    fn expect_char(&mut self, expected: char) -> Result<()> {
        match self.next_char() {
            Some(actual) if actual == expected => Ok(()),
            Some(actual) => bail!("Expected {expected:?}, got {actual:?}."),
            None => bail!("Expected {expected:?}, got end of input."),
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.text[self.offset..].chars().next()
    }

    fn next_char(&mut self) -> Option<char> {
        let character = self.peek_char()?;
        self.offset += character.len_utf8();
        Some(character)
    }
}

pub(super) fn as_list(sexp: &Sexp) -> Option<&[Sexp]> {
    maybe_list(sexp)
}

pub(super) fn maybe_list(sexp: &Sexp) -> Option<&[Sexp]> {
    match sexp {
        Sexp::List(items) => Some(items),
        _ => None,
    }
}

pub(super) fn tag(list: &[Sexp]) -> Option<&str> {
    string_at(list, 0)
}

pub(super) fn string_at(list: &[Sexp], index: usize) -> Option<&str> {
    match list.get(index)? {
        Sexp::Atom(value) | Sexp::Str(value) => Some(value),
        Sexp::List(_) => None,
    }
}

pub(super) fn numeric_at(list: &[Sexp], index: usize) -> Option<f64> {
    let value = string_at(list, index)?.parse::<f64>().ok()?;
    value.is_finite().then_some(value)
}

pub(super) fn child_list<'a>(list: &'a [Sexp], name: &'a str) -> Option<&'a [Sexp]> {
    list_children(list, name).next()
}

pub(super) fn list_children<'a>(
    list: &'a [Sexp],
    name: &'a str,
) -> impl Iterator<Item = &'a [Sexp]> {
    list.iter().skip(1).filter_map(move |item| {
        let child = maybe_list(item)?;
        (tag(child) == Some(name)).then_some(child)
    })
}
