/// Diagnostic code reference. Maps codes to human-readable explanations.
pub fn run(code: &str) -> i32 {
    let normalized = code.to_uppercase();
    match lookup(&normalized) {
        Some((title, explanation)) => {
            println!("\x1b[1m{normalized}\x1b[0m: {title}\n");
            println!("{explanation}");
            0
        }
        None => {
            eprintln!("unknown diagnostic code: {code}");
            eprintln!("hint: codes follow the pattern E### (error), W### (warning), I### (info)");
            1
        }
    }
}

fn lookup(code: &str) -> Option<(&'static str, &'static str)> {
    Some(match code {
        // === Core errors (specforge-parser + specforge-resolver) ===
        "E001" => ("Parse error",
            "The input could not be parsed as valid SpecForge syntax.\n\
             Check for missing braces, unclosed strings, or invalid field syntax."),
        "E002" => ("Duplicate entity",
            "Two entities with the same kind and ID were found.\n\
             Entity IDs must be unique within their kind across all files."),
        "E003" => ("Unresolved reference",
            "A reference list contains an ID that does not match any entity in the graph.\n\
             Check for typos or missing entity definitions."),
        "E004" => ("Invalid field value",
            "A field has a value that does not match its expected type.\n\
             For example, a reference list field contains a plain string."),
        "E005" => ("Cycle detected",
            "A circular dependency was found in the graph.\n\
             Entity A depends on B, which depends on A (directly or transitively)."),
        "E006" => ("Missing required field",
            "An entity is missing a field that its kind requires.\n\
             Check the extension manifest for required fields."),
        "E007" => ("Invalid status value",
            "A status field contains a value not in the allowed enum.\n\
             Check the extension manifest for valid status values."),
        "E008" => ("Invalid priority value",
            "A priority field contains a value not in the allowed enum.\n\
             Valid priorities are typically: critical, high, medium, low."),
        "E009" => ("Invalid artifact type",
            "A deliverable's artifact_type is not recognized.\n\
             Valid types are defined by the @specforge/product extension."),
        "E010" => ("Invalid event direction",
            "An event's direction must be 'inbound', 'outbound', or 'internal'."),
        "E015" => ("Invalid interaction model",
            "A channel's interaction_model is not recognized.\n\
             Valid models are defined by the @specforge/product extension."),
        "E016" => ("Invalid technical level",
            "A persona's technical_level is not recognized.\n\
             Valid levels are defined by the @specforge/product extension."),
        "E022" => ("Mistyped reference",
            "A reference points to an entity of the wrong kind.\n\
             For example, a 'features' field referencing a behavior instead of a feature."),

        // === Core warnings ===
        "W001" => ("Missing verify statement",
            "A testable entity has no verify statements.\n\
             Verify statements declare expected behavior for traceability."),
        "W002" => ("Unused entity",
            "An entity is not referenced by any other entity.\n\
             It may be orphaned or missing connections."),
        "W003" => ("Missing contract field",
            "A behavior entity has no contract field.\n\
             Contracts define the expected input/output behavior."),
        "W010" => ("Ambiguous reference",
            "A reference could match entities of different kinds.\n\
             Consider using more specific field names or entity IDs."),

        // === Product extension warnings ===
        "W041" => ("Orphan feature",
            "A feature is not referenced by any journey, milestone, or module.\n\
             It may not be reachable in the product graph."),
        "W042" => ("Orphan journey",
            "A journey has no deliverables referencing it."),
        "W043" => ("Orphan deliverable",
            "A deliverable is not included in any release."),
        "W044" => ("Orphan milestone",
            "A milestone is not referenced by any release."),
        "W045" => ("Orphan module",
            "A module is not referenced by any deliverable or milestone."),
        "W046" => ("Orphan term",
            "A term is not referenced by any other entity via see_also."),
        "W049" => ("Empty reference list",
            "A reference list field is present but empty.\n\
             Either add references or remove the field."),
        "W057" => ("Missing title",
            "An entity has no title string after its ID.\n\
             Titles improve readability and appear in exports."),
        "W060" => ("Cross-kind ID collision",
            "The same ID is used by entities of different kinds.\n\
             This can cause ambiguity in reference resolution."),

        // === Status transition warnings ===
        "W075" => ("Mixed list types",
            "A list field contains both string literals and identifier references.\n\
             Use a consistent type: all strings or all identifiers."),
        "W087" => ("Invalid feature status transition",
            "A feature's status changed to a state not reachable from its previous state.\n\
             Valid transitions are defined by the status state machine."),
        "W088" => ("Invalid milestone status transition",
            "A milestone's status transition is not valid."),
        "W089" => ("Invalid deliverable status transition",
            "A deliverable's status transition is not valid."),
        "W090" => ("Invalid persona status transition",
            "A persona's status transition is not valid."),
        "W091" => ("Invalid channel status transition",
            "A channel's status transition is not valid."),
        "W092" => ("Release dependency cycle",
            "Releases form a circular dependency chain."),
        "W093" => ("Release version not semver",
            "A release's version field is not valid semver.\n\
             Use the format MAJOR.MINOR.PATCH (e.g., 1.2.3)."),
        "W094" => ("Invalid release status transition",
            "A release's status transition is not valid."),
        "W095" => ("Invalid effort value",
            "An effort field contains an unrecognized size.\n\
             Valid values: xs, s, m, l, xl."),

        "W061" => ("Reference cycle detected",
            "A circular dependency was found in entity references.\n\
             Entity A references B, which references A (directly or transitively).\n\
             This is a warning — cycles may indicate a design issue."),
        "W062" => ("Malformed semver version",
            "A version string in an extension manifest is not valid semver.\n\
             Use versions like 1.0.0 and ranges like ^1.0.0, ~1.2.0, or >=1.0.0."),
        "W063" => ("Circular peer dependency",
            "Two or more extensions have circular peer dependency declarations.\n\
             Extension A depends on B, and B depends on A. Break the cycle by removing \n\
             one dependency or restructuring the extension boundaries."),
        "E028" => ("Incompatible host API version",
            "An extension requires a host API version that this build of SpecForge \n\
             does not support. Upgrade SpecForge or use a compatible extension version."),
        "I999" => ("Diagnostic output truncated",
            "More than 100 diagnostics were generated. Only the first 100 are shown.\n\
             Fix the reported issues and rerun to see remaining diagnostics."),

        // === Info diagnostics ===
        "I004" => ("Cross-extension reference",
            "A reference targets an entity from another extension that is not installed.\n\
             The reference is kept as-is but cannot be validated."),
        "I005" => ("Entity from unknown extension",
            "An entity uses a kind not registered by any installed extension.\n\
             The entity is still parsed but not validated."),
        "I010" => ("No spec files found",
            "The spec root directory contains no .spec files.\n\
             Create spec files or check the spec_root setting in specforge.json."),
        "I076" => ("Deliverable chain gap",
            "A deliverable's dependency chain has a gap (missing intermediate deliverable)."),
        "I077" => ("Feature multi-milestone",
            "A feature appears in multiple milestones.\n\
             This is informational — it may indicate scope overlap."),
        "I078" => ("Priority escalation gap",
            "A high-priority feature depends on a lower-priority feature."),
        "I079" => ("Milestone implicit ordering",
            "Milestones have an implicit ordering that may not match intent."),
        "I085" => ("Inconsistent owner strings",
            "Different entities use slightly different owner strings for the same person."),
        "I091" => ("Duplicate release version",
            "Two releases share the same version string."),
        "I097" => ("Build cache absent",
            "Status transition validation requires specforge-cache.json.\n\
             Run `specforge check` to generate the cache file."),

        _ => return None,
    })
}
