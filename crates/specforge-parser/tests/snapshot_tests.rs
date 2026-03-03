use specforge_parser::parse;

fn parse_snapshot(name: &str, source: &str) {
    let file = parse(source, &format!("{name}.spec"));
    let entities: Vec<_> = file.entities.iter().map(|e| {
        format!("{} {} {:?}", e.kind, e.id, e.title)
    }).collect();
    let errors: Vec<_> = file.errors.iter().map(|e| &e.message).collect();
    insta::assert_json_snapshot!(name, serde_json::json!({
        "entity_count": file.entities.len(),
        "entities": entities,
        "error_count": file.errors.len(),
        "errors": errors,
        "import_count": file.imports.len(),
    }));
}

#[test]
fn snapshot_invariant() {
    parse_snapshot("invariant", r#"
invariant data_integrity "Data Integrity" {
  guarantee """all data MUST be validated"""
  enforced_by [validate_input]
  risk high
}
"#);
}

#[test]
fn snapshot_behavior() {
    parse_snapshot("behavior", r#"
behavior validate_input "Validate Input" {
  invariants [data_integrity]
  contract """the system MUST validate all input"""
  verify unit "test validation"
  verify integration "test end-to-end"
}
"#);
}

#[test]
fn snapshot_feature() {
    parse_snapshot("feature", r#"
feature input_validation "Input Validation" {
  behaviors [validate_input, create_user]
  problem """invalid data enters the system"""
  solution """validate all inputs"""
  status active
}
"#);
}

#[test]
fn snapshot_event() {
    parse_snapshot("event", r#"
event user_created "User Created" {
  trigger validate_input
  channel user_events
  consumers [create_user]
  payload {
    user_id string
    email string
  }
}
"#);
}

#[test]
fn snapshot_type_struct() {
    parse_snapshot("type_struct", r#"
type User {
  id string @readonly @unique
  name string
  email string @unique
  active boolean
}
"#);
}

#[test]
fn snapshot_type_union() {
    parse_snapshot("type_union", r#"
type Status = active | inactive | deleted
"#);
}

#[test]
fn snapshot_spec_block() {
    parse_snapshot("spec_block", r#"
spec "myproject" {
  version "1.0"
  plugins ["@specforge/product"]

  persona developer "Developer" {
    description "builds the software"
  }

  surface cli "CLI" {
    type terminal
  }
}
"#);
}

#[test]
fn snapshot_use_import() {
    parse_snapshot("use_import", r#"
use invariants/core
use behaviors/parsing { validate_input, create_user }
"#);
}

#[test]
fn snapshot_error_recovery() {
    parse_snapshot("error_recovery", r#"
invariant data_integrity "Valid" {
  guarantee """ok"""
}

this is invalid syntax

behavior validate_input "Also Valid" {
  contract """ok"""
}
"#);
}
