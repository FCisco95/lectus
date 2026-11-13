//! Bounded local vocabulary suggestions. Only selected terms are saved by the UI.
use regex::Regex;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File, Metadata};
use std::io::Read;
use std::path::Path;
use std::sync::LazyLock;

#[derive(Debug, Serialize)]
pub struct TermCandidate {
    pub term: String,
    pub count: u32,
    pub score: f32,
}
#[derive(Debug, Serialize)]
pub struct ScanReport {
    pub candidates: Vec<TermCandidate>,
    pub files_read: u32,
    pub truncated: bool,
}
pub struct ScanLimits {
    pub max_files: usize,
    pub max_total_bytes: u64,
    pub max_file_bytes: u64,
    pub max_depth: usize,
}
impl Default for ScanLimits {
    fn default() -> Self {
        Self {
            max_files: 5000,
            max_total_bytes: 50 * 1024 * 1024,
            max_file_bytes: 1024 * 1024,
            max_depth: 12,
        }
    }
}

static WORDS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\p{L}\p{N}]+(?:-[\p{L}\p{N}]+)*").unwrap());
static INLINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"`+[^`]*`+").unwrap());
static LINKS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"!?\[([^\]\r\n]*)\]\([^\)\r\n]*\)").unwrap());
static URLS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:https?://|www\.)[^\s<>]+").unwrap());

fn clean_text(text: &str) -> String {
    let mut body = String::new();
    let mut front_matter = false;
    let mut fence: Option<(char, usize)> = None;
    for (index, line) in text.trim_start_matches('\u{feff}').lines().enumerate() {
        if index == 0 && line.trim_end() == "---" {
            front_matter = true;
            continue;
        }
        if front_matter {
            if matches!(line.trim(), "---" | "...") {
                front_matter = false;
            }
            continue;
        }
        let trimmed = line.trim_start();
        let marker = trimmed.chars().next().filter(|c| matches!(*c, '`' | '~'));
        let count = marker
            .map(|marker| trimmed.chars().take_while(|c| *c == marker).count())
            .unwrap_or(0);
        if let Some((open, width)) = fence {
            if marker == Some(open) && count >= width && trimmed[count..].trim().is_empty() {
                fence = None;
            }
            continue;
        }
        if count >= 3 {
            fence = Some((marker.unwrap(), count));
            continue;
        }
        body.push_str(line);
        body.push('\n');
    }
    let body = LINKS.replace_all(&body, "$1");
    let body = INLINE.replace_all(&body, " ");
    URLS.replace_all(&body, " ")
        .replace("[[", "")
        .replace("]]", "")
}

fn lowercase_word(word: &str) -> bool {
    word.chars().any(char::is_alphabetic) && word == word.to_lowercase()
}
fn title_word(word: &str) -> bool {
    let mut chars = word.chars();
    chars.next().is_some_and(char::is_uppercase)
        && chars.filter(|c| c.is_alphabetic()).all(char::is_lowercase)
}
fn stop_word(term: &str) -> bool {
    matches!(
        term.to_lowercase().replace(' ', "-").as_str(),
        "i" | "ok"
            | "todo"
            | "fixme"
            | "readme"
            | "january"
            | "february"
            | "march"
            | "april"
            | "may"
            | "june"
            | "july"
            | "august"
            | "september"
            | "october"
            | "november"
            | "december"
            | "monday"
            | "tuesday"
            | "wednesday"
            | "thursday"
            | "friday"
            | "saturday"
            | "sunday"
            | "janeiro"
            | "fevereiro"
            | "março"
            | "marco"
            | "abril"
            | "maio"
            | "junho"
            | "julho"
            | "agosto"
            | "setembro"
            | "outubro"
            | "novembro"
            | "dezembro"
            | "segunda"
            | "segunda-feira"
            | "terça"
            | "terça-feira"
            | "terca"
            | "terca-feira"
            | "quarta"
            | "quarta-feira"
            | "quinta"
            | "quinta-feira"
            | "sexta"
            | "sexta-feira"
            | "sábado"
            | "sabado"
            | "domingo"
    )
}
struct Frequency {
    term: String,
    count: u32,
    capitals: u32,
    weight: f32,
}
fn add_term(terms: &mut HashMap<String, Frequency>, term: &str, weight: f32) {
    if stop_word(term) {
        return;
    }
    let entry = terms
        .entry(term.to_lowercase())
        .or_insert_with(|| Frequency {
            term: term.into(),
            count: 0,
            capitals: 0,
            weight,
        });
    entry.count = entry.count.saturating_add(1);
    if term.chars().any(char::is_uppercase) {
        entry.capitals = entry.capitals.saturating_add(1);
    }
    if weight > entry.weight || (weight == entry.weight && term < entry.term.as_str()) {
        entry.term = term.into();
        entry.weight = weight;
    }
}

