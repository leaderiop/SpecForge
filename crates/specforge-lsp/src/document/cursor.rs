//! What the LSP knows about one position of a document (ADR 0023).

use specforge_common::Sym;
use specforge_ops::navigate::{Navigator, Occurrence};
use specforge_parser::lex::LexemeKind;
use tower_lsp::lsp_types::{Position, Range};

use super::LineIndex;
use super::syntax::{Expect, Role, Syntax, line_end, statement_start};

/// What the LSP knows about one position of a document, read from its
/// syntax. The entity it names is read from navigation, which checks each
/// token against the same text (ADR 0016's `Precision::Token`), so a graph
/// that lags the buffer never names the wrong entity.
pub struct Cursor<'d> {
    pub(super) text: &'d str,
    pub(super) index: &'d LineIndex,
    pub(super) syntax: &'d Syntax,
    pub(super) offset: usize,
    pub(super) position: Position,
}

/// Where a cursor is lexically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    Code,
    String,
    Comment,
}

/// The identifier or scheme ref ID under a cursor: the cursor inside it or
/// just past its end (as navigation's `occurrence_at` counts).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word<'d> {
    pub text: &'d str,
    pub range: Range,
    /// Its text before the cursor: what completion matches.
    pub prefix: &'d str,
}

/// The entity block around a cursor, as written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityAt<'d> {
    pub kind: &'d str,
    pub id: Option<&'d str>,
}

/// What a cursor names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A `use` statement's path: the cursor anywhere on the statement but
    /// on a binding that names an entity.
    Import { path: String },
    /// An entity, and the range of the token that names it here.
    Entity { id: Sym, origin: Range },
    /// A field of the enclosing entity's kind: the cursor on its name, in
    /// the entity's own body.
    Field { kind: String, field: String },
}

impl<'d> Cursor<'d> {
    /// The cursor's position.
    pub fn position(&self) -> Position {
        self.position
    }

    /// The index of the last lexeme starting at or before the cursor.
    fn last_started(&self) -> Option<usize> {
        let lexemes = &self.syntax.lexemes;
        lexemes
            .partition_point(|l| l.start <= self.offset)
            .checked_sub(1)
    }

    /// The name lexeme under the cursor: the cursor inside it or just past
    /// its end.
    fn word_index(&self) -> Option<usize> {
        let lexemes = &self.syntax.lexemes;
        let last = self.last_started()?;
        let lexeme = lexemes[last];
        if lexeme.is_name() && self.offset <= lexeme.end {
            return Some(last);
        }
        let before = last.checked_sub(1)?;
        (lexeme.start == self.offset
            && lexemes[before].end == self.offset
            && lexemes[before].is_name())
        .then_some(before)
    }

