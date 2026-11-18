use super::*;

// ===========================================================================
// Slice 4: Schema Diffing
// ===========================================================================

#[specforge_test(requirement = "FEATURE-048")]
fn diff_removed_kind_is_breaking() {
    let old = sample_schema();
    let mut new = sample_schema();
    new.entity_kinds.retain(|k| k.name != "feature");

    let changes = diff(&old, &new);
    assert!(breaking(&changes));
    assert!(
        changes
            .iter()
            .any(|c| matches!(c, SchemaChange::KindRemoved(name) if name == "feature"))
    );
}

#[specforge_test(requirement = "FEATURE-048")]
fn diff_added_kind_is_non_breaking() {
    let old = sample_schema();
    let mut new = sample_schema();
    new.entity_kinds.push(SchemaEntityKind {
        name: "event".to_string(),
        source_extension: "@specforge/software".to_string(),
        testable: true,
        dot_color: None,
        fields: vec![],
    });

    let changes = diff(&old, &new);
    assert!(!breaking(&changes));
    assert!(additions(&changes));
}

#[specforge_test(requirement = "FEATURE-048")]
fn diff_added_optional_field_non_breaking() {
    let old = sample_schema();
    let mut new = sample_schema();
    new.entity_kinds[0]
        .fields
        .push(optional_field("description"));

    let changes = diff(&old, &new);
    assert!(!breaking(&changes));
    assert!(additions(&changes));
}

#[specforge_test(requirement = "FEATURE-048")]
fn diff_added_required_field_is_breaking() {
    let old = sample_schema();
    let mut new = sample_schema();
    new.entity_kinds[0].fields.push(SchemaField {
        required: true,
        ..optional_field("severity")
    });

    assert!(breaking(&diff(&old, &new)));
}

#[specforge_test(requirement = "FEATURE-048")]
fn diff_removed_field_is_breaking() {
    let old = sample_schema();
    let mut new = sample_schema();
    new.entity_kinds[0].fields.clear();

    let changes = diff(&old, &new);
    assert!(breaking(&changes));
    assert!(changes.iter().any(|c| matches!(c, SchemaChange::FieldRemoved { kind, field } if kind == "behavior" && field == "contract")));
}

#[specforge_test(requirement = "FEATURE-048")]
fn diff_removed_edge_is_breaking() {
    let old = sample_schema();
    let mut new = sample_schema();
    new.edge_types.clear();

    assert!(breaking(&diff(&old, &new)));
}

#[specforge_test(requirement = "FEATURE-048")]
fn diff_added_edge_non_breaking() {
    let old = sample_schema();
    let mut new = sample_schema();
    new.edge_types.push(SchemaEdgeType {
        label: "enforces".to_string(),
        source_extension: "@specforge/software".to_string(),
        source_kinds: None,
        target_kinds: None,
    });

    let changes = diff(&old, &new);
    assert!(!breaking(&changes));
    assert!(additions(&changes));
}

#[specforge_test(requirement = "FEATURE-048")]
fn diff_no_previous_schema_all_non_breaking() {
    let dir = tempfile::tempdir().unwrap();

    let versioned = SchemaCache::in_dir(dir.path()).version(sample_schema(), &PlainDisk);

    assert!(!breaking(versioned.changes()));
    assert!(versioned.notices().is_empty());
    assert_eq!(versioned.version(), &SchemaVersion::new(1, 0, 0));
}

#[specforge_test(requirement = "FEATURE-048")]
fn diff_identical_schemas_no_changes() {
    let changes = diff(&sample_schema(), &sample_schema());
    assert!(changes.is_empty());
    assert!(!breaking(&changes));
    assert!(!additions(&changes));
}

#[specforge_test(requirement = "FEATURE-048")]
fn diff_multiple_field_changes() {
    let old = sample_schema();
    let mut new = sample_schema();
    // Remove existing field and add a new one
    new.entity_kinds[0].fields.clear();
    new.entity_kinds[0]
        .fields
        .push(optional_field("description"));

    let changes = diff(&old, &new);
    assert_eq!(
        changes,
        vec![
            SchemaChange::FieldRemoved {
                kind: "behavior".to_string(),
                field: "contract".to_string(),
            },
            SchemaChange::FieldAdded {
                kind: "behavior".to_string(),
                field: "description".to_string(),
                required: false,
            },
        ]
    );
    assert!(breaking(&changes)); // removed "contract"
    assert!(additions(&changes)); // added "description"
}

#[specforge_test(requirement = "FEATURE-048")]
fn the_versioned_schema_lists_each_change_since_the_record() {
    let dir = tempfile::tempdir().unwrap();
    let cache = SchemaCache::in_dir(dir.path());
    cache.seed(&GraphProtocolSchema::empty());

    let versioned = cache.version(sample_schema(), &PlainDisk);

    assert_eq!(
        versioned.changes(),
        [
            SchemaChange::ExtensionAdded("@specforge/software".to_string()),
            // A new kind's fields come with it: no separate FieldAdded.
            SchemaChange::KindAdded("behavior".to_string()),
            SchemaChange::KindAdded("feature".to_string()),
            SchemaChange::EdgeTypeAdded("implements".to_string()),
        ]
    );
}

// B:detect_breaking_schema_changes — verify contract "requires/ensures consistency for breaking schema change detection"
#[specforge_test(requirement = "FEATURE-048")]
fn detect_breaking_contract() {
    let old = sample_schema();

    let mut new_breaking = sample_schema();
    new_breaking.entity_kinds.retain(|k| k.name != "feature");
    let changes = diff(&old, &new_breaking);
    // ensures: breaking_changes_classified
    assert!(breaking(&changes));

    let mut new_nonbreaking = sample_schema();
    new_nonbreaking.entity_kinds[0]
        .fields
        .push(optional_field("notes"));
    let changes2 = diff(&old, &new_nonbreaking);
    // ensures: nonbreaking_changes_classified
    assert!(!breaking(&changes2));
    assert!(additions(&changes2));

    // ensures: migration_record_emitted
    assert!(!changes.is_empty());
}

#[specforge_test(requirement = "FEATURE-048")]
fn a_single_target_kind_widened_to_a_list_is_minor_and_narrowed_is_major() {
    let mut single = sample_schema();
    let mut field = optional_field("owner");
    field.target_kind = Some("person".to_string());
    single.entity_kinds[0].fields.push(field.clone());
    let mut wide = sample_schema();
    field.target_kind = None;
    field.target_kinds = vec!["person".to_string(), "team".to_string()];
    wide.entity_kinds[0].fields.push(field);

    let bump = |old, new| diff(old, new).iter().map(SchemaChange::bump).max();
    assert_eq!(bump(&single, &wide), Some(Bump::Minor));
    assert_eq!(bump(&wide, &single), Some(Bump::Major));
}
