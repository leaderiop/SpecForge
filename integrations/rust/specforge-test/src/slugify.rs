/// Converts a verify description into a test function name slug.
pub fn slugify_verify_description(description: &str) -> String {
    // Replace operators before character-level processing
    let description = description
        .replace("<=", " lte ")
        .replace(">=", " gte ")
        .replace('<', " lt ")
        .replace('>', " gt ");

    let mut result = String::with_capacity(description.len());

    for ch in description.chars() {
        match ch {
            ' ' => result.push('_'),
            'a'..='z' | '0'..='9' | '_' => result.push(ch),
            'A'..='Z' => result.push(ch.to_ascii_lowercase()),
            _ => {}
        }
    }

    // Collapse consecutive underscores
    let mut collapsed = String::with_capacity(result.len());
    let mut prev_underscore = false;
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

    collapsed.trim_matches('_').to_string()
}
