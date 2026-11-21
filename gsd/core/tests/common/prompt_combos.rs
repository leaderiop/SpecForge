//! Every lifecycle prompt with the argument combinations a sweep renders it with.

use gsd_kit::prompt::args::Values;

/// One render's input: the argument text and every declared name with its placeholder value (`args::values`).
pub type Combo = (String, Values);

/// Every prompt as `(command stem, combinations)`: none, every flag, every enum value, text and integer options.
pub fn prompts() -> Vec<(String, Vec<Combo>)> {
    let declaration = gsd_core::build().declaration();
    declaration
        .surfaces
        .prompts
        .iter()
        .map(|p| {
            // `(name, words, value)` of each given argument
            let combo = |given: &[(String, String, String)]| -> Combo {
                let words: Vec<&str> = given.iter().map(|(_, w, _)| w.as_str()).collect();
                let values = p
                    .args
                    .iter()
                    .map(|a| {
                        let value = given.iter().find(|(n, _, _)| *n == a.name);
                        (a.name.clone(), value.map(|(_, _, v)| v.clone()))
                    })
                    .collect();
                (words.join(" "), values)
            };
            let mut base = Vec::new();
            for a in p.args.iter().filter(|a| a.required) {
                base.push((a.name.clone(), "03".to_string(), "03".to_string()));
            }
            let mut texts = vec![combo(&base)];
            let mut flags = base.clone();
            for a in &p.args {
                let option = format!("--{}", a.name.replace('_', "-"));
                let name = a.name.clone();
                match a.field_type.as_str() {
                    "bool" => flags.push((name, option, "true".to_string())),
                    "integer" => flags.push((name, format!("{option} 2"), "2".to_string())),
                    "string" => {
                        flags.push((name, format!("{option} \"text\""), "text".to_string()))
                    }
                    "enum" => {
                        for value in &a.enum_values {
                            let mut one = base.clone();
                            one.push((name.clone(), format!("{option} {value}"), value.clone()));
                            texts.push(combo(&one));
                        }
                    }
                    _ => {}
                }
            }
            texts.push(combo(&flags));
            (p.id.replace('_', "-"), texts)
        })
        .collect()
}
