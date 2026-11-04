//! Tier 1 classifier: extension + filename keyword rules. Always available.

use crate::config::{Config, Rule, OTHER};
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Rules,
    Ai,
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Classification {
    pub category: String,
    /// Human-readable reason shown in the preview.
    pub reason: String,
    pub source: Source,
    /// True when rules are not sure (generic PDF/doc, or no rule matched); the AI tier may refine.
    pub ambiguous: bool,
}

/// Lowercase words of a name, split on separators (Unicode word boundaries, so
/// Devanagari etc. work) and *also* on letter/digit boundaries:
/// `GST_Invoice2024-final` -> [gst, invoice, 2024, final].
pub fn words(stem: &str) -> Vec<String> {
    let mut out = Vec::new();
    for w in stem.replace('_', " ").unicode_words() {
        let w = w.to_lowercase();
        let mut cur = String::new();
        let mut cur_digit: Option<bool> = None;
        for g in w.graphemes(true) {
            let is_digit = g.chars().all(|c| c.is_ascii_digit());
            if cur_digit.is_some_and(|d| d != is_digit) {
                out.push(std::mem::take(&mut cur));
            }
            cur_digit = Some(is_digit);
            cur.push_str(g);
        }
        if !cur.is_empty() {
            out.push(cur);
        }
    }
    out
}

/// Lowercase words split only on separators: `Invoice2024` stays `invoice2024`.
pub fn raw_words(stem: &str) -> Vec<String> {
    stem.replace('_', " ")
        .unicode_words()
        .map(|w| w.to_lowercase())
        .collect()
}

/// Lowercase extension without the dot (`Photo.JPG` -> `jpg`). Empty if none.
pub fn extension(file_name: &str) -> String {
    let (_, ext) = crate::paths::split_name(file_name);
    // for .tar.gz report the last part ("gz")
    ext.rsplit('.').next().unwrap_or("").to_lowercase()
}

enum Matcher {
    /// Keyword words (split form, raw form). Matches if either sequence appears
    /// consecutively in the corresponding filename word list.
    Words(Vec<String>, Vec<String>),
    Re(Regex),
}

struct CompiledRule {
    rule: Rule,
    matchers: Vec<(String, Matcher)>,
}

/// Rules compiled once per scan.
pub struct RuleEngine {
    rules: Vec<CompiledRule>,
}

impl RuleEngine {
    pub fn new(config: &Config) -> RuleEngine {
        let rules = config
            .rules
            .iter()
            .filter(|r| config.category(&r.category).is_some())
            .map(|r| CompiledRule {
                rule: r.clone(),
                matchers: r
                    .keywords
                    .iter()
                    .filter_map(|k| {
                        let m = if let Some(re) = k.strip_prefix("re:") {
                            Matcher::Re(RegexBuilder::new(re).case_insensitive(true).build().ok()?)
                        } else {
                            let (split, raw) = (words(k), raw_words(k));
                            if split.is_empty() {
                                return None;
                            }
                            Matcher::Words(split, raw)
                        };
                        Some((k.clone(), m))
                    })
                    .collect(),
            })
            .collect();
        RuleEngine { rules }
    }

    pub fn classify(&self, file_name: &str) -> Classification {
        let ext = extension(file_name);
        let (stem, _) = crate::paths::split_name(file_name);
        let ws = words(&stem);
        let raw = raw_words(&stem);

        for cr in &self.rules {
            let r = &cr.rule;
            if !r.extensions.is_empty() && !r.extensions.iter().any(|e| e == &ext) {
                continue;
            }
            if r.keywords.is_empty() {
                if r.extensions.is_empty() {
                    continue; // a rule with no conditions would match everything; ignore it
                }
                return Classification {
                    category: r.category.clone(),
                    reason: format!(".{ext} file"),
                    source: Source::Rules,
                    ambiguous: !r.confident,
                };
            }
            for (kw, m) in &cr.matchers {
                let hit = match m {
                    Matcher::Re(re) => re.is_match(&stem),
                    Matcher::Words(split, rawk) => {
                        contains_seq(&ws, split) || contains_seq(&raw, rawk)
                    }
                };
                if hit {
                    let shown = kw
                        .strip_prefix("re:")
                        .map(|_| "name pattern".to_string())
                        .unwrap_or(format!("\"{kw}\""));
                    return Classification {
                        category: r.category.clone(),
                        reason: format!("name contains {shown}"),
                        source: Source::Rules,
                        ambiguous: !r.confident,
                    };
                }
            }
        }
        Classification {
            category: OTHER.to_string(),
            reason: if ext.is_empty() {
                "no extension".into()
            } else {
                format!("unknown type .{ext}")
            },
            source: Source::Rules,
            ambiguous: true,
        }
    }
}