    /// The identifier or scheme ref ID under the cursor.
    pub fn word(&self) -> Option<Word<'d>> {
        let lexeme = self.syntax.lexemes[self.word_index()?];
        Some(Word {
            text: lexeme.text(self.text),
            range: Range {
                start: self.index.position(lexeme.start),
                end: self.index.position(lexeme.end),
            },
            prefix: &self.text[lexeme.start..self.offset],
        })
    }

    /// Whether the cursor is in code, a string or a comment.
    pub fn place(&self) -> Place {
        let Some(last) = self.last_started() else {
            return Place::Code;
        };
        let lexeme = self.syntax.lexemes[last];
        if lexeme.start >= self.offset || self.offset > lexeme.end {
            return Place::Code;
        }
        match lexeme.kind {
            LexemeKind::Comment => Place::Comment,
            LexemeKind::Str if self.offset < lexeme.end || !closed(lexeme.text(self.text)) => {
                Place::String
            }
            _ => Place::Code,
        }
    }

    /// The walk's state at the cursor, and the frame then innermost.
    pub(super) fn context(&self) -> (Expect, Option<u32>) {
        if let Some(word) = self.word_index() {
            return self.syntax.before[word];
        }
        let lexemes = &self.syntax.lexemes;
        let before = lexemes.partition_point(|l| l.end <= self.offset);
        let previous = (0..before)
            .rev()
            .find(|&i| lexemes[i].kind != LexemeKind::Comment);
        let Some(previous) = previous else {
            return (Expect::Top, None);
        };
        let (state, top) = self.syntax.after[previous];
        let newline = self.text.as_bytes()[lexemes[previous].end..self.offset].contains(&b'\n');
        if newline {
            let start = statement_start(&self.syntax.frames, top);
            return (line_end(state, None, start), top);
        }
        (state, top)
    }

    /// The header the cursor is on or right after (an inline `ref`, a
    /// union, a header being typed).
    fn header(&self) -> Option<u32> {
        if let Some(word) = self.word_index()
            && let Some(header) = self.syntax.header_of[word]
        {
            return Some(header);
        }
        match self.context().0 {
            Expect::HeaderName(header)
            | Expect::HeaderTitle(header)
            | Expect::HeaderOpen(header) => Some(header),
            _ => None,
        }
    }

    /// The entity block around the cursor, as written: its body, or its
    /// header. `None` at the top level and in a define block.
    pub fn entity(&self) -> Option<EntityAt<'d>> {
        let (_, top) = self.context();
        let header = match self.syntax.entity_frame(top) {
            Some(frame) => self.syntax.frames[frame as usize].header?,
            None => self.header()?,
        };
        let header = self.syntax.headers[header as usize];
        Some(EntityAt {
            kind: self.syntax.text(self.text, header.kind),
            id: header.name.map(|name| self.syntax.text(self.text, name)),
        })
    }

    /// The key lexeme of the field, in the entity's own body, whose name,
    /// value or list the cursor is on.
    pub(super) fn field_key(&self) -> Option<u32> {
        let key = match self.word_index().and_then(|w| self.syntax.key_of[w]) {
            Some(key) => key,
            None => match self.context() {
                (
                    Expect::Value(ctx)
                    | Expect::AfterValue(ctx)
                    | Expect::Annotation(ctx)
                    | Expect::AnnotationArg(ctx)
                    | Expect::Suffix(ctx)
                    | Expect::ExprOpen(ctx),
                    _,
                ) => ctx.key?,
                (Expect::List | Expect::Expr | Expect::Params, Some(frame)) => {
                    self.syntax.frames[frame as usize].key?
                }
                _ => return None,
            },
        };
        self.syntax.in_own_body(key).then_some(key)
    }

    /// The field, in the entity's own body, whose name, value or list the
    /// cursor is on.
    pub fn field(&self) -> Option<&'d str> {
        Some(self.syntax.text(self.text, self.field_key()?))
    }

    /// The path of the `use` statement the cursor is on (anywhere from its
    /// first lexeme to its last), without its quotes.
    pub fn import(&self) -> Option<&'d str> {
        let lexemes = &self.syntax.lexemes;
        let import = self.syntax.imports.iter().find(|import| {
            lexemes[import.first as usize].start <= self.offset
                && self.offset <= lexemes[import.last as usize].end
        })?;
        let path = lexemes[import.path? as usize].text(self.text);
        Some(path.trim_matches('"'))
    }

    /// The declaration or reference token under the cursor, as navigation
    /// reads it: what prepareRename answers and rename may edit.
    pub fn occurrence<F: Fn(&str) -> Option<String>>(
        &self,
        nav: &Navigator<'_, F>,
        file: &str,
    ) -> Option<Occurrence> {
        let (line, col) = self.index.source_position(self.position)?;
        nav.occurrence_at(file, line, col)
    }

    /// What the cursor names, asked in this order: a `use` binding's
    /// imported name that names an entity; the `use` statement it is on;
    /// the declaration or reference token under it (navigation's
    /// occurrence, token-precise); an identifier or scheme ref ID at a
    /// reference position that names an entity of the graph; the field
    /// whose name it is on. `None` otherwise: a word in a string or a
    /// comment, a kind keyword, a keyword and a value of a field typed as
    /// no reference name nothing. `file` is the document's session key (as
    /// spans name it).
    pub fn target<F: Fn(&str) -> Option<String>>(
        &self,
        nav: &Navigator<'_, F>,
        file: &str,
    ) -> Option<Target> {
        let view = nav.view();
        let word = self.word_index();
        let entity = |i: usize| {
            let lexeme = self.syntax.lexemes[i];
            let id = lexeme.text(self.text);
            view.graph.node(id)?;
            Some(Target::Entity {
                id: Sym::new(id),
                origin: Range {
                    start: self.index.position(lexeme.start),
                    end: self.index.position(lexeme.end),
                },
            })
        };
        if let Some(word) = word
            && self.syntax.binding[word]
            && let Some(target) = entity(word)
        {
            return Some(target);
        }
        if let Some(path) = self.import() {
            return Some(Target::Import {
                path: path.to_string(),
            });
        }
        if self.place() != Place::Code {
            return None;
        }
        if let Some(occurrence) = self.occurrence(nav, file) {
            return Some(Target::Entity {
                id: occurrence.target,
                origin: self.index.range(&occurrence.span),
            });
        }
        let word = word?;
        let fields = &view.registries.fields;
        if self
            .syntax
            .reference_position(self.text, word as u32, fields)
            && let Some(target) = entity(word)
        {
            return Some(target);
        }
        if self.syntax.roles[word] == Role::Key
            && let Some(kind) = self.syntax.kind_of_key(self.text, word as u32)
        {
            return Some(Target::Field {
                kind: kind.to_string(),
                field: self.syntax.text(self.text, word as u32).to_string(),
            });
        }
        None
    }
}

/// Whether a string lexeme is closed: it ends with its own quotes.
fn closed(string: &str) -> bool {
    if let Some(body) = string.strip_prefix("\"\"\"") {
        return body.ends_with("\"\"\"");
    }
    let Some(body) = string.strip_prefix('"') else {
        return false;
    };
    let Some(inner) = body.strip_suffix('"') else {
        return false;
    };
    // An escaped quote does not close it: count the backslashes before it.
    inner.bytes().rev().take_while(|b| *b == b'\\').count() % 2 == 0
}
