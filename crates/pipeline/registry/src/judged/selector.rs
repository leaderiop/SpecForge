//! A selector (ADR 0290): what a judged rule's input picks from the project, parsed once and printed back in one
//! canonical text, which is what the digests use.
//!
//! ```text
//! selector := "file" "(" ( string ( "," string )* | chain ) ")" | "test" "(" chain ( "," chain )* ")" | chain
//! chain    := head step*
//! head     := "self" | kinds
//! kinds    := pick ( "|" pick )*
//! pick     := word ( "[" word "=" value ( "|" value )* "]" )?
//! value    := word
//! step     := "." path | "<" kinds "." path
//! path     := word ( "." word )*
//! ```
//!
//! A pick's filter (ADR 0356 D13) keeps only the entities of its kind whose field holds one of the values.

use std::collections::BTreeSet;
use std::fmt;

/// Where a chain starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Head {
    /// The judged entity.
    This,
    /// Every entity of these kinds with no scope parent.
    Kinds(Kinds),
}

/// The kinds a head or a referrer step picks, each with at most one field filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kinds {
    /// The kind names, as written.
    pub names: Vec<String>,
    /// The field filters, at most one per kind.
    pub filters: Vec<Filter>,
}

/// `kind[field=value|…]`: only the entities of `kind` whose authored scalar `field` holds one of `values`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    pub kind: String,
    pub field: String,
    pub values: BTreeSet<String>,
}

impl Kinds {
    /// Kinds with no filter.
    pub fn of(names: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Kinds {
            names: names.into_iter().map(Into::into).collect(),
            filters: Vec::new(),
        }
    }

    /// The kind names, as written.
    pub fn iter(&self) -> std::slice::Iter<'_, String> {
        self.names.iter()
    }

    /// The filter on `kind`, when it carries one.
    pub fn filter(&self, kind: &str) -> Option<&Filter> {
        self.filters.iter().find(|filter| filter.kind == kind)
    }
}

impl<'a> IntoIterator for &'a Kinds {
    type Item = &'a String;
    type IntoIter = std::slice::Iter<'a, String>;
    fn into_iter(self) -> Self::IntoIter {
        self.names.iter()
    }
}

/// One step of a chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// `.path`: what the reference field (or member) `path` of each entity names.
    Path(String),
    /// `<kinds.path`: the entities of `kinds` whose `path` names one of the entities so far.
    Referrers { kinds: Kinds, path: String },
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
    /// The recorded tests of the entities the chains pick, together (ADR 0331 D7).
    Tests(Vec<Chain>),
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

    fn kinds(&mut self) -> Result<Kinds, String> {
        let mut kinds = Kinds::of(Vec::<String>::new());
        loop {
            let name = self.word()?;
            if self.eat('[') {
                let at = self.at;
                if name == "self" {
                    return Err(format!("'self' carries no filter at {at}"));
                }
                if kinds.filter(&name).is_some() {
                    return Err(format!("kind '{name}' carries one filter at {at}"));
                }
                let field = self.word()?;
                self.expect('=')?;
                let mut values = BTreeSet::from([self.word()?]);
                while self.eat('|') {
                    values.insert(self.word()?);
                }
                self.expect(']')?;
                kinds.filters.push(Filter {
                    kind: name.clone(),
                    field,
                    values,
                });
            }
            kinds.names.push(name);
            if !self.eat('|') {
                return Ok(kinds);
            }
        }
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
        let head = match kinds.names.as_slice() {
            [one] if one == "self" && kinds.filters.is_empty() => Head::This,
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
            let mut chains = vec![self.chain()?];
            while self.eat(',') {
                chains.push(self.chain()?);
            }
            self.expect(')')?;
            return Ok(Selector::Tests(chains));
        }
        Ok(Selector::Entities(self.chain()?))
    }
}

