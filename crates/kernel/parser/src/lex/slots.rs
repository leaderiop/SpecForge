//! The slot reader (ADR 0283): where the grammar reads one bare word, the lexer reads one lexeme.
//!
//! The base lexer cuts `AUTH-01` into `AUTH`, `-`, `01`. The grammar reads it whole in four places. The
//! lexer's agreement with the grammar is `lexer_agrees_with_the_grammar` over the corpus, and
//! `the_lexer_agrees_with_the_grammar_on_every_edited_form` over the forms a random edit leaves intact.
//!
//! | slot | where | the word |
//! |---|---|---|
//! | header name | after the kind word of a top-level form, unless it is a union (`kind name ["title"] =`) | `Ident` |
//! | value | the first lexeme after a key in a body or nested block, unless it opens a string, `[`, `{` or `expr {` | `Number` when it has the shape of a number or a date, else `Ident` |
//! | list item | after `[` or `,` of a value list | as a value |
//! | binding | after `{` or `,` of `use { }` | `Ident` |
//!
//! Everywhere else (`expr { }`, union variants, type expressions, keys, verify kinds, methods, parameters and
//! annotations) the base lexemes stand: `a-b` is `a`, `-`, `b`.

use super::{
    BodyStatement, Lexeme, LexemeKind, body_statement_at, form_starts, is_number_word, name_len,
    word_run,
};

/// What a `{` or `[` opened.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Frame {
    /// A block's body: members are keys and values, `verify` and `method` statements.
    Body,
    /// A nested block value: members are keys and values.
    Block,
    /// A value list.
    List,
    /// An `expr { }` group.
    Expr,
    /// A selective import's bindings.
    Use,
}

/// The open frames, each with whether its next lexeme is a slot: a value in a body or nested block, an item in
/// a list, a binding in `use { }`.
#[derive(Default)]
struct Open {
    frames: Vec<(Frame, bool)>,
}

impl Open {
    fn top(&self) -> Option<Frame> {
        self.frames.last().map(|(frame, _)| *frame)
    }

    fn slot(&self) -> bool {
        self.frames.last().is_some_and(|(_, slot)| *slot)
    }

    fn set_slot(&mut self, to: bool) {
        if let Some((_, slot)) = self.frames.last_mut() {
            *slot = to;
        }
    }

    fn push(&mut self, frame: Frame, slot: bool) {
        self.frames.push((frame, slot));
    }

    /// Close frames through the nearest one `matches` says is the closer's.
    fn close(&mut self, matches: impl Fn(Frame) -> bool) {
        while let Some((frame, _)) = self.frames.pop() {
            if matches(frame) {
                break;
            }
        }
    }
}

struct Reader<'t> {
    text: &'t str,
    base: Vec<Lexeme>,
    out: Vec<Lexeme>,
    /// The index of the next lexeme to read.
    at: usize,
}

/// `base` (the lexemes of `text` as the base lexer cuts them) with the words of the slots above joined.
pub(super) fn read_words(text: &str, base: Vec<Lexeme>) -> Vec<Lexeme> {
    let starts = form_starts(text, &base);
    let mut reader = Reader {
        text,
        out: Vec::with_capacity(base.len()),
        base,
        at: 0,
    };
    reader.read(&starts);
    reader.out
}

