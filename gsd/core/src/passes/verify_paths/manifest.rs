//! Reading a `package.json` without a JSON crate (the crate links `serde_json` only for its tests): just enough JSON
//! to tell a valid object from a broken file and to list the keys of its `scripts`.

enum Node {
    Object(Vec<(String, Node)>),
    Other,
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

/// The script names of a `package.json` text; `None` when the text is not a JSON object.
pub fn scripts(text: &str) -> Option<Vec<String>> {
    let mut reader = Reader {
        bytes: text.as_bytes(),
        at: 0,
    };
    let Node::Object(members) = reader.value(0)? else {
        return None;
    };
    reader.space();
    if reader.at != reader.bytes.len() {
        return None;
    }
    Some(
        members
            .into_iter()
            .find(|(name, _)| name == "scripts")
            .and_then(|(_, node)| match node {
                Node::Object(scripts) => Some(scripts.into_iter().map(|(n, _)| n).collect()),
                Node::Other => None,
            })
            .unwrap_or_default(),
    )
}

impl Reader<'_> {
    fn space(&mut self) {
        while self.bytes.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }

    fn eat(&mut self, byte: u8) -> Option<()> {
        self.space();
        (self.bytes.get(self.at) == Some(&byte)).then(|| self.at += 1)
    }

    fn value(&mut self, depth: usize) -> Option<Node> {
        self.space();
        if depth > 64 {
            return None;
        }
        match *self.bytes.get(self.at)? {
            b'{' => {
                self.at += 1;
                let mut members = Vec::new();
                if self.eat(b'}').is_some() {
                    return Some(Node::Object(members));
                }
                loop {
                    self.space();
                    let name = self.string()?;
                    self.eat(b':')?;
                    members.push((name, self.value(depth + 1)?));
                    if self.eat(b',').is_none() {
                        self.eat(b'}')?;
                        return Some(Node::Object(members));
                    }
                }
            }
            b'[' => {
                self.at += 1;
                if self.eat(b']').is_some() {
                    return Some(Node::Other);
                }
                loop {
                    self.value(depth + 1)?;
                    if self.eat(b',').is_none() {
                        self.eat(b']')?;
                        return Some(Node::Other);
                    }
                }
            }
            b'"' => self.string().map(|_| Node::Other),
            _ => self.word().then_some(Node::Other),
        }
    }

    /// `true`, `false`, `null` or a number (shortcut: the digits are not checked against JSON's grammar).
    fn word(&mut self) -> bool {
        let start = self.at;
        while self
            .bytes
            .get(self.at)
            .is_some_and(|b| b.is_ascii_alphanumeric() || b"+-.".contains(b))
        {
            self.at += 1;
        }
        let word = &self.bytes[start..self.at];
        matches!(word, b"true" | b"false" | b"null")
            || (!word.is_empty()
                && word
                    .iter()
                    .all(|b| b.is_ascii_digit() || b"+-.eE".contains(b)))
    }

    fn string(&mut self) -> Option<String> {
        self.space();
        (self.bytes.get(self.at) == Some(&b'"')).then_some(())?;
        self.at += 1;
        let mut out = Vec::new();
        loop {
            let byte = *self.bytes.get(self.at)?;
            self.at += 1;
            match byte {
                b'"' => return String::from_utf8(out).ok(),
                b'\\' => {
                    let escape = *self.bytes.get(self.at)?;
                    self.at += 1;
                    match escape {
                        b'"' | b'\\' | b'/' => out.push(escape),
                        b'b' | b'f' | b'n' | b'r' | b't' => out.push(b'?'),
                        b'u' => {
                            let hex =
                                std::str::from_utf8(self.bytes.get(self.at..self.at + 4)?).ok()?;
                            let code = u32::from_str_radix(hex, 16).ok()?;
                            self.at += 4;
                            let mut buf = [0; 4];
                            let ch = char::from_u32(code).unwrap_or('?');
                            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                        _ => return None,
                    }
                }
                0..=0x1f => return None,
                other => out.push(other),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::scripts;

    #[test]
    fn the_scripts_of_a_manifest_are_listed() {
        let text = r#"{ "name": "w", "n": [1, -2.5e3, true, null, {"a": []}], "scripts": { "test": "x \"y\"", "lint": "z" } }"#;
        assert_eq!(scripts(text), Some(vec!["test".into(), "lint".into()]));
        assert_eq!(scripts("{}"), Some(vec![]));
        assert_eq!(scripts(r#"{"scripts": "no"}"#), Some(vec![]));
    }

    #[test]
    fn a_broken_or_non_object_manifest_is_none() {
        for text in [
            "",
            "{ not json",
            "[]",
            "{\"a\": 1,}",
            "{\"a\": 1} x",
            "{\"a\": tru}",
        ] {
            assert_eq!(scripts(text), None, "{text}");
        }
    }
}
