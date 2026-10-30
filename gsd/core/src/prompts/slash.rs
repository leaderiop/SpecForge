//! The slash command of a lifecycle prompt (D-r42-11): upstream's command frontmatter, bound with `c.slash_prompt`.
//! The host names it `gsd-<id with ->`. The slash binding is hidden by the setting of the first cluster that lists the
//! command; the prompt itself is always served, so a judge prompt is never hidden.

use specforge_extension_sdk::prelude::*;

use super::workflow::Spec;
use crate::settings::surface::{first_cluster, setting};

pub fn bind(c: &mut ContributionsBuilder, spec: &'static Spec) {
    let Some(slash) = &spec.slash else {
        return;
    };
    let when = first_cluster(spec.command).map(setting);
    c.slash_prompt(spec.id, |s| {
        s.argument_hint(slash.hint)
            .tools(slash.tools)
            .attribution(super::workflow::attribution(&format!(
                "commands/gsd/{}.md",
                spec.command
            )));
        if let Some(effort) = slash.effort {
            s.effort(effort);
        }
        if let Some(when) = &when {
            s.when(when);
        }
    });
}
