//! The ids of `@code/core`'s facts (ADR 0347 D2–D5): a test, an item and a package, each parsing back to itself.

use std::fmt;
use std::str::FromStr;

/// What a run id starts with, before its test's id.
const RUN: &str = "run:";
/// What a `js` id's `<file>` percent-encodes: `#` ends the container of a test id, `:` starts an item's names.
const FILE_ESCAPES: &[char] = &['%', '#', ':'];
/// What a JavaScript name segment percent-encodes: `.` joins members, a private name starts with `#`.
const NAME_ESCAPES: &[char] = &['%', '#', '.', ':'];
/// What a crate segment's `<dir>` percent-encodes: `@` starts it.
const DIR_ESCAPES: &[char] = &['%', '#', ':', '@'];
/// What a purl subpath percent-encodes.
const SUBPATH_ESCAPES: &[char] = &['%', '#', '?', ' '];

/// An id that does not parse: what was expected of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdError(String);

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for IdError {}

/// The scheme of a test or item id: the language its pack reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Scheme {
    Js,
    Rust,
}

impl Scheme {
    pub const fn as_str(self) -> &'static str {
        match self {
            Scheme::Js => "js",
            Scheme::Rust => "rust",
        }
    }

    fn parse(text: &str) -> Result<Scheme, IdError> {
        match text {
            "js" => Ok(Scheme::Js),
            "rust" => Ok(Scheme::Rust),
            other => Err(IdError(format!(
                "'{other}' is no scheme of a test or item: js or rust"
            ))),
        }
    }

    /// What joins the segments of a test path: a JavaScript suite path, a libtest module path.
    fn test_separator(self) -> &'static str {
        match self {
            Scheme::Js => " > ",
            Scheme::Rust => "::",
        }
    }
}

/// A test's id (ADR 0329 D2, ADR 0347 D4): `js:<package>/<file>#<s1 > … > name>` or
/// `rust:<nextest binary id>#<test path>`. The container ends at the first `#`: a `js` file has it encoded, a package
/// name and a binary id never hold one.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TestId {
    scheme: Scheme,
    container: String,
    path: Vec<String>,
}

impl TestId {
    /// The test `path` (suites outermost first, then the name) of the file `file` under the JavaScript package
    /// `package` (its name, `@scope/` kept).
    pub fn js(package: &str, file: &str, path: &[String]) -> TestId {
        TestId {
            scheme: Scheme::Js,
            container: format!("{package}/{}", escape(file, FILE_ESCAPES)),
            path: path.to_vec(),
        }
    }

    /// The test `path` (libtest's: `mod::tests::name`, no crate) of the nextest binary `binary_id`.
    pub fn rust(binary_id: &str, path: &str) -> TestId {
        TestId {
            scheme: Scheme::Rust,
            container: binary_id.to_string(),
            path: path.split("::").map(str::to_string).collect(),
        }
    }

    pub fn scheme(&self) -> Scheme {
        self.scheme
    }

    /// The id of the run of this test: `run:<test id>`.
    pub fn run(&self) -> String {
        format!("{RUN}{self}")
    }

    /// The test whose run is `run` (`run:<test id>`): the inverse of [`TestId::run`].
    pub fn of_run(run: &str) -> Result<TestId, IdError> {
        run.strip_prefix(RUN)
            .ok_or_else(|| IdError(format!("'{run}' is no run id: run:<test id>")))?
            .parse()
    }

    /// `<scheme>:<container>#`: what every test of one file (`js`) or binary (`rust`) starts with.
    pub fn container(&self) -> String {
        format!("{}:{}#", self.scheme.as_str(), self.container)
    }

    /// `run:<scheme>:<container>#`: what the run of every test of one file or binary starts with.
    pub fn run_container(&self) -> String {
        format!("{RUN}{}", self.container())
    }

    /// The package the test is in: the JavaScript package, or the cargo package of the binary.
    pub fn package(&self) -> &str {
        match self.scheme {
            Scheme::Rust => self
                .container
                .split_once("::")
                .map_or(self.container.as_str(), |(package, _)| package),
            Scheme::Js => {
                let skip = usize::from(self.container.starts_with('@'));
                let end = self
                    .container
                    .match_indices('/')
                    .nth(skip)
                    .map_or(self.container.len(), |(at, _)| at);
                &self.container[..end]
            }
        }
    }

