use crate::outcome::{Exit, OutputArg, OutputFormat, Refusal};
use crate::terminal::{Terminal, outln};
use specforge_ops::extension;
use specforge_ops::view::ProjectView;

// `specforge extensions`'s command line.
#[derive(clap::Args)]
pub(crate) struct Args {
    #[command(flatten)]
    project: crate::project::ProjectArg,

    #[command(flatten)]
    output: OutputArg,
}

impl Args {
    /// `specforge extensions`: every extension the project enables, has
    /// locked or loaded, alphabetically, with the entity kinds each
    /// registered and how many of the project's entities use them.
    pub(crate) fn run(self, terminal: &Terminal) -> Exit {
        let format = self.output.format;
        let project = match self.project.compile() {
            Ok(project) => project,
            Err(error) => return Refusal::of(format).report(terminal, &error),
        };
        let listing = extension::list(&ProjectView::of(&project));
        let entries = &listing.extensions;

        match format {
            OutputFormat::Json => terminal.json(&listing),
            OutputFormat::Human => {
                print_human(terminal, entries);
            }
        }

        Exit::Passed
    }
}

fn print_human(terminal: &Terminal, entries: &[extension::ExtensionInfo]) {
    if entries.is_empty() {
        outln!(terminal, "This project has no extensions.");
        outln!(terminal);
        outln!(terminal, "Add one with: specforge add <extension>");
        return;
    }
    outln!(terminal, "Extensions of the project:");
    outln!(terminal);
    for entry in entries {
        let version = entry
            .version
            .as_deref()
            .map(|v| format!(" v{v}"))
            .unwrap_or_default();
        let provides = if entry.entity_kinds.is_empty() {
            String::new()
        } else {
            format!(
                ": {} {} ({})",
                entry.entity_count,
                if entry.entity_count == 1 {
                    "entity"
                } else {
                    "entities"
                },
                entry.entity_kinds.join(", ")
            )
        };
        let status = match entry.status {
            extension::Status::Loaded => String::new(),
            other => format!(" [{}]", other.as_str()),
        };
        outln!(
            terminal,
            "  {}{version} ({}){status}{provides}",
            entry.name,
            entry.source
        );
        for read in &entry.reads {
            outln!(terminal, "      reads {}", read.line());
        }
    }
    outln!(terminal);
    let loaded = entries
        .iter()
        .filter(|e| e.status == extension::Status::Loaded)
        .count();
    outln!(terminal, "{} extension(s), {loaded} loaded.", entries.len());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::Facts;

    #[test]
    fn the_human_listing_names_each_declared_read() {
        let entry = |name: &str, reads| extension::ExtensionInfo {
            name: name.into(),
            version: Some("1.0.0".into()),
            source: "registry".into(),
            status: extension::Status::Loaded,
            entity_kinds: Vec::new(),
            entity_count: 0,
            validation_rules: 0,
            reads,
        };
        let read = extension::DeclaredRead {
            owner: "pass:scan".into(),
            name: "env".into(),
            field: None,
            glob: Some(".env*".into()),
            text: true,
        };
        let (terminal, capture) = Terminal::captured("", Facts::PIPED);

        print_human(
            &terminal,
            &[entry("@a/reader", vec![read]), entry("@a/quiet", vec![])],
        );

        let shown = capture.stdout();
        assert!(
            shown.contains(
                "@a/reader v1.0.0 (registry)\n      reads pass:scan env: .env* (with text)\n"
            ),
            "{shown}"
        );
        assert_eq!(shown.matches("reads ").count(), 1, "{shown}");
    }
}
