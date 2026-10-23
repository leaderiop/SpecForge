# A body statement is read by one rule

**Status:** accepted (2026-10-23, architecture round 26, plan 12). Extends
[ADR 0023](0023-the-lsp-reads-a-document-through-one-module.md) D4 and amends its D1 (the lexer's divergences from the
grammar).

## Context

The grammar decides "verify statement or field named verify" with one precedence rule: a statement needs a regular
`"..."` string after `verify`, directly or after one kind word; anything else is a field. The LSP's lexeme walk kept a
private copy of that rule, and the copy took any string (a triple-quoted one too) and "`verify` and one word and
nothing else on the line" as a statement. The editor showed `verify """x"""`, `verify string` and `verify true`
fields as statements and offered verify kinds in them. The agreement test ran over the corpus only, which writes a
field named verify once, annotated, the one form the copy gets right. The same concept had been fixed module by
module six times (registry, emitter, ops, hover); the walk was the last text-level reader.

```text
BEFORE  walk: is_verify_statement (any Str, ident+nothing -> statement)   held to nothing
AFTER   lex::body_statement_at -> Verify | VerifyTyped | Method | Field    held to the grammar
```

## Decision

- **D1. One rule in the lexer**, beside `form_starts`: `specforge_parser::lex::body_statement_at` (and
  `BodyStatement`). The walk asks it and holds no statement rule; `is_verify_statement` and `is_method_statement`
  are deleted.
- **D2. Its text rule.** `verify` starts a statement when a regular string follows, directly or after one identifier;
  `method` when an identifier follows on its line, then `(` or nothing. A triple-quoted string is a field's value,
  also after a kind.
- **D3. A verify line being typed** (`verify`, or `verify` and one identifier, alone on its line, no regular string
  next) is `VerifyTyped`. The document classifies it as the grammar reads it, a field. Only completion on the cursor's
  line reads it as the statement it is becoming (verify kinds; nothing after the kind). The highlight of a verify
  statement therefore turns from property to keyword when its description's quote is typed, as the compile's W020
  already agrees.
- **D4. A kind is read on its line**, with one look past the line for the description. The grammar reads across line
  breaks (a bare `verify` above `contract "c"` swallows that field, E006); the rule does not.
- **D5. Two agreement tests.** The parser's `body_statements_are_the_grammars` and the LSP's
  `tokens_agree_with_the_grammar_on_the_forms_the_corpus_does_not_write`, over the repository's spec and the lexemes
  fixture `crates/kernel/parser/tests/fixtures/lexemes.spec`.

## Rejected

A registry-aware rule (the walk names no kind, ADR 0023 D4); a document that reads the statement it is becoming
everywhere (the highlight would disagree with the grammar on a field named verify); a second walk per completion;
reading across lines (the grammar's swallowing of the next field).

## What would reopen it

A grammar change to `field`, `verify_statement` or `method_statement` (the tests fail first); the grammar adopting the
on-its-line reading (D4 stops being a divergence).
