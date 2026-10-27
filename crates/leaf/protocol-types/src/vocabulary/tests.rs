use super::*;

#[test]
fn canonical_names_round_trip() {
    for t in FieldType::ALL {
        assert_eq!(FieldType::parse(t.as_str()), Some(*t));
        let json = serde_json::to_string(t).unwrap();
        assert_eq!(json, format!("\"{}\"", t.as_str()));
        assert_eq!(serde_json::from_str::<FieldType>(&json).unwrap(), *t);
    }
    for r in ProofRole::ALL {
        assert_eq!(ProofRole::parse(r.as_str()), Some(*r));
        let json = serde_json::to_string(r).unwrap();
        assert_eq!(serde_json::from_str::<ProofRole>(&json).unwrap(), *r);
    }
    assert_eq!(ProofRole::parse("assumed"), None);
    for c in CheckKind::ALL {
        assert_eq!(CheckKind::parse(c.as_str()), Some(*c));
        let json = serde_json::to_string(c).unwrap();
        assert_eq!(serde_json::from_str::<CheckKind>(&json).unwrap(), *c);
    }
    for k in ConstraintKind::ALL {
        assert_eq!(ConstraintKind::parse(k.as_str()), Some(*k));
        let json = serde_json::to_string(k).unwrap();
        assert_eq!(serde_json::from_str::<ConstraintKind>(&json).unwrap(), *k);
    }
}

#[test]
fn reference_and_list_types() {
    for t in FieldType::ALL {
        assert_eq!(
            t.is_reference(),
            matches!(t, FieldType::Reference | FieldType::ReferenceList),
            "{t}"
        );
        assert_eq!(
            t.is_list(),
            matches!(
                t,
                FieldType::StringList
                    | FieldType::ReferenceList
                    | FieldType::EnumList
                    | FieldType::BlockList
            ),
            "{t}"
        );
        assert_eq!(
            t.is_block(),
            matches!(t, FieldType::Block | FieldType::BlockList),
            "{t}"
        );
    }
}

#[test]
fn older_spellings_are_accepted() {
    assert_eq!(FieldType::parse("bool_type"), Some(FieldType::Bool));
    assert_eq!(FieldType::parse("boolean"), Some(FieldType::Bool));
    assert_eq!(FieldType::parse("block_type"), Some(FieldType::Block));
    assert_eq!(
        CheckKind::parse("missing_field"),
        Some(CheckKind::MissingRequiredField)
    );
    assert_eq!(
        CheckKind::parse("field_constraint"),
        Some(CheckKind::FieldValueConstraint)
    );
    assert_eq!(CheckKind::parse("cycle"), Some(CheckKind::CycleDetection));
    assert_eq!(
        CheckKind::parse("conditional_required"),
        Some(CheckKind::ConditionalFieldRequired)
    );
}

#[test]
fn unknown_names_are_rejected() {
    assert_eq!(FieldType::parse("datetime"), None);
    assert_eq!(CheckKind::parse("nope"), None);
    // The greet fixture once declared this; no host ever read it.
    assert_eq!(ConstraintKind::parse("field-constraint"), None);
    assert!(serde_json::from_str::<CheckKind>("\"nope\"").is_err());
}

#[test]
fn date_is_a_field_type_with_no_older_spelling() {
    assert_eq!(FieldType::parse("date"), Some(FieldType::Date));
    assert_eq!(FieldType::parse("date_type"), None);
}

#[test]
fn enum_list_block_list_and_file_base_are_new_names() {
    assert_eq!(FieldType::parse("enum_list"), Some(FieldType::EnumList));
    assert_eq!(FieldType::parse("block_list"), Some(FieldType::BlockList));
    assert_eq!(FieldType::parse("enum_list_type"), None);
    assert_eq!(FileBase::parse("project"), Some(FileBase::Project));
    assert_eq!(FileBase::parse("spec"), Some(FileBase::Spec));
    assert_eq!(FileBase::parse("root"), None);
}
