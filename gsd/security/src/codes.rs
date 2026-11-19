//! The code book of `@gsd/security`: every `gsd.security/` code, declared in the `codes` category so `specforge
//! explain gsd.security/W301` describes it. The host adds the namespace to a local code (ADR 0286).

use specforge_extension_sdk::prelude::*;

/// `(local code, title, explanation)`.
pub const CODES: &[(&str, &str, &str)] = &[
    (
        "E101",
        "a threat ID is claimed by more than one plan of its phase",
        "Two plans of one phase hold a threat with the same T-<phase>-NN ID, so the audit cannot tell them apart: \
         renumber one after the phase's highest. The supply-chain row T-<phase>-SC is every plan's and never counts \
         (gsd-core #4683).",
    ),
    (
        "W301",
        "a completed phase has no verified security audit",
        "A phase that is complete has no security audit, its audit is still a draft, or it has threats open at or \
         above workflow.security_block_on: run /gsd-secure-phase for the phase. A warning, never a refusal (gsd-core \
         #4972); switched off by workflow.security_enforcement.",
    ),
    (
        "W302",
        "a closed threat's mitigation is not judged",
        "A closed threat claims its disposition holds, and no fresh verdict of the security workflow says so: for \
         `mitigate`, that the mitigation is in its files; for `accept`, that an accepted risk records it; for \
         `transfer`, that the transfer is documented. The verdict reads the threat, its plan, its files, its accepted \
         risks and the VALIDATION.md rows that secure it, so an edit to one of its files makes it stale: run \
         /gsd-secure-phase, which records it with `specforge gsd record-security-verdict`. A warning; switched off by \
         workflow.security_enforcement.",
    ),
    (
        "W303",
        "a phase's threat flags are not judged against its register",
        "A phase's security audit has no fresh verdict on whether every threat flag its plans' summaries raise maps to \
         a threat of the phase's register: upstream's auditor logs a flag with no mapping as an `unregistered_flag` and \
         SECURITY.md lists it under Unregistered Flags. The verdict reads the audit, the phase's summaries and its \
         threats, so a new summary or threat makes it stale: run /gsd-secure-phase, which records `mapped` or \
         `unregistered` with `specforge gsd record-security-verdict`. A warning; switched off by \
         workflow.security_enforcement.",
    ),
];

pub fn declare(c: &mut ContributionsBuilder) {
    for (id, title, explanation) in CODES {
        c.code(id, |d| {
            d.title(title).explanation(explanation);
        });
    }
}
