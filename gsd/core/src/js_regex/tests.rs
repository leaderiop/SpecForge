//! `\b` and `\B` as JavaScript reads them without the `u` flag: a word character is ASCII `[A-Za-z0-9_]`.

use super::*;

fn chars(text: &str) -> Vec<char> {
    text.chars().collect()
}

fn matches(pattern: &str, text: &str) -> bool {
    Regex::new(pattern).is_match(&chars(text))
}

#[test]
fn a_word_boundary_is_javascripts() {
    for text in ["Cap the total", "a cap."] {
        assert!(matches(r"\bcap\b", text), "{text}");
    }
    for text in ["capital", "recap"] {
        assert!(!matches(r"\bcap\b", text), "{text}");
    }
    // `é` is no word character without the `u` flag, so a boundary falls before it
    assert!(matches(r"\bcaf\b", "café"));
    assert!(matches(r"\Bap\B", "caps"));
    assert!(!matches(r"\Bap\B", "ap"));
    assert!(matches(r"x\b", "x"));
    assert!(matches(r"\b_9", "a _9"));
    assert!(!matches(r"\b_9", "a_9"));
    assert_eq!(
        Regex::new(r"\bcap\b").find(&chars("recap cap")),
        Some((6, 9))
    );
}

#[test]
#[should_panic(expected = "a backspace class item")]
fn a_backspace_in_a_class_is_refused() {
    Regex::new(r"[\b]");
}

/// A linear congruential generator (Knuth's MMIX constants): the same texts on every run, and no `rand`.
struct Lcg(u64);

impl Lcg {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as usize % n
    }
}

#[test]
fn a_word_boundary_agrees_with_the_oracle() {
    const TEXTS: usize = 2_000;
    let (mut rng, mut mismatches) = (Lcg(0x6732_2026_1112), Vec::new());
    for (_, p) in crate::probe::edge::SHAPE_CUES {
        let (re, node) = (Regex::new(p), parse(p));
        let words: Vec<String> = p
            .split(|c: char| !c.is_ascii_alphabetic())
            .filter(|w| w.len() > 1)
            .map(String::from)
            .collect();
        let mut alphabet: Vec<char> = p.chars().filter(char::is_ascii_alphabetic).collect();
        alphabet.extend([' ', 'é', '_', '9']);
        let mut hits = 0;
        for _ in 0..TEXTS {
            let mut t = Vec::new();
            while t.len() < 24 {
                match rng.below(4) {
                    // a word of the pattern, cases mixed, between two edge characters, word or not
                    0 => {
                        let w = &words[rng.below(words.len())];
                        let edge = [' ', 'é', '_', '9', '.', ' '];
                        t.push(edge[rng.below(edge.len())]);
                        t.extend(w.chars().map(|c| match rng.below(3) {
                            0 => c.to_ascii_uppercase(),
                            _ => c,
                        }));
                        t.push(edge[rng.below(edge.len())]);
                    }
                    _ => t.push(alphabet[rng.below(alphabet.len())]),
                }
            }
            let (got, want) = (re.find(&t), oracle::find(&node, &t));
            hits += usize::from(want.is_some());
            if got != want {
                let text: String = t.iter().collect();
                mismatches.push(format!("{p} on {text:?}: {got:?}, the oracle {want:?}"));
            }
        }
        assert!(hits * 10 >= TEXTS, "{p}: only {hits} texts match");
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn a_word_boundary_is_linear() {
    let re = Regex::new(r"\b(count|number)\b");
    for n in [10_000, 40_000] {
        STEPS.with(|s| s.set(0));
        assert_eq!(re.find(&vec!['a'; n]), None);
        let steps = STEPS.with(std::cell::Cell::get);
        assert!(steps <= 64 * n, "{steps} steps on {n} characters");
    }
}