    /// The nextest binary id, for a Rust test.
    pub fn binary(&self) -> Option<&str> {
        (self.scheme == Scheme::Rust).then_some(self.container.as_str())
    }

    /// The suites (JavaScript) or modules (Rust) the test is nested in, outermost first.
    pub fn suite(&self) -> &[String] {
        &self.path[..self.path.len().saturating_sub(1)]
    }

    /// The test's own name: its path's last segment.
    pub fn name(&self) -> &str {
        self.path.last().map_or("", String::as_str)
    }

    /// The path written as the id writes it: `board > adds`, `tests::adds`.
    pub fn path(&self) -> String {
        self.path.join(self.scheme.test_separator())
    }
}

impl fmt::Display for TestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}#{}",
            self.scheme.as_str(),
            self.container,
            self.path()
        )
    }
}

impl FromStr for TestId {
    type Err = IdError;

    fn from_str(id: &str) -> Result<TestId, IdError> {
        let (scheme, rest) = id
            .split_once(':')
            .ok_or_else(|| IdError(format!("'{id}' has no scheme")))?;
        let scheme = Scheme::parse(scheme)?;
        let (container, path) = rest
            .split_once('#')
            .ok_or_else(|| IdError(format!("'{id}' has no '#' before its test path")))?;
        if container.is_empty() || path.is_empty() {
            return Err(IdError(format!(
                "'{id}' needs a container and a path around its '#'"
            )));
        }
        Ok(TestId {
            scheme,
            container: container.to_string(),
            path: path
                .split(scheme.test_separator())
                .map(str::to_string)
                .collect(),
        })
    }
}

/// The type a Rust member is defined on: its self type and, in a trait impl, the trait. Each is normalised as
/// [`ItemId::rust`] says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Owner {
    pub self_ty: String,
    pub trait_ty: Option<String>,
}

/// An item's id (ADR 0347 D4): `rust:<crate segment>::<mod>::…::<name>` (a member `…::<Type>::m` or
/// `…::<Type as Trait>::m`) or `js:<package>/<file>::<name>[.<member>…]`. It never holds `#`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemId {
    scheme: Scheme,
    container: String,
    path: Vec<String>,
}

impl ItemId {
    /// The Rust item `name` in the module path `module` (no crate) of the crate segment `krate` ([`crate_segment`]),
    /// defined on `owner` when it is a member. A raw identifier is written without `r#`. A path type keeps its last
    /// segment without generic arguments (`crate::a::Vec<T>` is `Vec`, `From<io::Error>` is `From`); any other type
    /// (`&T`, `[T]`, `(A, B)`, `dyn Tr`) is its text with whitespace collapsed (D3).
    pub fn rust(krate: &str, module: &[String], owner: Option<&Owner>, name: &str) -> ItemId {
        let mut path: Vec<String> = module.iter().map(|m| unraw(m).to_string()).collect();
        if let Some(owner) = owner {
            let self_ty = normal_type(&owner.self_ty);
            path.push(match &owner.trait_ty {
                Some(trait_ty) => format!("<{self_ty} as {}>", normal_type(trait_ty)),
                None => self_ty,
            });
        }
        path.push(unraw(name).to_string());
        ItemId {
            scheme: Scheme::Rust,
            container: krate.to_string(),
            path,
        }
    }

    /// The JavaScript item `path` (a name, then its members) of the file `file` under the package `package`.
    pub fn js(package: &str, file: &str, path: &[String]) -> ItemId {
        ItemId {
            scheme: Scheme::Js,
            container: format!("{package}/{}", escape(file, FILE_ESCAPES)),
            path: path.to_vec(),
        }
    }

    pub fn scheme(&self) -> Scheme {
        self.scheme
    }

    /// `rust:<crate segment>::` or `js:<package>/<file>::`: what every item of one crate or file starts with.
    pub fn container(&self) -> String {
        format!("{}:{}::", self.scheme.as_str(), self.container)
    }

    /// The item's own unqualified name: its last segment (`add` for `…::Board::add`, `fmt` for
    /// `…::<Foo as Display>::fmt`, `add` for `js:…::Board.add`).
    pub fn name(&self) -> &str {
        self.path.last().map_or("", String::as_str)
    }