impl Reader<'_> {
    fn kind(&self, i: usize) -> Option<LexemeKind> {
        self.base.get(i).map(|l| l.kind)
    }

    fn punct(&self, i: usize, c: char) -> bool {
        self.kind(i) == Some(LexemeKind::Punct(c))
    }

    fn word_text(&self, i: usize) -> &str {
        self.base[i].text(self.text)
    }

    /// The first lexeme from `i` on that is no comment.
    fn code(&self, mut i: usize) -> Option<usize> {
        while self.kind(i) == Some(LexemeKind::Comment) {
            i += 1;
        }
        (i < self.base.len()).then_some(i)
    }

    /// Copy the lexeme at `self.at` and move on.
    fn copy(&mut self) {
        self.out.push(self.base[self.at]);
        self.at += 1;
    }

    /// Copy every lexeme up to and including the one at `to`.
    fn copy_through(&mut self, to: usize) {
        while self.at <= to {
            self.copy();
        }
    }

    /// Whether the `/` at `i` joins the word run right after it with no gap: a path's separator (or its root).
    fn joins(&self, i: usize) -> bool {
        self.punct(i, '/')
            && self
                .base
                .get(i + 1)
                .is_some_and(|n| n.start == self.base[i].end)
            && word_run(self.text, &self.base, i + 1, false).is_some()
    }

    /// Read the word starting at `self.at`, or the path of names joined by adjacent `/` (ADR 0284), a leading `/`
    /// rooting it. A word of a path is an `Ident`, as the grammar's `reference_path` reads it. Returns whether it
    /// was one identifier lexeme, which a type expression may start with.
    fn word(&mut self, name: bool) -> bool {
        let rooted = self.joins(self.at);
        if rooted {
            self.copy();
        }
        let joined = |me: &Self| {
            word_run(me.text, &me.base, me.at, !name).is_some_and(|(len, _)| me.joins(me.at + len))
        };
        let path = rooted || joined(self);
        let pure = self.segment(name || path);
        if !path {
            return pure;
        }
        while self.joins(self.at)
            && self
                .out
                .last()
                .is_some_and(|l| l.end == self.base[self.at].start)
        {
            self.copy();
            self.segment(true);
        }
        false
    }

    /// Read the word starting at `self.at` as one lexeme when the base lexemes spell one, whole; else copy
    /// one lexeme. `name`: a header, binding or path name (an `Ident` whatever its shape, no leading `-`).
    fn segment(&mut self, name: bool) -> bool {
        let pure = self.kind(self.at) == Some(LexemeKind::Ident);
        let Some((len, end)) = word_run(self.text, &self.base, self.at, !name) else {
            self.copy();
            return pure;
        };
        let start = self.base[self.at].start;
        let kind = if name || !is_number_word(&self.text[start..end]) {
            LexemeKind::Ident
        } else {
            LexemeKind::Number
        };
        self.out.push(Lexeme { start, end, kind });
        self.at += len;
        pure && len == 1
    }

    /// Whether the lexeme at `self.at` can start a bare word or a path.
    fn starts_word(&self) -> bool {
        matches!(
            self.kind(self.at),
            Some(LexemeKind::Ident | LexemeKind::Number | LexemeKind::Punct('-'))
        ) || self.joins(self.at)
    }

    fn read(&mut self, starts: &[usize]) {
        let mut open = Open::default();
        // Whether the form being read is a `use`, whose `{` opens bindings.
        let mut importing = false;
        let mut next_start = 0;
        while self.at < self.base.len() {
            let kind = self.base[self.at].kind;
            if kind == LexemeKind::Comment {
                self.copy();
                continue;
            }
            if starts.get(next_start) == Some(&self.at) {
                next_start += 1;
                open = Open::default();
                importing = false;
                self.header(&mut importing);
                continue;
            }
            match (open.top(), kind) {
                (_, LexemeKind::Punct('}')) => {
                    open.close(|frame| frame != Frame::List);
                    if open.top().is_none() {
                        importing = false;
                    }
                    self.copy();
                }
                (_, LexemeKind::Punct(']'))
                    if open.frames.iter().any(|(f, _)| *f == Frame::List) =>
                {
                    open.close(|frame| frame == Frame::List);
                    self.copy();
                }
                (None, LexemeKind::Punct('{')) => {
                    open.push(if importing { Frame::Use } else { Frame::Body }, importing);
                    self.copy();
                }
                (None, LexemeKind::Str { .. }) => {
                    // The import's path ends it.
                    importing = false;
                    self.copy();
                }
                (None | Some(Frame::Expr), _) => self.copy(),
                (Some(Frame::Use), _) => self.binding(&mut open),
                (Some(Frame::List), _) => {
                    if open.slot() && self.starts_word() {
                        self.word(false);
                        open.set_slot(false);
                    } else {
                        match kind {
                            LexemeKind::Punct(',') => open.set_slot(true),
                            LexemeKind::Str { .. } | LexemeKind::RefId => open.set_slot(false),
                            _ => {}
                        }
                        self.copy();
                    }
                }
                (Some(frame @ (Frame::Body | Frame::Block)), _) => {
                    if open.slot() {
                        self.value(&mut open);
                    } else {
                        self.member(frame, &mut open);
                    }
                }
            }
        }
    }

    /// The header of a form starting at `self.at`: its kind word, then (for an entity) its name.
    fn header(&mut self, importing: &mut bool) {
        let ident = self.kind(self.at) == Some(LexemeKind::Ident);
        let word = self.word_text(self.at).to_string();
        self.copy();
        if !ident {
            return;
        }
        match word.as_str() {
            "use" => *importing = true,
            "pub" => {
                if let Some(next) = self.code(self.at)
                    && self.kind(next) == Some(LexemeKind::Ident)
                    && self.word_text(next) == "use"
                {
                    self.copy_through(next);
                    *importing = true;
                }
            }
            "spec" | "ref" | "define" => {}
            _ => {
                let Some(name) = self.code(self.at) else {
                    return;
                };
                self.copy_through(name - 1);
                let Some(len) = name_len(self.text, &self.base, name) else {
                    return;
                };
                // A union's name is an identifier, as the grammar reads it: the base lexemes stand.
                let mut after = name + len;
                if matches!(self.kind(after), Some(LexemeKind::Str { .. })) {
                    after += 1;
                }
                if !self.punct(after, '=') {
                    self.word(true);
                }
            }
        }
    }

    /// A binding of `use { }`, or what follows one.
    fn binding(&mut self, open: &mut Open) {
        if open.slot() && self.starts_word() {
            self.word(true);
            open.set_slot(false);
            // `as alias`
            if let Some(a) = self.code(self.at)
                && self.kind(a) == Some(LexemeKind::Ident)
                && self.word_text(a) == "as"
            {
                self.copy_through(a);
                if let Some(b) = self.code(self.at)
                    && self.kind(b) == Some(LexemeKind::Ident)
                {
                    self.copy_through(b);
                }
            }
        } else {
            if self.punct(self.at, ',') {
                open.set_slot(true);
            }
            self.copy();
        }
    }

    /// A member of a body or nested block: its key, a statement, an annotation, or anything else.
    fn member(&mut self, frame: Frame, open: &mut Open) {
        match self.base[self.at].kind {
            LexemeKind::Ident | LexemeKind::Number => {
                if frame == Frame::Body {
                    match body_statement_at(self.text, &self.base, self.at) {
                        BodyStatement::Verify => {
                            self.copy();
                            if let Some(k) = self.code(self.at)
                                && self.kind(k) == Some(LexemeKind::Ident)
                            {
                                self.copy_through(k);
                            }
                            if let Some(s) = self.code(self.at) {
                                self.copy_through(s);
                            }
                            return;
                        }
                        BodyStatement::Method => {
                            self.method();
                            return;
                        }
                        BodyStatement::VerifyTyped | BodyStatement::Field => {}
                    }
                }
                self.copy();
                open.set_slot(true);
            }
            LexemeKind::Punct('@') => {
                // An annotation: `@name`, then maybe a string.
                self.copy();
                if let Some(n) = self.code(self.at)
                    && self.kind(n) == Some(LexemeKind::Ident)
                {
                    self.copy_through(n);
                    if let Some(s) = self.code(self.at)
                        && matches!(self.kind(s), Some(LexemeKind::Str { .. }))
                    {
                        self.copy_through(s);
                    }
                }
            }
            _ => self.copy(),
        }
    }

    /// `method name(params) -> Type`: nothing in it is a slot.
    fn method(&mut self) {
        self.copy();
        if let Some(name) = self.code(self.at) {
            self.copy_through(name);
        }
        if let Some(open) = self.code(self.at)
            && self.punct(open, '(')
        {
            self.copy_through(open);
            let mut depth = 1usize;
            while depth > 0 && self.at < self.base.len() {
                match self.kind(self.at) {
                    Some(LexemeKind::Punct('(')) => depth += 1,
                    Some(LexemeKind::Punct(')')) => depth -= 1,
                    _ => {}
                }
                self.copy();
            }
        }
        if let Some(arrow) = self.code(self.at)
            && self.punct(arrow, '-')
            && self.punct(arrow + 1, '>')
        {
            self.copy_through(arrow + 1);
            self.type_tail(true);
        }
    }

    /// A field's value, at `self.at`.
    fn value(&mut self, open: &mut Open) {
        open.set_slot(false);
        match self.base[self.at].kind {
            LexemeKind::Str { .. } => self.copy(),
            LexemeKind::Punct('[') => {
                self.copy();
                open.push(Frame::List, true);
            }
            LexemeKind::Punct('{') => {
                self.copy();
                open.push(Frame::Block, false);
            }
            // A key with no value: the brace closes the block, which the loop reads next.
            LexemeKind::Punct('}') => {}
            LexemeKind::Ident
                if self.word_text(self.at) == "expr"
                    && self.code(self.at + 1).is_some_and(|b| self.punct(b, '{')) =>
            {
                let brace = self.code(self.at + 1).unwrap_or(self.at);
                self.copy_through(brace);
                open.push(Frame::Expr, false);
            }
            LexemeKind::Ident | LexemeKind::Number | LexemeKind::Punct('-' | '/') => {
                if self.word(false) {
                    self.type_tail(false);
                }
            }
            _ => self.copy(),
        }
    }

    /// What follows a value's first identifier when the value is a type expression: `| T`, `<A, B>`, `[]`.
    /// Nothing in it is a slot. With `always`, the type starts here whatever follows (a method's return).
    fn type_tail(&mut self, always: bool) {
        let mut started = always;
        let mut depth = 0usize;
        let mut previous = self.at.checked_sub(1);
        while let Some(next) = self.code(self.at) {
            let kind = self.base[next].kind;
            let adjacent = previous.is_some_and(|p| self.base[p].end == self.base[next].start);
            if !started {
                let begins = kind == LexemeKind::Punct('|')
                    || (adjacent && kind == LexemeKind::Punct('<'))
                    || (adjacent && kind == LexemeKind::Punct('[') && self.punct(next + 1, ']'));
                if !begins {
                    return;
                }
                started = true;
            } else if depth == 0 {
                let newline = previous.is_some_and(|p| {
                    self.text[self.base[p].end..self.base[next].start].contains('\n')
                });
                let continues = kind == LexemeKind::Punct('|')
                    || previous.is_some_and(|p| {
                        matches!(self.base[p].kind, LexemeKind::Punct('|' | '<' | ',' | '('))
                    });
                if (newline && !continues) || matches!(kind, LexemeKind::Punct('@' | '}' | ']')) {
                    return;
                }
            }
            let arrow = previous.is_some_and(|p| self.base[p].kind == LexemeKind::Punct('-'));
            match kind {
                LexemeKind::Punct('<' | '(') => depth += 1,
                LexemeKind::Punct('>') if !arrow => depth = depth.saturating_sub(1),
                LexemeKind::Punct(')') => depth = depth.saturating_sub(1),
                _ => {}
            }
            self.copy_through(next);
            previous = Some(next);
        }
    }
}