pub fn extract_candidates(texts: &[String], existing: &[String], top: usize) -> Vec<TermCandidate> {
    if top == 0 {
        return Vec::new();
    }
    let existing: HashSet<String> = existing.iter().map(|word| word.to_lowercase()).collect();
    let mut terms = HashMap::new();
    for text in texts {
        let text = clean_text(text);
        let words: Vec<_> = WORDS.find_iter(&text).collect();
        for (index, found) in words.iter().enumerate() {
            let word = found.as_str();
            let middle = index > 0
                && !text[words[index - 1].end()..found.start()]
                    .chars()
                    .any(|c| matches!(c, '.' | '!' | '?' | '\n' | '\r'));
            let follows_lower = middle && lowercase_word(words[index - 1].as_str());
            let weight = if word.chars().skip(1).any(char::is_uppercase)
                && word.chars().any(char::is_lowercase)
            {
                3.0
            } else if (2..=6).contains(&word.chars().count())
                && word.chars().all(char::is_uppercase)
            {
                2.0
            } else if word.chars().any(char::is_alphabetic) && word.chars().any(char::is_numeric) {
                2.0
            } else if title_word(word) && follows_lower {
                1.0
            } else {
                0.0
            };
            if weight > 0.0 {
                add_term(&mut terms, word, weight);
            }
            if title_word(word) && follows_lower {
                let mut end = index + 1;
                while end < words.len() && end < index + 3 && title_word(words[end].as_str()) {
                    let gap = &text[words[end - 1].end()..words[end].start()];
                    if gap.is_empty()
                        || !gap
                            .chars()
                            .all(|c| c.is_whitespace() && c != '\n' && c != '\r')
                    {
                        break;
                    }
                    end += 1;
                }
                if end > index + 1 {
                    add_term(
                        &mut terms,
                        &words[index..end]
                            .iter()
                            .map(|word| word.as_str())
                            .collect::<Vec<_>>()
                            .join(" "),
                        1.5,
                    );
                }
            }
        }
    }
    // Count only lower-case forms of actual candidates, rather than every n-gram.
    let mut lower: HashMap<String, u32> = HashMap::new();
    for text in texts {
        let text = clean_text(text);
        let words: Vec<_> = WORDS.find_iter(&text).collect();
        for index in 1..words.len() {
            if text[words[index - 1].end()..words[index].start()]
                .chars()
                .any(|c| matches!(c, '.' | '!' | '?' | '\n' | '\r'))
            {
                continue;
            }
            let mut phrase = String::new();
            for end in index..usize::min(index + 3, words.len()) {
                if !lowercase_word(words[end].as_str()) {
                    break;
                }
                if end > index {
                    let gap = &text[words[end - 1].end()..words[end].start()];
                    if gap.is_empty()
                        || !gap
                            .chars()
                            .all(|c| c.is_whitespace() && c != '\n' && c != '\r')
                    {
                        break;
                    }
                    phrase.push(' ');
                }
                phrase.push_str(words[end].as_str());
                if terms.contains_key(&phrase) {
                    *lower.entry(phrase.clone()).or_default() += 1;
                }
            }
        }
    }
    let mut candidates: Vec<_> = terms
        .into_iter()
        .filter(|(key, value)| {
            value.count >= 2
                && !existing.contains(key)
                && (value.capitals == 0 || lower.get(key).copied().unwrap_or(0) <= value.capitals)
        })
        .map(|(_, value)| TermCandidate {
            term: value.term,
            count: value.count,
            score: value.weight * (1.0 + value.count as f32).ln(),
        })
        .collect();
    candidates.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.term.cmp(&b.term))
    });
    candidates.truncate(top);
    candidates
}

