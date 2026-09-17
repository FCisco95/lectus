//! Custom dictionary: bias prompt + post-recognition replacement rules.
//!
//! Two complementary mechanisms:
//! 1. [`bias_prompt`] turns the user's word list into an `initial_prompt` hint
//!    so the recognizer prefers correct spellings of names/jargon.
//! 2. [`apply_rules`] runs exact find/replace over the final transcript, in the
//!    order the rules are listed, for deterministic fixes the bias can't guarantee.

use crate::config::ReplacementRule;

/// Build an `initial_prompt` bias string from the dictionary words, or `None`
/// when the list is empty. The phrasing nudges the model toward these terms.
pub fn bias_prompt(words: &[String]) -> Option<String> {
    let terms: Vec<&str> = words
        .iter()
        .map(|w| w.trim())
        .filter(|w| !w.is_empty())
        .collect();
    if terms.is_empty() {
        return None;
    }
    Some(format!("Vocabulary: {}.", terms.join(", ")))
}

/// Rewrite dictionary terms to their stored spelling (whole words, case-insensitive).
///
/// Whisper's `initial_prompt` is only a hint and often ignored, so "Claude"
/// coming back as "claude" looked like the vocabulary was never saved. This
/// is the deterministic half of that feature.
pub fn apply_dictionary_spellings(text: &str, words: &[String]) -> String {
    let mut terms: Vec<&str> = words
        .iter()
        .map(|w| w.trim())
        .filter(|w| !w.is_empty())
        .collect();
    // Longer phrases first so "Claude Code" wins over "Claude".
    terms.sort_by_key(|w| std::cmp::Reverse(w.len()));
    let mut out = text.to_string();
    for term in terms {
        out = replace_whole_word_ignore_case(&out, term, term);
    }
    out
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '\'' || c == '-'
}

fn replace_whole_word_ignore_case(haystack: &str, needle: &str, replacement: &str) -> String {
    let hay_lower = haystack.to_lowercase();
    let needle_lower = needle.to_lowercase();
    if needle_lower.is_empty() {
        return haystack.to_string();
    }
    let mut out = String::with_capacity(haystack.len());
    let mut last = 0;
    let mut search_from = 0;
    while let Some(rel) = hay_lower[search_from..].find(&needle_lower) {
        let start = search_from + rel;
        let end = start + needle_lower.len();
        let before_ok = start == 0
            || haystack[..start]
                .chars()
                .next_back()
                .map(|c| !is_word_char(c))
                .unwrap_or(true);
        let after_ok = end >= haystack.len()
            || haystack[end..]
                .chars()
                .next()
                .map(|c| !is_word_char(c))
                .unwrap_or(true);
        if before_ok && after_ok {
            out.push_str(&haystack[last..start]);
            out.push_str(replacement);
            last = end;
        }
        search_from = end;
        if search_from == start {
            // Zero-width advance guard (shouldn't happen with a non-empty needle).
            break;
        }
    }
    out.push_str(&haystack[last..]);
    out
}

/// Apply replacement rules to `text` in order. Empty `from` fields are skipped.
/// Case-insensitive rules match regardless of case but substitute `to` verbatim.
pub fn apply_rules(text: &str, rules: &[ReplacementRule]) -> String {
    let mut out = text.to_string();
    for rule in rules {
        if rule.from.is_empty() {
            continue;
        }
        out = if rule.case_sensitive {
            out.replace(&rule.from, &rule.to)
        } else {
            replace_ignore_case(&out, &rule.from, &rule.to)
        };
    }
    out
}

/// Case-insensitive substring replace. Scans the haystack lower-cased to find
/// match positions, but copies the original (untouched) text between matches.
fn replace_ignore_case(haystack: &str, needle: &str, replacement: &str) -> String {
    let hay_lower = haystack.to_lowercase();
    let needle_lower = needle.to_lowercase();
    let mut out = String::with_capacity(haystack.len());
    let mut last = 0;
    let mut search_from = 0;
    while let Some(rel) = hay_lower[search_from..].find(&needle_lower) {
        let start = search_from + rel;
        let end = start + needle_lower.len();
        out.push_str(&haystack[last..start]);
        out.push_str(replacement);
        last = end;
        search_from = end;
    }
    out.push_str(&haystack[last..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(from: &str, to: &str, cs: bool) -> ReplacementRule {
        ReplacementRule { from: from.into(), to: to.into(), case_sensitive: cs }
    }

    #[test]
    fn bias_prompt_joins_words() {
        let words = vec!["Lectus".to_string(), "Tauri".to_string()];
        assert_eq!(bias_prompt(&words).unwrap(), "Vocabulary: Lectus, Tauri.");
    }

    #[test]
    fn bias_prompt_none_when_empty() {
        assert!(bias_prompt(&[]).is_none());
        assert!(bias_prompt(&["   ".to_string()]).is_none());
    }

    #[test]
    fn replaces_case_insensitive_by_default() {
        let rules = vec![rule("lectus", "Lectus", false)];
        assert_eq!(apply_rules("i love lectus and LECTUS", &rules), "i love Lectus and Lectus");
    }

    #[test]
    fn replaces_case_sensitive_when_asked() {
        let rules = vec![rule("lectus", "Lectus", true)];
        assert_eq!(apply_rules("lectus LECTUS", &rules), "Lectus LECTUS");
    }

    #[test]
    fn applies_rules_in_order() {
        let rules = vec![rule("at gmail", "@gmail", false), rule("dot com", ".com", false)];
        assert_eq!(apply_rules("me at gmail dot com", &rules), "me @gmail .com");
    }

    #[test]
    fn empty_from_is_skipped() {
        let rules = vec![rule("", "x", false)];
        assert_eq!(apply_rules("unchanged", &rules), "unchanged");
    }

    #[test]
    fn canonicalizes_dictionary_word_casing() {
        let words = vec!["Mycel".into(), "Claude".into()];
        assert_eq!(
            apply_dictionary_spellings("i asked claude about mycel today", &words),
            "i asked Claude about Mycel today"
        );
    }

    #[test]
    fn dictionary_spelling_skips_substring_hits() {
        let words = vec!["Claude".into()];
        assert_eq!(
            apply_dictionary_spellings("claudette pinged Claude", &words),
            "claudette pinged Claude"
        );
    }

    #[test]
    fn dictionary_spelling_keeps_multi_word_terms() {
        let words = vec!["Claude Code".into()];
        assert_eq!(
            apply_dictionary_spellings("open claude code please", &words),
            "open Claude Code please"
        );
    }
}
