//! A selector (ADR 0290): what a judged rule's input picks from the project, parsed once and printed back in one
//! canonical text, which is what the digests use.
//!
//! ```text
//! selector := "file" "(" ( string ( "," string )* | chain ) ")" | "test" "(" chain ")" | chain
//! chain    := head step*
//! head     := "self" | kinds
//! kinds    := word ( "|" word )*
//! step     := "." path | "<" kinds "." path
//! path     := word ( "." word )*
//! ```

use std::fmt;

/// Where a chain starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Head {
    /// The judged entity.
    This,
    /// Every entity of these kinds with no scope parent.
    Kinds(Vec<String>),
}

/// One step of a chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// `.path`: what the reference field (or member) `path` of each entity names.
    Path(String),
    /// `<kinds.path`: the entities of `kinds` whose `path` names one of the entities so far.
    Referrers { kinds: Vec<String>, path: String },
}

/// An entity-valued chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chain {
    pub head: Head,
    pub steps: Vec<Step>,
}

/// What a selector picks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selector {
    /// Entities.
    Entities(Chain),
    /// Files: the paths the last step's file field holds.
    FilesOf(Chain),
    /// Files: these paths from the project root.
    Files(Vec<String>),
    /// The recorded tests of the entities.
    Tests(Chain),
}

impl Selector {
    /// Read `text`; the error says what is wrong and where.
    pub fn parse(text: &str) -> Result<Selector, String> {
        let mut p = Parser { text, at: 0 };
        let selector = p.selector()?;
        p.space();
        match p.rest().chars().next() {
            None => Ok(selector),
            Some(c) => Err(format!("unexpected '{c}' at {}", p.at)),
        }
    }
}

struct Parser<'t> {
    text: &'t str,
    at: usize,
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

impl Parser<'_> {
    fn rest(&self) -> &str {
        &self.text[self.at..]
    }

    fn space(&mut self) {
        self.at += self.rest().len() - self.rest().trim_start().len();
    }

    fn eat(&mut self, c: char) -> bool {
        self.space();
        let found = self.rest().starts_with(c);
        if found {
            self.at += c.len_utf8();
        }
        found
    }

    fn expect(&mut self, c: char) -> Result<(), String> {
        if self.eat(c) {
            Ok(())
        } else {
            Err(format!("expected '{c}' at {}", self.at))
        }
    }

    fn word(&mut self) -> Result<String, String> {
        self.space();
        let len = self
            .rest()
            .find(|c| !is_word(c))
            .unwrap_or(self.rest().len());
        if len == 0 {
            return Err(format!("expected a name at {}", self.at));
        }
        let word = self.rest()[..len].to_string();
        self.at += len;
        Ok(word)
    }

    /// `word ( "." word )*`; a `.` not followed by a word is an error, not the end.
    fn path(&mut self) -> Result<String, String> {
        let mut path = self.word()?;
        while self.rest().starts_with('.') {
            self.at += 1;
            path.push('.');
            path.push_str(&self.word()?);
        }
        Ok(path)
    }

    fn kinds(&mut self) -> Result<Vec<String>, String> {
        let mut kinds = vec![self.word()?];
        while self.eat('|') {
            kinds.push(self.word()?);
        }
        Ok(kinds)
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut out = String::new();
        let mut chars = self.rest().char_indices();
        while let Some((i, c)) = chars.next() {
            match c {
                '"' => {
                    self.at += i + 1;
                    return Ok(out);
                }
                '\\' => match chars.next() {
                    Some((_, escaped)) => out.push(escaped),
                    None => break,
                },
                c => out.push(c),
            }
        }
        Err(format!("unterminated string at {}", self.at))
    }

    fn chain(&mut self) -> Result<Chain, String> {
        let kinds = self.kinds()?;
        let head = match kinds.as_slice() {
            [one] if one == "self" => Head::This,
            _ => Head::Kinds(kinds),
        };
        let mut steps = Vec::new();
        loop {
            self.space();
            if self.eat('.') {
                steps.push(Step::Path(self.path()?));
            } else if self.eat('<') {
                let kinds = self.kinds()?;
                self.expect('.')?;
                steps.push(Step::Referrers {
                    kinds,
                    path: self.path()?,
                });
            } else {
                return Ok(Chain { head, steps });
            }
        }
    }

    /// Whether the next word is `name` followed by `(`.
    fn call(&mut self, name: &str) -> bool {
        self.space();
        let rest = self.rest();
        let Some(after) = rest.strip_prefix(name) else {
            return false;
        };
        let opens = after.trim_start().starts_with('(');
        if opens {
            self.at += name.len();
            self.space();
            self.at += 1;
        }
        opens
    }

    fn selector(&mut self) -> Result<Selector, String> {
        if self.call("file") {
            self.space();
            let selector = if self.rest().starts_with('"') {
                let mut paths = vec![self.string()?];
                while self.eat(',') {
                    paths.push(self.string()?);
                }
                Selector::Files(paths)
            } else {
                Selector::FilesOf(self.chain()?)
            };
            self.expect(')')?;
            return Ok(selector);
        }
        if self.call("test") {
            let chain = self.chain()?;
            self.expect(')')?;
            return Ok(Selector::Tests(chain));
        }
        Ok(Selector::Entities(self.chain()?))
    }
}