impl fmt::Display for Kinds {
    /// Each name with its filter after its first occurrence, the filter's values sorted.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (at, name) in self.names.iter().enumerate() {
            if at > 0 {
                f.write_str("|")?;
            }
            f.write_str(name)?;
            let first = !self.names[..at].contains(name);
            if let Some(filter) = self.filter(name).filter(|_| first) {
                let values: Vec<&str> = filter.values.iter().map(String::as_str).collect();
                write!(f, "[{}={}]", filter.field, values.join("|"))?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for Chain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.head {
            Head::This => f.write_str("self")?,
            Head::Kinds(kinds) => write!(f, "{kinds}")?,
        }
        for step in &self.steps {
            match step {
                Step::Path(path) => write!(f, ".{path}")?,
                Step::Referrers { kinds, path } => write!(f, "<{kinds}.{path}")?,
            }
        }
        Ok(())
    }
}

impl fmt::Display for Selector {
    /// The canonical text: no spaces, `", "` between the strings of `file()` and the chains of `test()`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Selector::Entities(chain) => write!(f, "{chain}"),
            Selector::FilesOf(chain) => write!(f, "file({chain})"),
            Selector::Tests(chains) => {
                f.write_str("test(")?;
                for (i, chain) in chains.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{chain}")?;
                }
                f.write_str(")")
            }
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
        assert_eq!(
            Selector::parse("test( self ,self.plan )")
                .unwrap()
                .to_string(),
            "test(self, self.plan)"
        );
        assert!(Selector::parse("test(self,)").is_err());
        assert!(Selector::parse("self[a=b]").is_err());
        // `file` and `test` are kinds only when no `(` follows.
        assert_eq!(
            Selector::parse("file.path").unwrap().to_string(),
            "file.path"
        );
    }

    #[spec(
        behavior = "declare_judged_rule",
        verify = "a selector's kind filter prints its values sorted and parses back"
    )]
    fn a_kind_filter_prints_its_values_sorted_and_parses_back() {
        let selector = Selector::parse(
            "self.phase < decision [ origin = summary | context | context ] .phase",
        )
        .unwrap();
        assert_eq!(
            selector.to_string(),
            "self.phase<decision[origin=context|summary].phase"
        );
        assert_eq!(Selector::parse(&selector.to_string()), Ok(selector.clone()));
        let Selector::Entities(chain) = selector else {
            panic!("an entities selector")
        };
        let Step::Referrers { kinds, .. } = &chain.steps[1] else {
            panic!("a referrer step")
        };
        assert_eq!(kinds.names, ["decision"]);
        let filter = kinds.filter("decision").expect("decision carries a filter");
        assert_eq!(filter.field, "origin");
        assert_eq!(
            filter.values,
            BTreeSet::from(["context".to_string(), "summary".to_string()])
        );
        assert_eq!(
            Selector::parse("decision[origin=project]|phase")
                .unwrap()
                .to_string(),
            "decision[origin=project]|phase"
        );
        assert!(Selector::parse("decision[origin=a]|decision[origin=b]").is_err());
        assert!(Selector::parse("decision[origin]").is_err());
        assert!(Selector::parse("decision[origin=]").is_err());
    }

    fn word() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_]{0,6}".prop_filter("reserved", |w| w != "self" && w != "file" && w != "test")
    }

    fn filter() -> impl Strategy<Value = Option<(String, BTreeSet<String>)>> {
        prop::option::of((word(), prop::collection::btree_set(word(), 1..3)))
    }

    fn kinds() -> impl Strategy<Value = Kinds> {
        prop::collection::btree_map(word(), filter(), 1..3).prop_map(|picks| Kinds {
            names: picks.keys().cloned().collect(),
            filters: picks
                .into_iter()
                .filter_map(|(kind, filter)| {
                    filter.map(|(field, values)| Filter {
                        kind,
                        field,
                        values,
                    })
                })
                .collect(),
        })
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
            prop::collection::vec(chain(), 1..3).prop_map(Selector::Tests),
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