fn linked(metadata: &Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    false
}
struct Walker<'a> {
    limits: &'a ScanLimits,
    files: usize,
    entries: usize,
    bytes: u64,
    texts: Vec<String>,
    truncated: bool,
}
impl Walker<'_> {
    fn walk(&mut self, root: &Path, depth: usize) -> bool {
        if self.files >= self.limits.max_files {
            self.truncated = true;
            return true;
        }
        let Ok(entries) = fs::read_dir(root) else {
            return false;
        };
        for entry in entries.flatten() {
            self.entries = self.entries.saturating_add(1);
            // Also bound traversal of folders containing no supported files.
            if self.entries > self.limits.max_files.saturating_mul(8) {
                self.truncated = true;
                return true;
            }
            let path = entry.path();
            let Ok(metadata) = fs::symlink_metadata(&path) else {
                continue;
            };
            if linked(&metadata) {
                continue;
            }
            if metadata.is_dir() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if name.starts_with('.')
                    || matches!(name.as_str(), "node_modules" | "target" | "dist" | "build")
                {
                    continue;
                }
                if depth >= self.limits.max_depth {
                    self.truncated = true;
                    continue;
                }
                if self.walk(&path, depth + 1) {
                    return true;
                }
                continue;
            }
            if !metadata.is_file() {
                continue;
            }
            self.files += 1;
            let extension = path
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if matches!(extension.as_str(), "md" | "markdown" | "txt") {
                if metadata.len() > self.limits.max_file_bytes {
                    self.truncated = true;
                } else {
                    let remaining = self.limits.max_total_bytes.saturating_sub(self.bytes);
                    if metadata.len() > remaining {
                        self.truncated = true;
                        return true;
                    }
                    if let Ok(file) = File::open(&path) {
                        let cap = self.limits.max_file_bytes.min(remaining);
                        let mut bytes = Vec::new();
                        if file
                            .take(cap.saturating_add(1))
                            .read_to_end(&mut bytes)
                            .is_ok()
                        {
                            if bytes.len() as u64 > remaining {
                                self.truncated = true;
                                return true;
                            }
                            self.bytes += bytes.len() as u64;
                            if bytes.len() as u64 > self.limits.max_file_bytes {
                                self.truncated = true;
                            } else if let Ok(text) = String::from_utf8(bytes) {
                                self.texts.push(text);
                            }
                        }
                    }
                }
            }
            if self.files >= self.limits.max_files {
                self.truncated = true;
                return true;
            }
        }
        false
    }
}
pub fn scan_folder(root: &Path, limits: &ScanLimits, existing: &[String]) -> ScanReport {
    let mut walker = Walker {
        limits,
        files: 0,
        entries: 0,
        bytes: 0,
        texts: Vec::new(),
        truncated: false,
    };
    if fs::symlink_metadata(root).is_ok_and(|metadata| metadata.is_dir() && !linked(&metadata)) {
        walker.walk(root, 0);
    }
    ScanReport {
        candidates: extract_candidates(&walker.texts, existing, 30),
        files_read: u32::try_from(walker.texts.len()).unwrap_or(u32::MAX),
        truncated: walker.truncated,
    }
}
#[tauri::command]
pub async fn scan_folder_for_terms(
    path: String,
    state: tauri::State<'_, crate::AppState>,
) -> Result<ScanReport, String> {
    let existing = state
        .config
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .dictionary_words
        .clone();
    tauri::async_runtime::spawn_blocking(move || {
        let root = Path::new(&path);
        if !fs::symlink_metadata(root).is_ok_and(|metadata| metadata.is_dir() && !linked(&metadata))
            || fs::read_dir(root).is_err()
        {
            return Err("Choose a readable directory.".to_string());
        }
        Ok(scan_folder(root, &ScanLimits::default(), &existing))
    })
    .await
    .map_err(|_| "The folder scan was interrupted.".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn extracts_all_candidate_classes_with_their_weights() {
        let text = "we use DexScreener and DLMM with Token-2022 on Magic Eden. ".repeat(2);
        let candidates = extract_candidates(&[text], &[], 30);
        for term in ["DexScreener", "DLMM", "Token-2022", "Magic Eden"] {
            assert!(
                candidates.iter().any(|candidate| candidate.term == term),
                "missing {term}"
            );
        }
        let dex = candidates
            .iter()
            .find(|candidate| candidate.term == "DexScreener")
            .unwrap();
        assert_eq!(dex.count, 2);
        assert!((dex.score - 3.295837).abs() < 0.001);
    }

    #[test]
    fn ignores_markup_urls_sentence_starts_singletons_and_existing_words() {
        let text = "---\nFrontSecret: FrontSecret FrontSecret\n---\n\
            The project uses [[Magic Eden]]. The team uses [[Magic Eden]].\n\
            ```text\nwe use CodeSecret CodeSecret\n```\n\
            ~~~\nwe use TildeSecret TildeSecret\n~~~\n\
            we use `InlineSecret InlineSecret` and https://UrlSecret.example/UrlSecret.\n\
            we read [notes](LinkSecret) and [notes](LinkSecret).\n\
            we use Mycel and Mycel and SingleRare today.";
        let candidates = extract_candidates(&[text.into()], &["mycel".into()], 30);
        assert!(candidates
            .iter()
            .any(|candidate| candidate.term == "Magic Eden"));
        for term in [
            "The",
            "FrontSecret",
            "CodeSecret",
            "TildeSecret",
            "InlineSecret",
            "UrlSecret",
            "LinkSecret",
            "SingleRare",
            "Mycel",
        ] {
            assert!(
                !candidates.iter().any(|candidate| candidate.term == term),
                "unexpected {term}"
            );
        }
    }

    #[test]
    fn ranks_frequent_terms_above_rare_terms() {
        let texts = vec![
            "we use FrequentTerm today. ".repeat(10),
            "we use RareTerm today. ".repeat(2),
        ];
        let candidates = extract_candidates(&texts, &[], 30);
        assert_eq!(candidates[0].term, "FrequentTerm");
        assert_eq!(candidates[0].count, 10);
    }

    #[test]
    fn removes_generic_words_when_lowercase_mid_sentence_is_more_common() {
        let text = "we use Useful today. we use Useful again. we find useful things and useful notes and useful examples.";
        assert!(!extract_candidates(&[text.into()], &[], 30)
            .iter()
            .any(|candidate| candidate.term == "Useful"));
    }

    #[test]
    fn excludes_english_and_portuguese_calendar_names_and_work_markers() {
        let text = "we use ProjectAtlas on Monday in January and Segunda-feira in Março with TODO and README. ".repeat(2);
        let candidates = extract_candidates(&[text], &[], 30);
        assert!(candidates
            .iter()
            .any(|candidate| candidate.term == "ProjectAtlas"));
        for term in [
            "Monday",
            "January",
            "Segunda-feira",
            "Março",
            "TODO",
            "README",
        ] {
            assert!(!candidates.iter().any(|candidate| candidate.term == term));
        }
    }

    #[test]
    fn limits_results_and_breaks_score_ties_by_term() {
        let text = "we use ZetaGamma and AlphaBeta. ".repeat(2);
        let candidates = extract_candidates(&[text], &[], 1);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].term, "AlphaBeta");
        assert!(extract_candidates(&["we use DexScreener. ".repeat(2)], &[], 0).is_empty());
    }

    #[test]
    fn scans_only_supported_visible_utf8_text_files() {
        let directory = tempdir().unwrap();
        let root = directory.path();
        fs::write(root.join("a.md"), "we use DexScreener today.").unwrap();
        fs::write(root.join("b.txt"), "we use DexScreener today.").unwrap();
        fs::write(root.join("c.rs"), "we use CodeSecret CodeSecret.").unwrap();
        fs::write(root.join("invalid.txt"), [0xff, 0xfe]).unwrap();
        fs::create_dir(root.join(".hidden")).unwrap();
        fs::write(
            root.join(".hidden/d.md"),
            "we use HiddenSecret HiddenSecret.",
        )
        .unwrap();
        let report = scan_folder(root, &ScanLimits::default(), &[]);
        assert_eq!(report.files_read, 2);
        assert!(!report.truncated);
        assert!(report
            .candidates
            .iter()
            .any(|candidate| candidate.term == "DexScreener"));
    }

    #[test]
    fn scan_reports_file_count_truncation() {
        let directory = tempdir().unwrap();
        fs::write(directory.path().join("a.md"), "first").unwrap();
        fs::write(directory.path().join("b.txt"), "second").unwrap();
        let report = scan_folder(
            directory.path(),
            &ScanLimits {
                max_files: 1,
                ..ScanLimits::default()
            },
            &[],
        );
        assert!(report.truncated);
        assert!(report.files_read <= 1);
    }

    #[test]
    fn scan_respects_total_bytes_and_directory_depth() {
        let directory = tempdir().unwrap();
        fs::write(directory.path().join("a.md"), "text larger than four bytes").unwrap();
        let report = scan_folder(
            directory.path(),
            &ScanLimits {
                max_total_bytes: 4,
                ..ScanLimits::default()
            },
            &[],
        );
        assert!(report.truncated);
        assert_eq!(report.files_read, 0);
        fs::create_dir(directory.path().join("notes")).unwrap();
        fs::write(directory.path().join("notes/b.md"), "nested").unwrap();
        let report = scan_folder(
            directory.path(),
            &ScanLimits {
                max_depth: 0,
                ..ScanLimits::default()
            },
            &[],
        );
        assert!(report.truncated);
        assert_eq!(report.files_read, 1);
    }

    #[test]
    fn scan_skips_oversized_files_and_dependency_directories() {
        let directory = tempdir().unwrap();
        fs::write(directory.path().join("large.txt"), "too large").unwrap();
        for name in ["node_modules", "target", "dist", "build"] {
            fs::create_dir(directory.path().join(name)).unwrap();
            fs::write(directory.path().join(name).join("secret.md"), "x").unwrap();
        }
        let report = scan_folder(
            directory.path(),
            &ScanLimits {
                max_file_bytes: 4,
                ..ScanLimits::default()
            },
            &[],
        );
        assert_eq!(report.files_read, 0);
        assert!(report.truncated);
    }
}