    /// The id nextest gives this function when it is a test (D7), in a crate of `edition`: the lib's `<package>`,
    /// another target's `<package>::<target>` (an integration test) or `<package>::<kind>/<target>`, then `#` and its
    /// module path as libtest writes it, a word `edition` reserves raw (`r#match`). `None` for a JavaScript item, a
    /// member and a build script's item.
    pub fn test_id(&self, edition: Edition) -> Option<TestId> {
        if self.scheme != Scheme::Rust || self.path.iter().any(|s| s.starts_with('<')) {
            return None;
        }
        let krate = self
            .container
            .split_once('@')
            .map_or(self.container.as_str(), |(krate, _)| krate);
        let binary = match krate.splitn(3, '/').collect::<Vec<_>>()[..] {
            [package] => package.to_string(),
            [package, word, target] => match TargetKind::of_word(word)? {
                TargetKind::Test => format!("{package}::{target}"),
                TargetKind::Bin | TargetKind::Example | TargetKind::Bench => {
                    format!("{package}::{word}/{target}")
                }
                TargetKind::Lib | TargetKind::CustomBuild => return None,
            },
            _ => return None,
        };
        let path: Vec<String> = self.path.iter().map(|s| edition.written(s)).collect();
        Some(TestId::rust(&binary, &path.join("::")))
    }
}

impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.scheme.as_str(), self.container)?;
        match self.scheme {
            Scheme::Rust => self
                .path
                .iter()
                .try_for_each(|segment| write!(f, "::{segment}")),
            Scheme::Js => {
                let names: Vec<String> = self
                    .path
                    .iter()
                    .map(|name| escape(name, NAME_ESCAPES))
                    .collect();
                write!(f, "::{}", names.join("."))
            }
        }
    }
}

impl FromStr for ItemId {
    type Err = IdError;

    fn from_str(id: &str) -> Result<ItemId, IdError> {
        let (scheme, rest) = id
            .split_once(':')
            .ok_or_else(|| IdError(format!("'{id}' has no scheme")))?;
        let scheme = Scheme::parse(scheme)?;
        if rest.contains('#') {
            return Err(IdError(format!(
                "'{id}' holds '#': an item id never does (a test id does)"
            )));
        }
        let segments = top_level_split(rest);
        let (container, path) = match scheme {
            Scheme::Rust => segments
                .split_first()
                .map(|(container, path)| (container.to_string(), path.to_vec()))
                .ok_or_else(|| IdError(format!("'{id}' names no crate")))?,
            Scheme::Js => {
                let (container, names) = rest
                    .split_once("::")
                    .ok_or_else(|| IdError(format!("'{id}' has no '::' before its name")))?;
                let names = names.split('.').map(unescape).collect();
                (container.to_string(), names)
            }
        };
        if container.is_empty() || path.is_empty() || path.iter().any(String::is_empty) {
            return Err(IdError(format!("'{id}' has an empty segment")));
        }
        Ok(ItemId {
            scheme,
            container,
            path,
        })
    }
}

/// A package's id (ADR 0347 D5): a purl without version, `pkg:cargo/<name>`, `pkg:cargo/<name>#<dir>` (a second
/// package of one name) or `pkg:npm/<name>` (a scope's `@` encoded: `pkg:npm/%40scope/name`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageId {
    kind: PackageKind,
    name: String,
    dir: Option<String>,
}

/// The purl type of a package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum PackageKind {
    Cargo,
    Npm,
}

impl PackageId {
    /// The cargo package `name`: `pkg:cargo/<name>`.
    pub fn cargo(name: &str) -> PackageId {
        PackageId {
            kind: PackageKind::Cargo,
            name: name.to_string(),
            dir: None,
        }
    }

    /// The cargo package `name` at the project-relative directory `dir`, a second package of that name:
    /// `pkg:cargo/<name>#<dir>`, the purl subpath.
    pub fn cargo_in(name: &str, dir: &str) -> PackageId {
        PackageId {
            dir: Some(dir.to_string()),
            ..PackageId::cargo(name)
        }
    }

    /// The npm package `name`, `@scope/` kept: `pkg:npm/<name>`.
    pub fn npm(name: &str) -> PackageId {
        PackageId {
            kind: PackageKind::Npm,
            name: name.to_string(),
            dir: None,
        }
    }

    /// The package's name, as its manifest gives it.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The directory of a second package of one name.
    pub fn dir(&self) -> Option<&str> {
        self.dir.as_deref()
    }

    /// The purl type: `cargo` or `npm`, the ecosystem whose manifest pack writes the package.
    pub fn purl_type(&self) -> &'static str {
        match self.kind {
            PackageKind::Cargo => "cargo",
            PackageKind::Npm => "npm",
        }
    }
}

