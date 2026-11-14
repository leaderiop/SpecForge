//! A form's inclusive bounds, read by its type (ADR 0361): whole numbers for an `integer`, exact decimals for a
//! `number`. A value is compared `i64` against `i64` or `Decimal` against `Decimal`, never through `f64`.

use specforge_protocol_types::{Decimal, FieldType};

use super::{FieldForm, Refusal};

/// The least and greatest value a form takes, inclusive. A form of any type other than `number` holds
/// [`Bounds::Integer`], unbounded unless it is an `integer` that declares one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bounds {
    /// An `integer`'s whole bounds.
    Integer(Option<i64>, Option<i64>),
    /// A `number`'s decimal bounds.
    Number(Option<Decimal>, Option<Decimal>),
}

impl Bounds {
    /// The bounds `minimum` and `maximum` are for a field of `field_type`. A bound the type cannot read (a fraction or
    /// a value beyond `i64` on an `integer`, more than 15 significant digits on a `number`) is dropped: the registry
    /// refused it with W021 first.
    pub fn of(
        field_type: FieldType,
        minimum: Option<&serde_json::Number>,
        maximum: Option<&serde_json::Number>,
    ) -> Bounds {
        match field_type {
            FieldType::Number => {
                let read =
                    |n: Option<&serde_json::Number>| n.and_then(|n| Decimal::from_json(n).ok());
                Bounds::Number(read(minimum), read(maximum))
            }
            _ => Bounds::Integer(
                minimum.and_then(serde_json::Number::as_i64),
                maximum.and_then(serde_json::Number::as_i64),
            ),
        }
    }

    /// Whether neither bound is set.
    pub fn is_unbounded(&self) -> bool {
        matches!(
            self,
            Bounds::Integer(None, None) | Bounds::Number(None, None)
        )
    }

    /// Whether the least bound lies above the greatest.
    pub fn is_inverted(&self) -> bool {
        match self {
            Bounds::Integer(Some(low), Some(high)) => low > high,
            Bounds::Number(Some(low), Some(high)) => low > high,
            _ => false,
        }
    }

    /// Each bound's text, `None` when unset: `0`, `0.05`.
    pub fn texts(&self) -> (Option<String>, Option<String>) {
        fn text<T: ToString>(n: &Option<T>) -> Option<String> {
            n.as_ref().map(T::to_string)
        }
        match self {
            Bounds::Integer(low, high) => (text(low), text(high)),
            Bounds::Number(low, high) => (text(low), text(high)),
        }
    }

    /// Each bound as the JSON number the schema publishes.
    pub fn json(&self) -> (Option<serde_json::Number>, Option<serde_json::Number>) {
        match *self {
            Bounds::Integer(low, high) => (low.map(Into::into), high.map(Into::into)),
            Bounds::Number(low, high) => (low.map(Into::into), high.map(Into::into)),
        }
    }

    /// What a value within the bounds is, for a message: `a whole number from 1 to 3`, `a number of at most 0.95`.
    pub fn describe(&self) -> String {
        let noun = match self {
            Bounds::Integer(..) => "a whole number",
            Bounds::Number(..) => "a number",
        };
        match self.texts() {
            (Some(low), Some(high)) => format!("{noun} from {low} to {high}"),
            (Some(low), None) => format!("{noun} of at least {low}"),
            (None, Some(high)) => format!("{noun} of at most {high}"),
            (None, None) => noun.to_string(),
        }
    }

    /// E061's refusal of a value outside the bounds: `write a number of at most 0.95`.
    fn refusal(&self) -> Refusal {
        Refusal::with(&format!("write {}", self.describe()))
    }
}

impl FieldForm {
    /// `Ok` when the integer `text` lies within the declared bounds; E061's remedy names them.
    pub(super) fn in_bounds(&self, text: &str) -> Result<(), Refusal> {
        let Bounds::Integer(minimum, maximum) = self.bounds else {
            return Ok(());
        };
        let Ok(n) = text.trim().parse::<i64>() else {
            return Ok(());
        };
        if minimum.is_some_and(|m| n < m) || maximum.is_some_and(|m| n > m) {
            return Err(self.bounds.refusal());
        }
        Ok(())
    }

    /// `Ok` when `value` lies within the declared decimal bounds, compared exactly; E061's remedy names them.
    pub(super) fn in_decimal_bounds(&self, value: Decimal) -> Result<(), Refusal> {
        let Bounds::Number(minimum, maximum) = self.bounds else {
            return Ok(());
        };
        if minimum.is_some_and(|m| value < m) || maximum.is_some_and(|m| value > m) {
            return Err(self.bounds.refusal());
        }
        Ok(())
    }
}
