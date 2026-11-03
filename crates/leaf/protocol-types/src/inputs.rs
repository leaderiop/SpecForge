//! A call's inputs (protocol 2.0.0, ADR 0337 D2): what a pass, command, prompt, hook or collector declares the host
//! serves it before the call, as one tagged list. Which call kind is served which input is the registry's table; an
//! input its call kind is not served is W021 and served empty.

use serde::{Deserialize, Serialize};

use crate::FileRead;

/// One thing a call declares the host serves it before the call.
///
/// On the wire: `{"input": "files", "name": "sources", "glob": "src/**/*.rs"}`, `{"input": "tests"}`,
/// `{"input": "git", "status": true}`, `{"input": "setting", "extension": "@gsd/security", "name": "x.y"}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "input", rename_all = "snake_case")]
pub enum Input {
    /// Project files: the input's `files` carries their facts under the read's name (ADR 0293).
    Files(FileRead),
    /// The recorded test report, as `test_results` (a check, compute or guard pass).
    Tests,
    /// The extension's state, as `state`.
    State,
    /// The graph, as `graph`.
    Graph,
    /// The slash commands the project serves, as `slash_commands`.
    SlashCommands,
    /// The repository the `cwd` is in, as `git` (ADR 0320); with `status`, its typed status too. `when` names a bool
    /// setting of the extension: the facts are collected only while it is on.
    Git {
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        status: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        when: Option<String>,
    },
    /// Another extension's setting, in `other_settings` under `<extension>:<name>` (ADR 0323).
    Setting { extension: String, name: String },
}

impl Input {
    /// The files `read` names, served under `name`.
    pub fn files(name: &str, read: impl Into<FileRead>) -> Self {
        let mut read = read.into();
        read.name = name.to_string();
        Input::Files(read)
    }

    /// The repository the `cwd` is in, without its status.
    pub fn git() -> Self {
        Input::Git {
            status: false,
            when: None,
        }
    }

    /// A `git` input that carries the typed status too; any other input is returned as it is.
    pub fn status(self) -> Self {
        match self {
            Input::Git { when, .. } => Input::Git { status: true, when },
            other => other,
        }
    }

    /// A `git` input collected only while the bool setting `setting` is on; any other input is returned as it is.
    pub fn when(self, setting: &str) -> Self {
        match self {
            Input::Git { status, .. } => Input::Git {
                status,
                when: Some(setting.to_string()),
            },
            other => other,
        }
    }

    /// The setting `name` of `extension`.
    pub fn setting(extension: &str, name: &str) -> Self {
        Input::Setting {
            extension: extension.to_string(),
            name: name.to_string(),
        }
    }

    /// The input's kind, the one table its tag, its keys and what each call kind is served are read from.
    pub fn kind(&self) -> InputKind {
        match self {
            Input::Files(_) => InputKind::Files,
            Input::Tests => InputKind::Tests,
            Input::State => InputKind::State,
            Input::Graph => InputKind::Graph,
            Input::SlashCommands => InputKind::SlashCommands,
            Input::Git { .. } => InputKind::Git,
            Input::Setting { .. } => InputKind::Setting,
        }
    }

    /// The tag the input has on the wire (`files`, `tests`, …).
    pub fn tag(&self) -> &'static str {
        self.kind().tag()
    }
}

/// The kind of an [`Input`], without its data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputKind {
    Files,
    Tests,
    State,
    Graph,
    SlashCommands,
    Git,
    Setting,
}

impl InputKind {
    pub const ALL: [InputKind; 7] = [
        InputKind::Files,
        InputKind::Tests,
        InputKind::State,
        InputKind::Graph,
        InputKind::SlashCommands,
        InputKind::Git,
        InputKind::Setting,
    ];

    /// The one table: the kind's tag on the wire and the keys it writes beside the tag (a test holds `files`' to
    /// [`FileRead`]'s fields).
    const fn wire(self) -> (&'static str, &'static [&'static str]) {
        match self {
            InputKind::Files => ("files", &["name", "field", "glob", "base", "text"]),
            InputKind::Tests => ("tests", &[]),
            InputKind::State => ("state", &[]),
            InputKind::Graph => ("graph", &[]),
            InputKind::SlashCommands => ("slash_commands", &[]),
            InputKind::Git => ("git", &["status", "when"]),
            InputKind::Setting => ("setting", &["extension", "name"]),
        }
    }

    /// The tag the kind has on the wire (`files`, `tests`, …).
    pub const fn tag(self) -> &'static str {
        self.wire().0
    }
}

/// A `git` input as a call declares it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GitInput<'a> {
    /// Whether the typed status is served too.
    pub status: bool,
    /// The bool setting the facts are collected under.
    pub when: Option<&'a str>,
}

/// What a call's inputs ask for: the one accessor every host site reads them through.
#[derive(Debug, Clone, Copy)]
pub struct Inputs<'a>(pub &'a [Input]);

impl<'a> Inputs<'a> {
    /// The file reads, in order.
    pub fn files(self) -> Vec<FileRead> {
        self.0
            .iter()
            .filter_map(|input| match input {
                Input::Files(read) => Some(read.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn tests(self) -> bool {
        self.0.contains(&Input::Tests)
    }

    pub fn state(self) -> bool {
        self.0.contains(&Input::State)
    }

    pub fn graph(self) -> bool {
        self.0.contains(&Input::Graph)
    }

    pub fn slash_commands(self) -> bool {
        self.0.contains(&Input::SlashCommands)
    }

    /// The `git` input, when the call declares one (the first).
    pub fn git(self) -> Option<GitInput<'a>> {
        self.0.iter().find_map(|input| match input {
            Input::Git { status, when } => Some(GitInput {
                status: *status,
                when: when.as_deref(),
            }),
            _ => None,
        })
    }

    /// Each `setting` input, `(extension, name)`, in order.
    pub fn settings(self) -> impl Iterator<Item = (&'a str, &'a str)> {
        self.0.iter().filter_map(|input| match input {
            Input::Setting { extension, name } => Some((extension.as_str(), name.as_str())),
            _ => None,
        })
    }
}

/// Every key an input may write, the tag included: the W138 shape of an `inputs` item, read from [`InputKind`]'s table.
pub(crate) fn input_keys() -> &'static [&'static str] {
    static KEYS: std::sync::LazyLock<Vec<&'static str>> = std::sync::LazyLock::new(|| {
        let mut keys = vec!["input"];
        for kind in InputKind::ALL {
            for key in kind.wire().1 {
                if !keys.contains(key) {
                    keys.push(key);
                }
            }
        }
        keys
    });
    &KEYS
}

#[cfg(test)]
mod tests;
