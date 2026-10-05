//! The extension vocabulary: the names an extension uses for a field's type,
//! a validation rule's check kind and its constraint's kind.
//!
//! These enums are the one place the names are defined. The SDK writes them
//! (`as_str`), the host's registry build reads them (`parse`). On the wire
//! the descriptors still carry plain strings (`FieldDescriptor::field_type`,
//! `ValidationRuleDescriptor::check`, `FieldConstraintDescriptor::kind`), so a name this host does not know
//! costs the one field or rule a diagnostic instead of failing the whole
//! describe payload.
//!
//! `parse` also accepts the older spellings already in the wild — the
//! `_type`-suffixed field types of hand-written manifests and the names
//! earlier SDK releases emitted — so extensions built against them load
//! unchanged.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

macro_rules! vocabulary {
    (
        $(#[$meta:meta])*
        $name:ident {
            $( $(#[$vmeta:meta])* $variant:ident = $canonical:literal ),+ $(,)?
        }
        aliases { $( $alias:literal => $target:ident ),* $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name {
            $( $(#[$vmeta])* $variant, )+
        }

        impl $name {
            /// Every variant, in declaration order.
            pub const ALL: &'static [$name] = &[ $( $name::$variant, )+ ];

            /// The canonical wire name.
            pub fn as_str(self) -> &'static str {
                match self {
                    $( $name::$variant => $canonical, )+
                }
            }

            /// Read a wire name: the canonical name or an accepted older
            /// spelling. `None` for a name this host does not know.
            pub fn parse(name: &str) -> Option<Self> {
                match name {
                    $( $canonical => Some($name::$variant), )+
                    $( $alias => Some($name::$target), )*
                    _ => None,
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let name = std::borrow::Cow::<'de, str>::deserialize(deserializer)?;
                $name::parse(&name).ok_or_else(|| {
                    de::Error::unknown_variant(&name, &[ $( $canonical, )+ ])
                })
            }
        }
    };
}

vocabulary! {
    /// How the host reads a field's value.
    FieldType {
        /// A quoted string.
        String = "string",
        /// A whole number.
        Integer = "integer",
        /// `true` or `false`.
        Bool = "bool",
        /// One of the field's `enum_values`.
        Enum = "enum",
        /// A list of quoted strings.
        StringList = "string_list",
        /// One entity id; creates an edge.
        Reference = "reference",
        /// A list of entity ids; creates one edge per id.
        ReferenceList = "reference_list",
        /// A triple-quoted text block.
        Block = "block",
    }
    aliases {
        "string_type" => String,
        "integer_type" => Integer,
        "bool_type" => Bool,
        "boolean" => Bool,
        "enum_type" => Enum,
        "string_list_type" => StringList,
        "reference_type" => Reference,
        "reference_list_type" => ReferenceList,
        "block_type" => Block,
    }
}

vocabulary! {
    /// What a declarative validation rule checks, per entity of its
    /// `target_kind` (every entity when unset).
    CheckKind {
        /// No edge points at the entity (only `edge_type` edges when set).
        NoIncomingEdges = "no_incoming_edges",
        /// The entity points at nothing (only `edge_type` edges when set).
        NoOutgoingEdges = "no_outgoing_edges",
        /// The entity has no edges in either direction.
        NoEdges = "no_edges",
        /// The entity lacks `field`; union types are exempt when `field` is
        /// `verify`.
        MissingFieldWhenFlagSet = "missing_field_when_flag_set",
        /// `field`'s value breaks `constraint` (`non_empty`, `one_of`, or
        /// `matches`).
        FieldValueConstraint = "field_value_constraint",
        /// The entity sits on a cycle of `edge_type` edges.
        CycleDetection = "cycle_detection",
        /// The path in `field` does not exist.
        FileExists = "file_exists",
        /// The extension's `wasm_function` decides.
        Custom = "custom",
        /// When the field named by `constraint.pattern` holds one of
        /// `constraint.values`, `field` must be present and non-empty.
        ConditionalFieldRequired = "conditional_field_required",
        /// The entity lacks `field`.
        MissingRequiredField = "missing_required_field",
        /// A `verify` kind is not in `constraint.values`.
        VerifyKindAllowlist = "verify_kind_allowlist",
        /// A testable entity declares no `verify` obligations.
        NoVerifyStatements = "no_verify_statements",
    }
    aliases {
        // Emitted by SDK releases before the vocabulary was shared.
        "missing_field" => MissingRequiredField,
        "field_constraint" => FieldValueConstraint,
        "cycle" => CycleDetection,
        "conditional_required" => ConditionalFieldRequired,
    }
}

vocabulary! {
    /// How a rule's `constraint` reads its `pattern` and `values`.
    ConstraintKind {
        /// `field_value_constraint`: the value must not be empty.
        NonEmpty = "non_empty",
        /// The value must be one of `values` (`field_value_constraint`,
        /// `verify_kind_allowlist`).
        OneOf = "one_of",
        /// `field_value_constraint`: the value must match the regex in
        /// `pattern`.
        Matches = "matches",
        /// `conditional_field_required`: the condition holds when the field
        /// named by `pattern` has one of `values`.
        WhenFieldEquals = "when_field_equals",
    }
    aliases {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_names_round_trip() {
        for t in FieldType::ALL {
            assert_eq!(FieldType::parse(t.as_str()), Some(*t));
            let json = serde_json::to_string(t).unwrap();
            assert_eq!(json, format!("\"{}\"", t.as_str()));
            assert_eq!(serde_json::from_str::<FieldType>(&json).unwrap(), *t);
        }
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
        assert_eq!(FieldType::parse("date"), None);
        assert_eq!(CheckKind::parse("nope"), None);
        // The greet fixture once declared this; no host ever read it.
        assert_eq!(ConstraintKind::parse("field-constraint"), None);
        assert!(serde_json::from_str::<CheckKind>("\"nope\"").is_err());
    }
}
