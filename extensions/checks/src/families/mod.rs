//! The five families: what each may make the host do and the setting that switches it.

use specforge_extension_sdk::prelude::*;

use crate::codes::*;

/// The scopes the families' probes run under. Formats and digits read only the text, so they have none.
const FILES: &str = "files";
const LINKS: &str = "links";
const GIT: &str = "git";

/// A family's switch and the codes only that family reports. E103 is shared, so no switch gates it.
struct Family {
    switch: &'static str,
    gates: &'static [&'static str],
    describe: &'static str,
}

const FAMILIES: [Family; 5] = [
    Family {
        switch: "families.files",
        gates: &[E096, W176],
        describe: "Check the files and directories the values name",
    },
    Family {
        switch: "families.formats",
        gates: &[],
        describe:
            "Check that values are well formed: dates, versions, licences, addresses and the like",
    },
    Family {
        switch: "families.digits",
        gates: &[],
        describe: "Check the check digits of the identifiers values carry",
    },
    Family {
        switch: "families.links",
        gates: &[
            E091, E092, E093, E104, E112, W172, W173, W178, W181, W182, W183,
        ],
        describe: "Check that the URLs values name answer",
    },
    Family {
        switch: "families.git",
        gates: &[E106, W180],
        describe: "Check the commits, refs and tracked paths values name",
    },
];

pub(crate) fn contribute(c: &mut ContributionsBuilder) {
    c.capability(
        Capability::fs(FILES)
            .ops(&[FsAccess::Stat, FsAccess::Read, FsAccess::Walk])
            .why("check the files and directories the values name"),
    );
    c.capability(
        Capability::http(LINKS)
            .from_values()
            .schemes(&["https", "http"])
            .methods(&[HttpMethod::Head, HttpMethod::Get])
            .why("check that the URLs the values name answer"),
    );
    c.capability(
        Capability::git(GIT).why("check the commits, refs and tracked paths the values name"),
    );
    for family in &FAMILIES {
        c.setting(family.switch, |s| {
            s.bool()
                .default(true)
                .describe(family.describe)
                .gates(family.gates);
        });
    }
}
