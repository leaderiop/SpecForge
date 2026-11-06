//! `quick_init`: what one `/gsd-quick` or `/gsd-quick-batch` run starts from (upstream's `init quick`): the host's date
//! and time, and one quick id per task, each free in the graph, with its slug and branch name.
//!
//! The ids are a function of the host's nonce (ADR 0341), the day and the graph, so a test pins them. An id minted here
//! is taken only once a plan of that name is written: a second call may mint it again, and the plan's write refuses a
//! duplicate.

use std::collections::BTreeSet;

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use super::support::{answer, control, given, nonce, now, run, slug, text, title};
use crate::passes::plan_shape::is_quick_id;

/// How many quick ids a day has: three base-36 digits.
const IDS: u64 = 36 * 36 * 36;

/// The next value of a splitmix64 stream: enough to spread a nonce over ids, with no dependency.
fn draw(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The three base-36 digits of `n`.
fn base36(n: u64) -> String {
    [n / (36 * 36), n / 36 % 36, n % 36]
        .iter()
        .map(|digit| char::from_digit(*digit as u32, 36).expect("a digit under 36"))
        .collect()
}

/// A checked nonce as the stream's seed: its two halves folded.
fn seed(nonce: &str) -> u64 {
    let half = |text: &str| u64::from_str_radix(text, 16).expect("a nonce is hex");
    let (high, low) = nonce.split_at(16);
    half(high) ^ half(low)
}

/// `n` ids of `day` (`yymmdd`): each starts at a draw and probes forward to the first id neither `taken` nor already
/// minted here, so it ends, and finds a free id whenever one exists.
fn ids<'a>(
    seed: u64,
    day: &str,
    taken: impl Iterator<Item = &'a str>,
    n: usize,
) -> Result<Vec<String>, String> {
    let prefix = format!("{day}-");
    let mut taken: BTreeSet<String> = taken
        .filter(|id| id.starts_with(&prefix) && is_quick_id(id))
        .map(str::to_string)
        .collect();
    let free = IDS as usize - taken.len();
    if n > free {
        return Err(format!(
            "{n} quick ids asked, and {free} of {day}'s {IDS} are free"
        ));
    }
    let mut state = seed;
    let mut minted = Vec::with_capacity(n);
    for _ in 0..n {
        let start = draw(&mut state) % IDS;
        let id = (0..IDS)
            .map(|probe| format!("{prefix}{}", base36((start + probe) % IDS)))
            .find(|id| !taken.contains(id))
            .expect("fewer ids asked than are free");
        taken.insert(id.clone());
        minted.push(id);
    }
    Ok(minted)
}

/// The entries of a task list: a leading `-`, `*`, `N.` or `N)` and the blanks around it stripped, blank lines dropped.
fn tasks(list: &str) -> Vec<&str> {
    list.lines()
        .map(|line| {
            let line = line.trim();
            let digits = line.bytes().take_while(u8::is_ascii_digit).count();
            let rest = match line.as_bytes().first() {
                Some(b'-' | b'*') => Some(&line[1..]),
                _ if digits > 0 => line[digits..]
                    .strip_prefix('.')
                    .or_else(|| line[digits..].strip_prefix(')')),
                _ => None,
            };
            match rest {
                Some(rest) if rest.is_empty() || rest.starts_with(char::is_whitespace) => {
                    rest.trim_start()
                }
                _ => line,
            }
        })
        .filter(|entry| !entry.is_empty())
        .collect()
}

/// A branch and directory slug: [`slug`] cut to 40 characters, no trailing dash.
fn slug40(text: &str) -> String {
    let whole = slug(text);
    whole[..whole.len().min(40)]
        .trim_end_matches('-')
        .to_string()
}

/// A branch template with `{num}` and `{slug}` filled; an empty template names no branch.
fn branch(template: &str, id: &str, slug: &str) -> Option<String> {
    (!template.is_empty()).then(|| template.replace("{num}", id).replace("{slug}", slug))
}

