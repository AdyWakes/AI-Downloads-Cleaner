//! Tier 2 classifier: a small local model served by Ollama.
//!
//! Only *ambiguous* files (per the rules engine) are sent, in batches, as
//! filename + a short text snippet. The model must answer strict JSON; every
//! answer is validated and anything invalid silently falls back to the rules.
//! The endpoint must be on this computer (loopback) — nothing leaves the machine.

use crate::config::{AiConfig, Category, Config, OTHER};
use crate::error::{Error, Result};
use crate::rules::{Classification, Source};
use crate::scanner::FileEntry;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AiStatus {
    /// Ollama answered.
    pub reachable: bool,
    /// The configured model is installed.
    pub model_available: bool,
    pub model: String,
    pub installed_models: Vec<String>,
    /// AI tier will actually be used (enabled + reachable + model available).
    pub active: bool,
    pub enabled: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AiRunReport {
    pub used: bool,
    pub candidates: usize,
    pub classified: usize,
    pub changed: usize,
    pub failed_batches: usize,
    pub message: String,
}

/// Accept only `http://localhost`, `http://127.x.x.x`, `http://[::1]` endpoints.
pub fn check_endpoint(endpoint: &str) -> Result<String> {
    let e = endpoint.trim().trim_end_matches('/');
    let rest = e.strip_prefix("http://").ok_or_else(|| {
        Error::Ai("endpoint must start with http:// and point to this computer".into())
    })?;
    let hostport = rest.split('/').next().unwrap_or("");
    let host = if let Some(h) = hostport.strip_prefix('[') {
        h.split(']').next().unwrap_or("").to_string()
    } else {
        hostport.split(':').next().unwrap_or("").to_lowercase()
    };
    let ok = host == "localhost" || host == "::1" || {
        let parts: Vec<_> = host.split('.').collect();
        parts.len() == 4 && parts[0] == "127" && parts.iter().all(|p| p.parse::<u8>().is_ok())
    };
    if !ok {
        return Err(Error::Ai(format!(
            "AI endpoint must be on this computer (localhost), got '{host}'. tidydrop never sends files elsewhere."
        )));
    }
    Ok(e.to_string())
}

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(2))
        .timeout(timeout)
        .build()
}

fn model_matches(installed: &str, wanted: &str) -> bool {
    let norm = |s: &str| {
        if s.contains(':') {
            s.to_string()
        } else {
            format!("{s}:latest")
        }
    };
    norm(installed) == norm(wanted)
}

