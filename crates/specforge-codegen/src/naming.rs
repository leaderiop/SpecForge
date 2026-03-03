use specforge_common::NamingStyle;

/// Split an identifier into lowercase word segments.
///
/// Handles PascalCase, camelCase, snake_case, and kebab-case inputs.
fn split_words(input: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();

    for ch in input.chars() {
        if ch == '_' || ch == '-' {
            if !current.is_empty() {
                words.push(current.to_lowercase());
                current.clear();
            }
        } else if ch.is_uppercase() && !current.is_empty() {
            words.push(current.to_lowercase());
            current.clear();
            current.push(ch);
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        words.push(current.to_lowercase());
    }
    words
}

/// Transform a name to the target naming style.
pub fn transform(input: &str, style: NamingStyle) -> String {
    let words = split_words(input);
    if words.is_empty() {
        return String::new();
    }

    match style {
        NamingStyle::CamelCase => {
            let mut result = words[0].clone();
            for w in &words[1..] {
                result.push_str(&capitalize(w));
            }
            result
        }
        NamingStyle::PascalCase => words.iter().map(|w| capitalize(w)).collect(),
        NamingStyle::SnakeCase => words.join("_"),
        NamingStyle::KebabCase => words.join("-"),
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Convert a PascalCase type name to a kebab-case file name.
pub fn to_file_name(pascal_name: &str) -> String {
    transform(pascal_name, NamingStyle::KebabCase)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pascal_to_camel() {
        assert_eq!(transform("UserProfile", NamingStyle::CamelCase), "userProfile");
    }

    #[test]
    fn snake_to_pascal() {
        assert_eq!(transform("user_profile", NamingStyle::PascalCase), "UserProfile");
    }

    #[test]
    fn snake_to_camel() {
        assert_eq!(transform("user_profile", NamingStyle::CamelCase), "userProfile");
    }

    #[test]
    fn pascal_to_snake() {
        assert_eq!(transform("UserProfile", NamingStyle::SnakeCase), "user_profile");
    }

    #[test]
    fn pascal_to_kebab() {
        assert_eq!(transform("UserProfile", NamingStyle::KebabCase), "user-profile");
    }

    #[test]
    fn kebab_to_pascal() {
        assert_eq!(transform("user-profile", NamingStyle::PascalCase), "UserProfile");
    }

    #[test]
    fn single_word() {
        assert_eq!(transform("user", NamingStyle::PascalCase), "User");
        assert_eq!(transform("User", NamingStyle::CamelCase), "user");
        assert_eq!(transform("User", NamingStyle::SnakeCase), "user");
    }

    #[test]
    fn empty_string() {
        assert_eq!(transform("", NamingStyle::PascalCase), "");
    }

    #[test]
    fn file_name_from_pascal() {
        assert_eq!(to_file_name("FileSystem"), "file-system");
        assert_eq!(to_file_name("UserRepository"), "user-repository");
    }
}