impl fmt::Display for PackageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            PackageKind::Cargo => write!(f, "pkg:cargo/{}", self.name)?,
            PackageKind::Npm => match self.name.strip_prefix('@') {
                Some(scoped) => write!(f, "pkg:npm/%40{scoped}")?,
                None => write!(f, "pkg:npm/{}", self.name)?,
            },
        }
        match &self.dir {
            Some(dir) => write!(f, "#{}", escape(dir, SUBPATH_ESCAPES)),
            None => Ok(()),
        }
    }
}

impl FromStr for PackageId {
    type Err = IdError;

    fn from_str(id: &str) -> Result<PackageId, IdError> {
        let rest = id
            .strip_prefix("pkg:")
            .ok_or_else(|| IdError(format!("'{id}' is no purl: pkg:<type>/<name>")))?;
        let (rest, dir) = match rest.split_once('#') {
            Some((rest, dir)) => (rest, Some(unescape(dir))),
            None => (rest, None),
        };
        let (kind, name) = match rest.split_once('/') {
            Some(("cargo", name)) => (PackageKind::Cargo, name.to_string()),
            Some(("npm", name)) => (
                PackageKind::Npm,
                match name.strip_prefix("%40") {
                    Some(scoped) => format!("@{scoped}"),
                    None => name.to_string(),
                },
            ),
            _ => {
                return Err(IdError(format!(
                    "'{id}' is no cargo or npm purl: pkg:cargo/<name>, pkg:npm/<name>"
                )))
            }
        };
        if name.is_empty() || dir.as_deref() == Some("") {
            return Err(IdError(format!("'{id}' has an empty name or subpath")));
        }
        Ok(PackageId { kind, name, dir })
    }
}

/// A cargo target's kind, as a crate segment and a nextest binary id qualify it (D2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TargetKind {
    /// Every library crate type: `lib`, `rlib`, `dylib`, `cdylib`, `staticlib`, `proc-macro`.
    Lib,
    Bin,
    Test,
    Example,
    Bench,
    /// A build script.
    CustomBuild,
}

/// Each target kind's word in a crate segment.
const TARGET_WORDS: [(TargetKind, &str); 6] = [
    (TargetKind::Lib, "lib"),
    (TargetKind::Bin, "bin"),
    (TargetKind::Test, "test"),
    (TargetKind::Example, "example"),
    (TargetKind::Bench, "bench"),
    (TargetKind::CustomBuild, "custom-build"),
];

impl TargetKind {
    /// The kind of a target cargo metadata gives the kind `kind`: every library crate type is a [`TargetKind::Lib`].
    pub fn cargo(kind: &str) -> Result<TargetKind, IdError> {
        match kind {
            "rlib" | "dylib" | "cdylib" | "staticlib" | "proc-macro" => Ok(TargetKind::Lib),
            _ => TargetKind::of_word(kind).ok_or_else(|| {
                IdError(format!(
                    "'{kind}' is no cargo target kind: lib, rlib, dylib, cdylib, staticlib, proc-macro, bin, \
                     test, example, bench or custom-build"
                ))
            }),
        }
    }

    /// The word a crate segment writes for it.
    pub fn word(self) -> &'static str {
        TARGET_WORDS
            .iter()
            .find(|(kind, _)| *kind == self)
            .map_or("lib", |(_, word)| word)
    }

    fn of_word(word: &str) -> Option<TargetKind> {
        TARGET_WORDS
            .iter()
            .find(|(_, w)| *w == word)
            .map(|(kind, _)| *kind)
    }
}

/// A Rust edition: which words a crate reserves, so which names libtest writes raw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Edition {
    E2015,
    E2018,
    E2021,
    E2024,
}

