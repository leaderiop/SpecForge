//! Who is in which scope (ADR 0284): the index the linker, the finder and every surface ask what a written name or
//! path names.
//!
//! A kind may be scoped by one of its reference fields; an entity's id is then its scope's path and its name
//! (`v1.0/03/D-01`), and a name is unique within its scope. An entity of a kind that is scoped by nothing, or one
//! that writes no scope, is in the project scope, and its id is its name.

use specforge_common::Sym;
use std::collections::{HashMap, HashSet};

/// What a written name or path names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    /// One entity of a kind the reference takes: its path.
    One(Sym),
    /// Several do, and only a surface's input or a scope field takes a unique suffix: their paths, sorted.
    Ambiguous(Vec<Sym>),
    /// The nearest entity it names is of another kind (E022): its path.
    Mistyped(Sym),
    /// None: the entities that bear the written name or path outside the scope chain (E003's candidates), sorted;
    /// empty when there are none.
    None(Vec<Sym>),
}

/// One placed entity.
#[derive(Debug, Clone, Copy)]
struct Placed {
    kind: Sym,
    scope: Option<Sym>,
}

/// Every placed entity by path, and each scope's members by name (`None`: the project). A copy is cheap to make and
/// to change, so a planned edit asks how the project would read after it ([`Self::moved`], [`Self::added`]).
#[derive(Debug, Clone, Default)]
pub struct Scopes {
    members: HashMap<Option<Sym>, HashMap<Sym, Sym>>,
    placed: HashMap<Sym, Placed>,
    /// Every path by its last segment, for the suffix step.
    by_name: HashMap<Sym, HashSet<Sym>>,
}

/// `path` without its scope's prefix.
fn name_in(path: Sym, scope: Option<Sym>) -> Sym {
    match scope {
        Some(scope) => path
            .as_str()
            .strip_prefix(scope.as_str())
            .and_then(|rest| rest.strip_prefix('/'))
            .map_or(path, Sym::new),
        None => path,
    }
}

impl Scopes {
    pub fn new() -> Self {
        Self::default()
    }

    /// Place the entity `path` of `kind`, in `scope` (`None`: the project).
    pub fn insert(&mut self, path: Sym, kind: Sym, scope: Option<Sym>) {
        self.remove(path);
        let name = name_in(path, scope);
        self.members.entry(scope).or_default().insert(name, path);
        self.by_name
            .entry(Sym::new(last_segment(path.as_str())))
            .or_default()
            .insert(path);
        self.placed.insert(path, Placed { kind, scope });
    }

    /// Take the entity `path` out, if placed.
    pub fn remove(&mut self, path: Sym) {
        let Some(placed) = self.placed.remove(&path) else {
            return;
        };
        let name = name_in(path, placed.scope);
        if let Some(members) = self.members.get_mut(&placed.scope) {
            if members.get(&name) == Some(&path) {
                members.remove(&name);
            }
            if members.is_empty() {
                self.members.remove(&placed.scope);
            }
        }
        let last = Sym::new(last_segment(path.as_str()));
        if let Some(paths) = self.by_name.get_mut(&last) {
            paths.remove(&path);
            if paths.is_empty() {
                self.by_name.remove(&last);
            }
        }
    }

    /// Whether an entity is placed at `path`.
    pub fn contains(&self, path: &str) -> bool {
        self.placed.contains_key(&Sym::new(path))
    }

    /// The kind of the entity at `path`.
    pub fn kind(&self, path: &str) -> Option<Sym> {
        self.placed.get(&Sym::new(path)).map(|p| p.kind)
    }

    /// The scope of the entity at `path` (`None`: the project, or no such entity).
    pub fn scope(&self, path: &str) -> Option<Sym> {
        self.placed.get(&Sym::new(path))?.scope
    }

    /// The name the entity at `path` is declared with: `path` without its scope's; `path` itself when none is placed
    /// there.
    pub fn name(&self, path: Sym) -> &str {
        match self.placed.get(&path) {
            Some(Placed {
                scope: Some(scope), ..
            }) => path
                .as_str()
                .strip_prefix(scope.as_str())
                .and_then(|rest| rest.strip_prefix('/'))
                .unwrap_or(path.as_str()),
            _ => path.as_str(),
        }
    }

    /// Every placed path, sorted.
    pub fn paths(&self) -> Vec<Sym> {
        let mut paths: Vec<Sym> = self.placed.keys().copied().collect();
        paths.sort();
        paths
    }

