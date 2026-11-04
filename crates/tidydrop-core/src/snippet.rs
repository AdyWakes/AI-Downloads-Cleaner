//! Short text snippets for the AI tier: first lines of text files, first page of
//! PDFs, start of .docx. Read-only, size-capped, never panics.

use std::fs::File;
use std::io::Read;
use std::path::Path;

const TEXT_EXTS: &[&str] = &[
    "txt", "md", "csv", "tsv", "log", "json", "xml", "html", "htm", "rtf", "eml", "ini", "yaml",
    "yml",
];
const MAX_PDF_BYTES: u64 = 20 * 1024 * 1024;
const MAX_DOCX_XML_BYTES: u64 = 4 * 1024 * 1024;

fn collapse(s: &str, max_chars: usize) -> String {
    let mut out = String::new();
    let mut last_space = true;
    for c in s.chars() {
        if out.chars().count() >= max_chars {
            break;
        }
        if c.is_whitespace() || c.is_control() {
            if !last_space {
                out.push(' ');
                last_space = true;
            }
        } else {
            out.push(c);
            last_space = false;
        }
    }
    out.trim().to_string()
}

fn read_prefix(path: &Path, n: u64) -> Option<Vec<u8>> {
    let mut buf = Vec::new();
    File::open(path).ok()?.take(n).read_to_end(&mut buf).ok()?;
    Some(buf)
}

/// Strip XML/HTML tags and decode the common entities.
pub fn strip_tags(xml: &str) -> String {
    let xml = xml.replace("</w:p>", "\n").replace("<w:tab/>", " ");
    let mut out = String::with_capacity(xml.len() / 2);
    let mut in_tag = false;
    for c in xml.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                out.push(' ');
            }
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
}

fn docx_text(path: &Path) -> Option<String> {
    let f = File::open(path).ok()?;
    let mut zip = zip::ZipArchive::new(f).ok()?;
    let entry = zip.by_name("word/document.xml").ok()?;
    let mut xml = String::new();
    entry
        .take(MAX_DOCX_XML_BYTES)
        .read_to_string(&mut xml)
        .ok()?;
    Some(strip_tags(&xml))
}

fn pdf_text(path: &Path) -> Option<String> {
    if std::fs::metadata(path).ok()?.len() > MAX_PDF_BYTES {
        return None;
    }
    let path = path.to_path_buf();
    // PDF parsers can panic on malformed files; contain it.
    std::panic::catch_unwind(move || {
        let doc = lopdf::Document::load(&path).ok()?;
        let first = *doc.get_pages().keys().next()?;
        doc.extract_text(&[first]).ok()
    })
    .ok()
    .flatten()
}

/// Up to `max_chars` characters of text, or `None` if there's nothing useful.
pub fn extract(path: &Path, ext: &str, max_chars: usize) -> Option<String> {
    if max_chars == 0 {
        return None;
    }
    let raw = match ext {
        "pdf" => pdf_text(path)?,
        "docx" => docx_text(path)?,
        e if TEXT_EXTS.contains(&e) => {
            let bytes = read_prefix(path, 16 * 1024)?;
            let text = String::from_utf8_lossy(&bytes).into_owned();
            if e == "html" || e == "htm" || e == "xml" {
                strip_tags(&text)
            } else {
                text
            }
        }
        _ => return None,
    };
    let s = collapse(&raw, max_chars);
    if s.chars().filter(|c| c.is_alphanumeric()).count() < 3 {
        None
    } else {
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn text_snippet_capped_and_collapsed() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a.txt");
        std::fs::write(&p, "Hello\n\n   world\tthis is   a test ".repeat(100)).unwrap();
        let s = extract(&p, "txt", 30).unwrap();
        assert_eq!(s.chars().count(), 30);
        assert!(s.starts_with("Hello world this is a test"));
    }

    #[test]
    fn docx_snippet() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a.docx");
        let f = File::create(&p).unwrap();
        let mut z = zip::ZipWriter::new(f);
        z.start_file(
            "word/document.xml",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        z.write_all(b"<w:document><w:body><w:p><w:r><w:t>Tax Invoice</w:t></w:r></w:p><w:p><w:t>GSTIN 29ABC &amp; Co</w:t></w:p></w:body></w:document>").unwrap();
        z.finish().unwrap();
        let s = extract(&p, "docx", 200).unwrap();
        assert!(s.contains("Tax Invoice"), "{s}");
        assert!(s.contains("GSTIN 29ABC & Co"), "{s}");
    }

    #[test]
    fn garbage_is_none_not_panic() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("bad.pdf");
        std::fs::write(&p, b"%PDF-1.4 garbage \x00\x01\x02").unwrap();
        assert!(extract(&p, "pdf", 100).is_none());
        let p = d.path().join("bad.docx");
        std::fs::write(&p, b"not a zip").unwrap();
        assert!(extract(&p, "docx", 100).is_none());
        assert!(extract(&p, "exe", 100).is_none());
    }
}