/// The words every edition reserves that may be written raw (`crate`, `self`, `Self`, `super` and `_` may not).
const RESERVED: &[&str] = &[
    "abstract", "as", "become", "box", "break", "const", "continue", "do", "else", "enum",
    "extern", "false", "final", "fn", "for", "if", "impl", "in", "let", "loop", "macro", "match",
    "mod", "move", "mut", "override", "priv", "pub", "ref", "return", "static", "struct", "trait",
    "true", "type", "typeof", "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

impl Edition {
    /// The edition a manifest's `edition` names; none is 2015, cargo's default.
    pub fn cargo(edition: Option<&str>) -> Result<Edition, IdError> {
        match edition {
            None | Some("2015") => Ok(Edition::E2015),
            Some("2018") => Ok(Edition::E2018),
            Some("2021") => Ok(Edition::E2021),
            Some("2024") => Ok(Edition::E2024),
            Some(other) => Err(IdError(format!(
                "'{other}' is no Rust edition: 2015, 2018, 2021 or 2024"
            ))),
        }
    }

    /// Whether this edition reserves `word`: libtest then writes it raw.
    fn reserves(self, word: &str) -> bool {
        RESERVED.contains(&word)
            || (self >= Edition::E2018 && ["async", "await", "dyn", "try"].contains(&word))
            || (self >= Edition::E2024 && word == "gen")
    }

    /// `name` as rustc prints an identifier of this edition: `r#` before a reserved word.
    fn written(self, name: &str) -> String {
        match self.reserves(name) {
            true => format!("r#{name}"),
            false => name.to_string(),
        }
    }
}

/// The crate segment of the target `target` of kind `kind` of `package` (D2): `<name>` for a library,
/// `<name>/custom-build` for a build script, `<name>/<kind>/<target>` for `bin`, `test`, `example` and `bench`; a
/// [`PackageId::cargo_in`] package appends `@<dir>`.
pub fn crate_segment(package: &PackageId, kind: TargetKind, target: &str) -> String {
    let name = package.name();
    let base = match kind {
        TargetKind::Lib => name.to_string(),
        TargetKind::CustomBuild => format!("{name}/{}", kind.word()),
        _ => format!("{name}/{}/{target}", kind.word()),
    };
    match package.dir() {
        Some(dir) => format!("{base}@{}", escape(dir, DIR_ESCAPES)),
        None => base,
    }
}

/// `text` with each of `escapes` percent-encoded as `%XX` (uppercase hex of its UTF-8 bytes).
fn escape(text: &str, escapes: &[char]) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if escapes.contains(&c) {
            let mut bytes = [0; 4];
            for byte in c.encode_utf8(&mut bytes).bytes() {
                out.push_str(&format!("%{byte:02X}"));
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// `text` with each `%XX` decoded; a `%` that starts no valid sequence is kept.
fn unescape(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let hex = bytes
            .get(at + 1..at + 3)
            .and_then(|pair| std::str::from_utf8(pair).ok())
            .and_then(|pair| u8::from_str_radix(pair, 16).ok());
        match (bytes[at], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                at += 3;
            }
            (byte, _) => {
                out.push(byte);
                at += 1;
            }
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| text.to_string())
}

/// A Rust identifier without its `r#`.
fn unraw(name: &str) -> &str {
    name.strip_prefix("r#").unwrap_or(name)
}

/// A type as an id writes it (D3): a path type's last segment without generic arguments, any other its text with
/// whitespace collapsed.
fn normal_type(text: &str) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let (path, generics) = match collapsed.find('<') {
        Some(open) => (collapsed[..open].trim_end(), Some(&collapsed[open..])),
        None => (collapsed.as_str(), None),
    };
    let generics_close = generics.is_none_or(|g| g.ends_with('>') && closes_at_end(g));
    let is_path = !path.is_empty()
        && path
            .trim_start_matches("::")
            .split("::")
            .all(|segment| !segment.is_empty() && segment.chars().all(identifier_char));
    if is_path && generics_close {
        unraw(path.rsplit("::").next().unwrap_or(path)).to_string()
    } else {
        collapsed
    }
}

/// Whether `generics`, starting with `<`, closes its first `<` at its last character.
fn closes_at_end(generics: &str) -> bool {
    let mut depth = 0usize;
    for (at, c) in generics.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return at == generics.len() - 1;
                }
            }
            _ => {}
        }
    }
    false
}

fn identifier_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '#'
}

/// `text` split at each `::` outside `<…>`, `[…]` and `(…)`: an owner's text keeps its own `::`.
fn top_level_split(text: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    let bytes = text.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'<' | b'[' | b'(' => depth += 1,
            b'>' | b']' | b')' => depth = depth.saturating_sub(1),
            b':' if depth == 0 && bytes.get(at + 1) == Some(&b':') => {
                segments.push(text[start..at].to_string());
                at += 2;
                start = at;
                continue;
            }
            _ => {}
        }
        at += 1;
    }
    segments.push(text[start..].to_string());
    segments
}