fn contains_seq(hay: &[String], needle: &[String]) -> bool {
    if needle.is_empty() || needle.len() > hay.len() {
        return false;
    }
    hay.windows(needle.len()).any(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cls(name: &str) -> Classification {
        RuleEngine::new(&Config::default()).classify(name)
    }

    #[test]
    fn words_split() {
        assert_eq!(
            words("GST_Invoice2024-final"),
            vec!["gst", "invoice", "2024", "final"]
        );
        assert_eq!(
            words("Screen Shot 2023-01-01 at 10.00.00"),
            vec!["screen", "shot", "2023", "01", "01", "at", "10", ".", "00", ".", "00"]
        );
        assert!(words("बिजली का बिल मार्च").contains(&"बिल".to_string()));
    }

    #[test]
    fn extensions() {
        assert_eq!(cls("setup.EXE").category, "Installers");
        assert_eq!(cls("song.mp3").category, "Audio");
        assert_eq!(cls("movie.MKV").category, "Videos");
        assert_eq!(cls("photo.jpeg").category, "Images");
        assert_eq!(cls("backup.tar.gz").category, "Archives");
        assert_eq!(cls("data.csv").category, "Spreadsheets");
        assert_eq!(cls("deck.pptx").category, "Presentations");
        assert_eq!(cls("letter.docx").category, "Documents");
        assert_eq!(cls("main.py").category, "Code");
        assert_eq!(cls("app.ts").category, "Code");
        assert_eq!(cls("random.pdf").category, "PDFs");
        assert_eq!(cls("mystery.xyz").category, "Other");
        assert_eq!(cls("LICENSE").category, "Other");
    }

    #[test]
    fn keywords_win_over_extensions() {
        assert_eq!(cls("Amazon_Invoice_12345.pdf").category, "Bills & Receipts");
        assert_eq!(cls("GST-return-Q2.pdf").category, "Bills & Receipts");
        assert_eq!(cls("order-confirmation.html").category, "Bills & Receipts");
        assert_eq!(cls("receipt photo.JPG").category, "Bills & Receipts");
        assert_eq!(cls("INV-2024-001.pdf").category, "Bills & Receipts");
        assert_eq!(cls("Unit 3 Lecture Notes.pdf").category, "College/Study");
        assert_eq!(cls("DBMS_Assignment2.docx").category, "College/Study");
        assert_eq!(
            cls("Screenshot 2024-05-01 101010.png").category,
            "Screenshots"
        );
        assert_eq!(
            cls("Screen Shot 2023-01-01 at 10.00.00.png").category,
            "Screenshots"
        );
        assert_eq!(
            cls("Screenshot_20240501-101010.jpg").category,
            "Screenshots"
        );
        // keyword but non-matching extension falls through to extension rules
        assert_eq!(cls("invoice-generator-setup.exe").category, "Installers");
        assert_eq!(cls("screenshot-tool.zip").category, "Archives");
    }

    #[test]
    fn whole_words_only() {
        // "bill" must not match inside "billboard"; "order" not inside "border"
        assert_eq!(cls("billboard-design.pdf").category, "PDFs");
        assert_eq!(cls("border.png").category, "Images");
        assert_eq!(cls("My e-bill March.pdf").category, "Bills & Receipts");
    }

    #[test]
    fn hindi_and_case() {
        assert_eq!(cls("बिजली का बिल.pdf").category, "Bills & Receipts");
        assert_eq!(cls("दुकान रसीद.jpg").category, "Bills & Receipts");
        assert_eq!(cls("INVOICE.PDF").category, "Bills & Receipts");
        assert_eq!(cls("हिंदी कहानी.pdf").category, "PDFs");
    }

    #[test]
    fn ambiguity_flags() {
        assert!(cls("random.pdf").ambiguous);
        assert!(cls("mystery.xyz").ambiguous);
        assert!(cls("notes.txt").category == "College/Study");
        assert!(!cls("setup.exe").ambiguous);
        assert!(!cls("invoice.pdf").ambiguous);
    }

    #[test]
    fn custom_rule_and_regex() {
        let mut c = Config::default();
        c.rules.insert(
            0,
            Rule {
                category: "Other".into(),
                extensions: vec![],
                keywords: vec!["re:^tmp_\\d+$".into()],
                confident: true,
            },
        );
        let e = RuleEngine::new(&c);
        assert_eq!(e.classify("tmp_123.pdf").category, "Other");
        assert_eq!(e.classify("tmp_abc.pdf").category, "PDFs");
    }
}