impl fmt::Display for Chain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.head {
            Head::This => f.write_str("self")?,
            Head::Kinds(kinds) => f.write_str(&kinds.join("|"))?,
        }
        for step in &self.steps {
            match step {
                Step::Path(path) => write!(f, ".{path}")?,
                Step::Referrers { kinds, path } => write!(f, "<{}.{path}", kinds.join("|"))?,
            }
        }
        Ok(())
    }
}

impl fmt::Display for Selector {
    /// The canonical text: no spaces, `", "` between the strings of `file()`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Selector::Entities(chain) => write!(f, "{chain}"),
            Selector::FilesOf(chain) => write!(f, "file({chain})"),
            Selector::Tests(chain) => write!(f, "test({chain})"),
            Selector::Files(paths) => {
                f.write_str("file(")?;
                for (i, path) in paths.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "\"{}\"", path.replace('\\', "\\\\").replace('"', "\\\""))?;
                }
                f.write_str(")")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use specforge_test_macros::test as spec;

    #[spec(
        behavior = "declare_judged_rule",
        verify = "a selector prints back to its canonical text"
    )]
    fn spaces_do_not_change_the_canonical_text() {
        let a = Selector::parse("self . phase < task . plan").unwrap();
        let b = Selector::parse("self.phase<task.plan").unwrap();
        assert_eq!(a, b);
        assert_eq!(a.to_string(), "self.phase<task.plan");
        assert_eq!(
            Selector::parse("file ( \"a b\" , \"c\" )")
                .unwrap()
                .to_string(),
            "file(\"a b\", \"c\")"
        );
        assert!(Selector::parse("self.").is_err());
        assert!(Selector::parse("self phase").is_err());
        assert!(Selector::parse("file()").is_err());
        // `file` and `test` are kinds only when no `(` follows.
        assert_eq!(
            Selector::parse("file.path").unwrap().to_string(),
            "file.path"
        );
    }

    fn word() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_]{0,6}".prop_filter("reserved", |w| w != "self" && w != "file" && w != "test")
    }

    fn kinds() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(word(), 1..3)
    }

    fn path() -> impl Strategy<Value = String> {
        prop::collection::vec(word(), 1..3).prop_map(|w| w.join("."))
    }

    fn chain() -> impl Strategy<Value = Chain> {
        let head = prop_oneof![Just(Head::This), kinds().prop_map(Head::Kinds)];
        let step = prop_oneof![
            path().prop_map(Step::Path),
            (kinds(), path()).prop_map(|(kinds, path)| Step::Referrers { kinds, path }),
        ];
        // `.a.b` is one path: the parser reads a `.path` only straight after the head, and a `<kinds.path` ends in
        // the path it reads.
        let only_first_is_a_path = |steps: &Vec<Step>| {
            steps
                .iter()
                .skip(1)
                .all(|step| matches!(step, Step::Referrers { .. }))
        };
        (
            head,
            prop::collection::vec(step, 0..3).prop_filter("later path", only_first_is_a_path),
        )
            .prop_map(|(head, steps)| Chain { head, steps })
    }

    fn selector() -> impl Strategy<Value = Selector> {
        prop_oneof![
            chain().prop_map(Selector::Entities),
            chain().prop_map(Selector::FilesOf),
            chain().prop_map(Selector::Tests),
            prop::collection::vec("[ -~]{1,8}", 1..3).prop_map(Selector::Files),
        ]
    }

    proptest! {
        #[spec(
            behavior = "declare_judged_rule",
            verify = "a selector prints back to its canonical text"
        )]
        fn print_then_parse_is_the_identity(selector in selector()) {
            prop_assert_eq!(Selector::parse(&selector.to_string()), Ok(selector));
        }
    }
}