    /// The scopes a name is looked up in from `from`, nearest first: the holder itself (its members), each scope
    /// above it, the project. A rooted name, or none held (`from` `None`), is looked up in the project alone.
    fn levels(&self, from: Option<Sym>, rooted: bool) -> Vec<Option<Sym>> {
        let mut levels = Vec::new();
        let mut at = if rooted { None } else { from };
        while let Some(path) = at {
            if levels.len() > self.placed.len() {
                break;
            }
            levels.push(Some(path));
            at = self.placed.get(&path).and_then(|p| p.scope);
        }
        levels.push(None);
        levels
    }

    /// The entity `segments` name walking down from `level`.
    fn walk(&self, level: Option<Sym>, segments: &[Sym]) -> Option<Sym> {
        let mut scope = level;
        let mut found = None;
        for segment in segments {
            let path = *self.members.get(&scope)?.get(segment)?;
            found = Some(path);
            scope = Some(path);
        }
        found
    }

    /// `written` read from `from` by its scope chain alone, never by the suffix step: a leading `/` roots it at the
    /// project; a value holding `:` is a scheme ref id, looked up whole in the project.
    pub fn chain(&self, from: Option<Sym>, written: &str, admits: &dyn Fn(&str) -> bool) -> Found {
        if !written.contains(['/', ':']) {
            return self.chain_name(from, Sym::new(written), admits);
        }
        let (rooted, rest) = match written.strip_prefix('/') {
            Some(rest) => (true, rest),
            None => (false, written),
        };
        let whole = [rest];
        let split: Vec<&str> = rest.split('/').collect();
        let segments: &[&str] = if rest.contains(':') { &whole } else { &split };
        if segments.iter().any(|s| s.is_empty()) {
            return Found::None(Vec::new());
        }
        let segments: Vec<Sym> = segments.iter().map(|s| Sym::new(s)).collect();
        let mut mistyped = None;
        for level in self.levels(from, rooted) {
            let Some(path) = self.walk(level, &segments) else {
                continue;
            };
            if admits(self.placed[&path].kind.as_str()) {
                return Found::One(path);
            }
            mistyped.get_or_insert(path);
        }
        match mistyped {
            Some(path) => Found::Mistyped(path),
            None => Found::None(Vec::new()),
        }
    }

    /// [`Self::chain`] for one name.
    fn chain_name(&self, from: Option<Sym>, name: Sym, admits: &dyn Fn(&str) -> bool) -> Found {
        if name.as_str().is_empty() {
            return Found::None(Vec::new());
        }
        let mut mistyped = None;
        let mut at = from;
        loop {
            if let Some(&path) = self.members.get(&at).and_then(|m| m.get(&name)) {
                if admits(self.placed[&path].kind.as_str()) {
                    return Found::One(path);
                }
                mistyped.get_or_insert(path);
            }
            match at {
                None => break,
                Some(p) => at = self.placed.get(&p).and_then(|x| x.scope),
            }
        }
        mistyped.map_or(Found::None(Vec::new()), Found::Mistyped)
    }

    /// The paths that equal `written` or end with it on a segment boundary, of a kind `admits` takes, sorted.
    fn suffixed(&self, written: &str, admits: &dyn Fn(&str) -> bool) -> Vec<Sym> {
        if written.starts_with('/') || written.contains(':') {
            return Vec::new();
        }
        let Some(paths) = self.by_name.get(&Sym::new(last_segment(written))) else {
            return Vec::new();
        };
        let tail = format!("/{written}");
        let mut found: Vec<Sym> = paths
            .iter()
            .copied()
            .filter(|p| p.as_str() == written || p.as_str().ends_with(&tail))
            .filter(|p| admits(self.placed[p].kind.as_str()))
            .collect();
        found.sort();
        found
    }

    /// What `written` names from `from`, an entity holding it by its scope chain: its own members, each enclosing
    /// scope's, then the project's. Only with `from` `None` (a scope field, a surface's input; never a reference an
    /// entity holds) is a unique suffix taken when no level answers: `03` names `v1.0/03` when that is the one
    /// phase `03`. A miss names the entities that bear the written name elsewhere, as [`Found::None`] says.
    pub fn resolve(
        &self,
        from: Option<Sym>,
        written: &str,
        admits: &dyn Fn(&str) -> bool,
    ) -> Found {
        let found = self.chain(from, written, admits);
        if matches!(found, Found::One(_)) {
            return found;
        }
        let suffixed = self.suffixed(written, admits);
        if from.is_none() {
            match suffixed.as_slice() {
                [path] => return Found::One(*path),
                [_, _, ..] => return Found::Ambiguous(suffixed),
                [] => {}
            }
            if let Found::Mistyped(_) = found {
                return found;
            }
            return Found::None(Vec::new());
        }
        match found {
            Found::Mistyped(_) => found,
            _ => Found::None(suffixed),
        }
    }

