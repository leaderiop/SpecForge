/// Convert a verify description to a valid Rust function name suffix.
///
/// Lowercase, replace operators with words, strip non-alphanumeric, collapse underscores.
pub fn slugify(input: &str) -> String {
    let lowered = input.to_lowercase();

    // Replace comparison operators with word equivalents (order matters: >= before >)
    let replaced = lowered
        .replace(">=", "_gte_")
        .replace("<=", "_lte_")
        .replace('>', "_gt_")
        .replace('<', "_lt_")
        .replace("!=", "_neq_")
        .replace("==", "_eq_");

    // Replace non-alphanumeric characters with underscores
    let mut result = String::with_capacity(replaced.len());
    for ch in replaced.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            result.push(ch);
        } else {
            result.push('_');
        }
    }

    // Collapse consecutive underscores and trim edges
    let mut collapsed = String::with_capacity(result.len());
    let mut prev_underscore = true; // start true to trim leading
    for ch in result.chars() {
        if ch == '_' {
            if !prev_underscore {
                collapsed.push('_');
            }
            prev_underscore = true;
        } else {
            collapsed.push(ch);
            prev_underscore = false;
        }
    }

    // Trim trailing underscore
    if collapsed.ends_with('_') {
        collapsed.pop();
    }

    collapsed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_description() {
        assert_eq!(slugify("rejects empty email"), "rejects_empty_email");
    }

    #[test]
    fn operators_replaced() {
        assert_eq!(slugify("value >= 10"), "value_gte_10");
        assert_eq!(slugify("value <= 0"), "value_lte_0");
        assert_eq!(slugify("count > max"), "count_gt_max");
        assert_eq!(slugify("age < minimum"), "age_lt_minimum");
    }

    #[test]
    fn special_chars_stripped() {
        assert_eq!(slugify("validates (all) inputs!"), "validates_all_inputs");
    }

    #[test]
    fn consecutive_spaces_collapsed() {
        assert_eq!(slugify("  multiple   spaces  "), "multiple_spaces");
    }

    #[test]
    fn empty_string() {
        assert_eq!(slugify(""), "");
    }

    #[test]
    fn already_snake_case() {
        assert_eq!(slugify("already_valid_name"), "already_valid_name");
    }

    #[test]
    fn mixed_case_lowered() {
        assert_eq!(slugify("Handles CamelCase Input"), "handles_camelcase_input");
    }
}
