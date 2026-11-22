//! The check pass `reports` (ADR 0388): what the facts a standard report's reader minted leave unmapped, one code per
//! class, and the catalogue of those codes.

use specforge_code_facts::{
    ItemKind, API_ITEM, COVERAGE, DEPENDENCY, STATIC_FINDING, UNKNOWN, VULNERABILITY,
};
use specforge_extension_sdk::prelude::*;

use crate::REPORT_KINDS;

/// For each report fact that is not pending: W104 when it names no `item` (a `coverage`; an `api_item` of a kind an
/// item can have, not a variant, re-export, extern crate nor `other`, and not a crate root, which no item is; a
/// `static_finding` naming no `package` either), then W105 for a `static_finding` whose rule or level is `unknown`,
/// then W106 for a `dependency` with no `purl`, then W107 for a `vulnerability` whose severity is `unknown`.
pub(crate) fn reports(input: &PassInput) -> Vec<PassDiagnostic> {
    input
        .entities
        .iter()
        .filter(|e| !e.pending && REPORT_KINDS.contains(&e.kind.as_str()))
        .flat_map(|fact| {
            unmapped(fact)
                .into_iter()
                .chain(unknown(fact))
                .chain(purl_less(fact))
                .chain(unrated(fact))
        })
        .collect()
}

/// W104 when `fact` names no item and is a report entry an item should hold.
fn unmapped(fact: &PassEntity) -> Option<PassDiagnostic> {
    if fact.fields.contains_key("item") {
        return None;
    }
    let field = |name: &str| fact.fields.get(name).map_or("", String::as_str);
    let message = if fact.kind == COVERAGE.kind {
        format!(
            "the coverage fact '{}' names no item: no collected item holds its lines of `{}`",
            fact.id,
            field("file")
        )
    } else if fact.kind == API_ITEM.kind
        && ItemKind::NAMES.contains(&field("kind"))
        && !is_crate_root(&fact.id)
    {
        format!(
            "the api_item '{}' names no item: no collected item has its path `{}`",
            fact.id,
            field("path")
        )
    } else if fact.kind == STATIC_FINDING.kind && !fact.fields.contains_key("package") {
        format!(
            "the static_finding '{}' names no item and no package: no collected item or package holds its location",
            fact.id
        )
    } else {
        return None;
    };
    Some(PassDiagnostic::warning("W104", message).with_entity(&fact.id))
}

/// W105 when `fact` is a `static_finding` whose rule or level (or both) reads `unknown`, saying which.
fn unknown(fact: &PassEntity) -> Option<PassDiagnostic> {
    if fact.kind != STATIC_FINDING.kind {
        return None;
    }
    let unstated: Vec<&str> = ["rule", "level"]
        .into_iter()
        .filter(|name| fact.fields.get(*name).map(String::as_str) == Some(UNKNOWN))
        .collect();
    if unstated.is_empty() {
        return None;
    }
    let message = format!(
        "the static_finding '{}' states no {}: its SARIF result reads `{UNKNOWN}`",
        fact.id,
        unstated.join(" and ")
    );
    Some(PassDiagnostic::warning("W105", message).with_entity(&fact.id))
}

/// W106 when `fact` is a `dependency` with no `purl`, naming it by its name.
fn purl_less(fact: &PassEntity) -> Option<PassDiagnostic> {
    if fact.kind != DEPENDENCY.kind || fact.fields.contains_key("purl") {
        return None;
    }
    let message = format!(
        "the dependency '{}' has no purl: its BOM component `{}` is named by name and version only",
        fact.id,
        fact.fields.get("name").map_or("", String::as_str)
    );
    Some(PassDiagnostic::warning("W106", message).with_entity(&fact.id))
}

/// W107 when `fact` is a `vulnerability` whose severity reads `unknown`, naming its OSV id.
fn unrated(fact: &PassEntity) -> Option<PassDiagnostic> {
    if fact.kind != VULNERABILITY.kind
        || fact.fields.get("severity").map(String::as_str) != Some(UNKNOWN)
    {
        return None;
    }
    let message = format!(
        "the vulnerability '{}' has an unknown severity: its OSV record `{}` states no label and no CVSS v3 vector \
         this reader rates",
        fact.id,
        fact.fields.get("osv_id").map_or("", String::as_str)
    );
    Some(PassDiagnostic::warning("W107", message).with_entity(&fact.id))
}

/// Whether `id` is a Rust crate root's `api:rust:<crate>`: a crate is no item.
fn is_crate_root(id: &str) -> bool {
    id.strip_prefix("api:rust:")
        .is_some_and(|krate| !krate.contains("::"))
}

/// The codes the `reports` pass raises.
pub(crate) fn catalogue(c: &mut ContributionsBuilder) {
    c.code("W104", |d| {
        d.title("A report entry names no item").explanation(
            "A fact a report reader minted cites no item, because the report's file or line lies in no collected \
             item (a macro expansion, top-level statements, or a file no source pack collected), or because an API \
             snapshot's entry has a path no collected item has: a re-exported path whose definition path differs, \
             an item a macro generates, or a crate whose library is named apart from its package. A static finding \
             is reported only when no collected package holds its file either, or it has no location. The entry is \
             kept with what the report says of it, so nothing is lost. Collect the source pack for those files \
             (`@rust/source`, `@javascript/source`) and the manifest pack for their packages, then collect the \
             report again.",
        );
    });
    c.code("W105", |d| {
        d.title("A finding's rule or level is unknown").explanation(
            "A static finding's SARIF result states no rule (no `ruleId`, no `rule`, and no rule index naming a \
             rule of its tool's driver), or states no level (none on the result, and none in its rule's \
             `defaultConfiguration`): the field reads `unknown`. The finding is kept, never dropped. SARIF's own \
             default level is `warning`; it is not applied, so a level the tool never wrote cannot pass for one \
             it did. Make the tool write its rule ids and levels in its SARIF output, or the rule metadata \
             (`tool.driver.rules` with a `defaultConfiguration.level`), then collect the report again.",
        );
    });
    c.code("W106", |d| {
        d.title("A dependency has no purl").explanation(
            "A component of a CycloneDX bill of materials carries no package URL (`purl`), so its dependency is \
             named by name and version only (`dep:<package>/@<name@version>`). The dependency is kept, never \
             dropped, but no other report can be matched to it: an OSV scan or another BOM names a package by its \
             purl. Generate the BOM with an SBOM tool that writes a purl for every component (cargo-cyclonedx, \
             cyclonedx-npm), then collect it again.",
        );
    });
    c.code("W107", |d| {
        d.title("A vulnerability's severity is unknown").explanation(
            "An OSV record of a vulnerability scan has no severity label (`database_specific.severity`) and no \
             CVSS v3 vector this reader rates: its only severity entries are CVSS v4 or v2 vectors, or its v3 \
             vector is malformed. The vulnerability is kept with severity `unknown`, never dropped and never given \
             a guessed rating, so a gate on severity cannot pass it by mistake. Read the advisory by its OSV id \
             (osv.dev, or the database its id names) and judge its severity there.",
        );
    });
}