pub fn status(cfg: &AiConfig) -> AiStatus {
    let mut st = AiStatus {
        model: cfg.model.clone(),
        enabled: cfg.enabled,
        ..Default::default()
    };
    let base = match check_endpoint(&cfg.endpoint) {
        Ok(b) => b,
        Err(e) => {
            st.message = e.to_string();
            return st;
        }
    };
    let resp = agent(Duration::from_secs(3))
        .get(&format!("{base}/api/tags"))
        .call();
    match resp.map_err(|e| e.to_string()).and_then(|r| {
        r.into_json::<serde_json::Value>()
            .map_err(|e| e.to_string())
    }) {
        Ok(v) => {
            st.reachable = true;
            st.installed_models = v["models"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|m| m["name"].as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            st.model_available = st
                .installed_models
                .iter()
                .any(|m| model_matches(m, &cfg.model));
            st.active = cfg.enabled && st.model_available;
            st.message = if !cfg.enabled {
                "AI is turned off in Settings.".into()
            } else if st.model_available {
                format!("Using {} on this computer.", cfg.model)
            } else {
                format!(
                    "Ollama is running but the model '{}' isn't installed. Run: ollama pull {}",
                    cfg.model, cfg.model
                )
            };
        }
        Err(_) => {
            st.message = "Ollama isn't running. tidydrop is using smart rules only.".into();
        }
    }
    st
}

/// One file as presented to the model.
#[derive(Debug, Clone, Serialize)]
pub struct AiInput {
    pub id: usize,
    pub name: String,
    pub snippet: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AiAnswer {
    results: Vec<AiResultItem>,
}

#[derive(Debug, Deserialize)]
struct AiResultItem {
    id: serde_json::Value,
    category: String,
    #[serde(default)]
    reason: String,
}

pub fn build_prompt(categories: &[Category], files: &[AiInput]) -> String {
    let mut p = String::from(
        "You sort files from a Downloads folder into folders. Choose exactly one category per file \
         from this list (use the name exactly as written):\n",
    );
    for c in categories {
        p.push_str(&format!("- {}: {}\n", c.name, c.description));
    }
    p.push_str("\nFiles (id, filename, and optionally the start of its text):\n");
    for f in files {
        p.push_str(&format!("[{}] {}\n", f.id, f.name));
        if let Some(s) = &f.snippet {
            p.push_str(&format!("    text: {s}\n"));
        }
    }
    p.push_str(
        "\nReply with JSON only, no prose, in exactly this shape:\n\
         {\"results\":[{\"id\":0,\"category\":\"<one category name>\",\"reason\":\"<max 8 words>\"}]}\n\
         Include every id once. If unsure, use \"Other\".",
    );
    p
}

/// Parse and validate a model reply. Returns (id, canonical category, reason) for valid items only.
pub fn parse_answer(
    content: &str,
    categories: &[Category],
    allowed_ids: &[usize],
) -> Vec<(usize, String, String)> {
    // Some models wrap JSON in prose or code fences; take the outermost object.
    let start = content.find('{');
    let end = content.rfind('}');
    let Some((s, e)) = start.zip(end).filter(|(s, e)| s < e) else {
        return vec![];
    };
    let Ok(ans) = serde_json::from_str::<AiAnswer>(&content[s..=e]) else {
        return vec![];
    };
    let mut seen = std::collections::HashSet::new();
    ans.results
        .into_iter()
        .filter_map(|r| {
            let id = match &r.id {
                serde_json::Value::Number(n) => n.as_u64()? as usize,
                serde_json::Value::String(s) => s.trim().trim_matches(['[', ']']).parse().ok()?,
                _ => return None,
            };
            if !allowed_ids.contains(&id) || !seen.insert(id) {
                return None;
            }
            let wanted = r.category.trim().to_lowercase();
            let cat = categories
                .iter()
                .find(|c| c.name.to_lowercase() == wanted)?;
            let reason: String = r
                .reason
                .chars()
                .filter(|c| !c.is_control())
                .take(80)
                .collect();
            Some((id, cat.name.clone(), reason.trim().to_string()))
        })
        .collect()
}

fn ask(cfg: &AiConfig, base: &str, prompt: &str) -> Result<String> {
    let body = json!({
        "model": cfg.model,
        "messages": [{"role": "user", "content": prompt}],
        "stream": false,
        "format": "json",
        "options": {"temperature": 0, "num_predict": 1024},
        "keep_alive": "5m",
    });
    let resp = agent(Duration::from_secs(cfg.timeout_secs.max(5)))
        .post(&format!("{base}/api/chat"))
        .send_json(body)
        .map_err(|e| Error::Ai(e.to_string()))?;
    let v: serde_json::Value = resp.into_json().map_err(|e| Error::Ai(e.to_string()))?;
    v["message"]["content"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| Error::Ai("unexpected reply from Ollama".into()))
}

/// Refine ambiguous rule classifications in place. Any failure leaves the rule result.
/// `progress(done, total)` is called after each batch.
pub fn refine(
    files: &[FileEntry],
    classes: &mut [Classification],
    config: &Config,
    progress: &mut dyn FnMut(usize, usize),
) -> AiRunReport {
    let cfg = &config.ai;
    let mut report = AiRunReport::default();
    let base = match check_endpoint(&cfg.endpoint) {
        Ok(b) => b,
        Err(e) => {
            report.message = e.to_string();
            return report;
        }
    };
    let candidates: Vec<usize> = (0..files.len()).filter(|&i| classes[i].ambiguous).collect();
    report.candidates = candidates.len();
    if candidates.is_empty() {
        report.message = "Rules were confident about every file.".into();
        return report;
    }
    report.used = true;
    let total = candidates.len();
    let mut done = 0;
    progress(0, total);
    let mut consecutive_failures = 0;
    for chunk in candidates.chunks(cfg.batch_size.max(1)) {
        let inputs: Vec<AiInput> = chunk
            .iter()
            .map(|&i| AiInput {
                id: i,
                name: files[i].name.clone(),
                snippet: if files[i].cloud_only {
                    None
                } else {
                    crate::snippet::extract(&files[i].path, &files[i].ext, cfg.max_snippet_chars)
                },
            })
            .collect();
        let prompt = build_prompt(&config.categories, &inputs);
        match ask(cfg, &base, &prompt) {
            Ok(content) => {
                consecutive_failures = 0;
                let answers = parse_answer(&content, &config.categories, chunk);
                if answers.is_empty() {
                    report.failed_batches += 1;
                }
                for (id, cat, reason) in answers {
                    report.classified += 1;
                    let rule = &classes[id];
                    // "Other" from the AI never overrides a real type match from rules.
                    if cat == OTHER && rule.category != OTHER {
                        continue;
                    }
                    if cat != rule.category {
                        report.changed += 1;
                    }
                    classes[id] = Classification {
                        category: cat,
                        reason: if reason.is_empty() {
                            "AI suggestion".into()
                        } else {
                            format!("AI: {reason}")
                        },
                        source: Source::Ai,
                        ambiguous: false,
                    };
                }
            }
            Err(e) => {
                report.failed_batches += 1;
                consecutive_failures += 1;
                report.message = format!("AI request failed ({e}); using rules for those files.");
                if consecutive_failures >= 2 {
                    report.message = format!(
                        "AI stopped after repeated errors ({e}); using rules for the rest."
                    );
                    progress(total, total);
                    return report;
                }
            }
        }
        done += chunk.len();
        progress(done, total);
    }
    if report.message.is_empty() {
        report.message = format!(
            "AI looked at {} unclear files and re-sorted {}.",
            report.candidates, report.changed
        );
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;

    /// Tiny fake Ollama: answers each request with the next canned body.
    fn fake_ollama(bodies: Vec<String>) -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        std::thread::spawn(move || {
            for body in bodies {
                let Ok((mut s, _)) = l.accept() else { return };
                let mut r = BufReader::new(s.try_clone().unwrap());
                let mut len = 0usize;
                loop {
                    let mut line = String::new();
                    if r.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                        break;
                    }
                    if let Some(v) = line.to_lowercase().strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap_or(0);
                    }
                }
                let mut buf = vec![0; len];
                let _ = r.read_exact(&mut buf);
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = s.write_all(resp.as_bytes());
            }
        });
        format!("http://{addr}")
    }

    fn chat(content: &str) -> String {
        serde_json::to_string(&json!({"message": {"role": "assistant", "content": content}}))
            .unwrap()
    }

    fn setup(names: &[&str]) -> (tempfile::TempDir, Vec<FileEntry>, Config) {
        let d = tempfile::tempdir().unwrap();
        for n in names {
            std::fs::write(d.path().join(n), format!("text of {n}")).unwrap();
        }
        let mut c = Config::default();
        c.skip_recent_secs = 0;
        let s = crate::scanner::scan(d.path(), &c).unwrap();
        (d, s.files, c)
    }

    #[test]
    fn endpoint_must_be_local() {
        assert!(check_endpoint("http://127.0.0.1:11434").is_ok());
        assert!(check_endpoint("http://localhost:11434/").is_ok());
        assert!(check_endpoint("http://[::1]:11434").is_ok());
        assert!(check_endpoint("http://192.168.1.5:11434").is_err());
        assert!(check_endpoint("https://api.example.com").is_err());
        assert!(check_endpoint("http://localhost.evil.com").is_err());
        assert!(check_endpoint("http://127.0.0.1.evil.com").is_err());
    }

    #[test]
    fn parse_validates_strictly() {
        let cats = Config::default().categories;
        let ok = parse_answer(
            r#"Sure! ```{"results":[{"id":3,"category":"bills & receipts","reason":"electricity bill"},
               {"id":4,"category":"Made Up","reason":"x"},{"id":99,"category":"PDFs"},{"id":"5","category":"PDFs"},{"id":3,"category":"PDFs"}]}```"#,
            &cats,
            &[3, 4, 5],
        );
        assert_eq!(
            ok,
            vec![
                (3, "Bills & Receipts".into(), "electricity bill".into()),
                (5, "PDFs".into(), "".into())
            ]
        );
        assert!(parse_answer("not json", &cats, &[1]).is_empty());
        assert!(parse_answer(r#"{"results": "nope"}"#, &cats, &[1]).is_empty());
    }

    #[test]
    fn refine_uses_valid_answers_and_keeps_rules_otherwise() {
        let (_d, files, mut c) = setup(&["a.pdf", "b.pdf", "c.xyz", "setup.exe"]);
        let mut classes = crate::plan::classify_with_rules(&files, &c);
        // a.pdf, b.pdf, c.xyz are ambiguous; setup.exe is not
        let body = chat(
            r#"{"results":[{"id":0,"category":"Bills & Receipts","reason":"looks like a bill"},{"id":1,"category":"Bogus"},{"id":2,"category":"Other"}]}"#,
        );
        c.ai.endpoint = fake_ollama(vec![body]);
        c.ai.batch_size = 10;
        let mut calls = vec![];
        let rep = refine(&files, &mut classes, &c, &mut |d, t| calls.push((d, t)));
        assert_eq!(rep.candidates, 3);
        assert_eq!(classes[0].category, "Bills & Receipts");
        assert_eq!(classes[0].source, Source::Ai);
        assert_eq!(classes[1].category, "PDFs");
        assert_eq!(classes[1].source, Source::Rules);
        assert_eq!(classes[2].category, "Other");
        assert_eq!(classes[3].category, "Installers");
        assert_eq!(calls.last(), Some(&(3, 3)));
    }

    #[test]
    fn refine_falls_back_when_ollama_is_down_or_garbled() {
        let (_d, files, mut c) = setup(&["a.pdf", "b.pdf"]);
        let before = crate::plan::classify_with_rules(&files, &c);
        // Nothing listening on this port
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        c.ai.endpoint = format!("http://{}", l.local_addr().unwrap());
        drop(l);
        let mut classes = before.clone();
        let rep = refine(&files, &mut classes, &c, &mut |_, _| {});
        assert_eq!(classes, before);
        assert!(rep.failed_batches >= 1);
        assert!(!status(&c.ai).reachable);

        c.ai.endpoint = fake_ollama(vec![chat("I think these are PDFs!")]);
        let mut classes = before.clone();
        refine(&files, &mut classes, &c, &mut |_, _| {});
        assert_eq!(classes, before);

        c.ai.endpoint = "http://10.0.0.1:11434".into();
        let mut classes = before.clone();
        let rep = refine(&files, &mut classes, &c, &mut |_, _| {});
        assert!(!rep.used);
        assert_eq!(classes, before);
    }

    #[test]
    fn status_reports_models() {
        let mut c = AiConfig::default();
        c.endpoint = fake_ollama(vec![
            r#"{"models":[{"name":"llama3.2:3b"},{"name":"qwen2.5:3b"}]}"#.into(),
        ]);
        let s = status(&c);
        assert!(s.reachable && s.model_available && s.active);
        c.model = "mistral".into();
        c.endpoint = fake_ollama(vec![r#"{"models":[{"name":"llama3.2:3b"}]}"#.into()]);
        let s = status(&c);
        assert!(s.reachable && !s.model_available && !s.active);
        assert!(s.message.contains("ollama pull mistral"));
    }

    #[test]
    fn prompt_contains_categories_and_snippets() {
        let cats = Config::default().categories;
        let p = build_prompt(
            &cats,
            &[AiInput {
                id: 7,
                name: "scan001.pdf".into(),
                snippet: Some("TAX INVOICE GSTIN".into()),
            }],
        );
        assert!(p.contains("[7] scan001.pdf"));
        assert!(p.contains("TAX INVOICE GSTIN"));
        assert!(p.contains("- Bills & Receipts:"));
    }
}