    /// The shortest spelling of `target` that `from`'s scope chain answers with `target`, by the chain alone: a
    /// written spelling must not break when an unrelated scope gains a name. The name, then each longer suffix of
    /// the path, then `/` and the full path. Total: the rooted full path resolves at the project.
    pub fn spell(&self, from: Option<Sym>, target: Sym, admits: &dyn Fn(&str) -> bool) -> String {
        let path = target.as_str();
        if path.contains(':') {
            return path.to_string();
        }
        let segments: Vec<&str> = path.split('/').collect();
        for take in 1..=segments.len() {
            let text = segments[segments.len() - take..].join("/");
            if self.chain(from, &text, admits) == Found::One(target) {
                return text;
            }
        }
        format!("/{path}")
    }

    /// These scopes after the entities `moves` (old path, new path) are moved, each with the entities under it.
    pub fn moved(&self, moves: &[(Sym, Sym)]) -> Scopes {
        let at = |path: Sym| -> Sym {
            for (old, new) in moves {
                if path == *old {
                    return *new;
                }
                if let Some(rest) = path
                    .as_str()
                    .strip_prefix(old.as_str())
                    .and_then(|rest| rest.strip_prefix('/'))
                {
                    return Sym::new(&format!("{new}/{rest}"));
                }
            }
            path
        };
        let mut moved = Scopes::new();
        for (path, placed) in &self.placed {
            moved.insert(at(*path), placed.kind, placed.scope.map(at));
        }
        moved
    }

    /// These scopes with the entity `path` of `kind` added, in `scope`.
    pub fn added(&self, path: Sym, kind: Sym, scope: Option<Sym>) -> Scopes {
        let mut added = self.clone();
        added.insert(path, kind, scope);
        added
    }
}

/// The last `/`-separated segment of `path`, or the whole of a scheme ref id (`gh.pr:org/repo`).
fn last_segment(path: &str) -> &str {
    if path.contains(':') {
        return path;
    }
    path.rsplit('/').next().unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scopes() -> Scopes {
        let mut s = Scopes::new();
        let any = Sym::new;
        s.insert(any("v1.0"), any("milestone"), None);
        s.insert(any("v1.0/03"), any("phase"), Some(any("v1.0")));
        s.insert(any("v1.0/03/D-01"), any("decision"), Some(any("v1.0/03")));
        s.insert(any("D-01"), any("decision"), None);
        s.insert(any("v2.0"), any("milestone"), None);
        s.insert(any("v2.0/03"), any("phase"), Some(any("v2.0")));
        s
    }

    fn any_kind(_: &str) -> bool {
        true
    }

    #[test]
    fn a_name_is_found_nearest_first_and_a_rooted_path_in_the_project() {
        let s = scopes();
        let from = Some(Sym::new("v1.0/03"));
        assert_eq!(
            s.resolve(from, "D-01", &any_kind),
            Found::One(Sym::new("v1.0/03/D-01"))
        );
        assert_eq!(
            s.resolve(from, "/D-01", &any_kind),
            Found::One(Sym::new("D-01"))
        );
        assert_eq!(
            s.resolve(from, "03/D-01", &any_kind),
            Found::One(Sym::new("v1.0/03/D-01"))
        );
    }

    #[test]
    fn only_a_scope_field_or_a_surface_takes_a_suffix() {
        let s = scopes();
        let phase = |k: &str| k == "phase";
        let held = s.resolve(Some(Sym::new("D-01")), "03", &phase);
        assert_eq!(
            held,
            Found::None(vec![Sym::new("v1.0/03"), Sym::new("v2.0/03")])
        );
        assert_eq!(
            s.resolve(None, "03", &phase),
            Found::Ambiguous(vec![Sym::new("v1.0/03"), Sym::new("v2.0/03")])
        );
        assert_eq!(
            s.resolve(None, "v2.0/03", &phase),
            Found::One(Sym::new("v2.0/03"))
        );
    }

    #[test]
    fn a_spelling_is_the_shortest_the_chain_answers() {
        let s = scopes();
        let any = &any_kind;
        let from = Some(Sym::new("v1.0/03"));
        assert_eq!(s.spell(from, Sym::new("v1.0/03/D-01"), any), "D-01");
        assert_eq!(s.spell(from, Sym::new("D-01"), any), "/D-01");
        assert_eq!(s.spell(from, Sym::new("v2.0/03"), any), "v2.0/03");
        assert_eq!(s.spell(None, Sym::new("v1.0/03"), any), "v1.0/03");
    }

    #[test]
    fn a_move_takes_the_subtree() {
        let s = scopes().moved(&[(Sym::new("v1.0/03"), Sym::new("v2.0/04"))]);
        assert!(s.contains("v2.0/04/D-01"));
        assert!(!s.contains("v1.0/03/D-01"));
    }
}