/// `quick_init [--description|--tasks|--count] [--batch] [--template]`: read-only, no gate, no subject.
fn init(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let now = now(call)?;
    let seed = seed(nonce(call)?);
    let description = given(call, "description")?;
    let list = call.str("tasks");
    let count = call.count("count");
    let batch = call.flag("batch");
    let template = text(call, "template")?.unwrap_or_default();
    if count.is_some() && (description.is_some() || list.is_some()) {
        return Err("--count asks for bare ids: it takes no --description or --tasks".to_string());
    }
    if count == Some(0) {
        return Err("--count asks for at least 1 id".to_string());
    }
    if description.is_some() && list.is_some() {
        return Err("--description is one task and --tasks a list: give one".to_string());
    }
    let (Some(date), Some(day)) = (now.get(..10), now.get(2..10)) else {
        return Err(format!("the host's clock '{now}' is no date"));
    };
    let day = day.replace('-', "");
    let texts: Vec<&str> = match (description, list) {
        (Some(one), _) => vec![one],
        (_, Some(list)) => tasks(list),
        _ => Vec::new(),
    };
    if let Some(code) = texts.iter().find_map(|entry| control(entry)) {
        return Err(format!("'tasks' holds a control character ({code})"));
    }
    match (batch, list) {
        (true, None) => return Err("--batch needs --tasks".to_string()),
        (true, Some(_)) if texts.len() < 2 => {
            return Err(format!(
                "a batch needs at least 2 tasks; the list holds {}",
                texts.len()
            ));
        }
        (false, Some(_)) if texts.is_empty() => {
            return Err("--tasks holds no task".to_string());
        }
        _ => {}
    }
    let n = if texts.is_empty() {
        count.unwrap_or(1)
    } else {
        texts.len()
    };
    let minted = ids(
        seed,
        &day,
        call.graph().nodes().iter().map(|node| node.id.as_str()),
        n,
    )?;
    let mut human = String::new();
    let items: Vec<_> = minted
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let mut item = json!({ "id": id });
            human.push_str(id);
            if let Some(task) = texts.get(i) {
                // A task with no ASCII letter or digit has no slug of its own: its id finds it as well.
                let slug = Some(slug40(task))
                    .filter(|slug| !slug.is_empty())
                    .unwrap_or_else(|| id.clone());
                if let Some(name) = branch(template, id, &slug) {
                    item["branch_name"] = json!(name);
                }
                item["description"] = json!(task);
                item["title"] = json!(title(task));
                item["slug"] = json!(slug);
                human.push('\t');
                human.push_str(task);
            }
            human.push('\n');
            item
        })
        .collect();
    let mut payload = json!({ "date": date, "timestamp": now, "count": n, "items": items });
    if batch {
        payload["batch"] = json!(format!("{day}-batch"));
    }
    Ok(answer(call, payload, human, None))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("quick_init", |cmd| {
        cmd.description("What a quick task or a batch starts from: the host's date and time and one quick id per task, each free in the graph, with its slug and branch name")
            .arg("description", |a| {
                a.description("one task: its one item gets a slug and a branch name");
            })
            .arg("tasks", |a| {
                a.description("a task list: one item per entry, its list marker and blank lines dropped; attach it with `=`, as a list starts with a hyphen");
            })
            .arg("count", |a| {
                a.count().description("this many bare ids, at least 1 (default 1); takes no --description or --tasks");
            })
            .arg("batch", |a| {
                a.flag().description("the items are a batch: needs --tasks with at least 2 entries, and the answer gains `batch`");
            })
            .arg("template", |a| {
                a.description("the branch template (`git.quick_branch_template`): `{num}` is the id, `{slug}` the slug; empty names no branch");
            })
            .handler(|call| run(call, init));
    });
}

#[cfg(test)]
mod tests;
