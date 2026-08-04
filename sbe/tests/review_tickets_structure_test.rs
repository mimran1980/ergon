//! Structural gate for the repo-root hand-off `REVIEW_TICKETS.md`.
//!
//! This is the durable acceptance check for the review-ticket deliverable:
//! a fresh LLM must receive imperative tickets with required fields and
//! roadmap labels — not a free-form memo.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]

use std::error::Error;
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sbe parent is repo root")
        .to_path_buf()
}

fn review_tickets_path() -> PathBuf {
    repo_root().join("REVIEW_TICKETS.md")
}

#[test]
fn review_tickets_exists_and_is_nonempty() -> Result<(), Box<dyn Error>> {
    let path = review_tickets_path();
    assert!(
        path.is_file(),
        "REVIEW_TICKETS.md missing at {}",
        path.display()
    );
    let text = fs::read_to_string(&path)?;
    assert!(!text.trim().is_empty(), "REVIEW_TICKETS.md is empty");
    Ok(())
}

#[test]
fn review_tickets_section_order_and_roadmap_labels() -> Result<(), Box<dyn Error>> {
    let text = fs::read_to_string(review_tickets_path())?;
    let qw = text
        .find("# 1. Quick wins")
        .ok_or("missing Quick wins section")?;
    let main = text
        .find("# 2. Main tickets")
        .ok_or("missing Main tickets section")?;
    let road = text
        .find("# 3. Roadmap cross-check")
        .ok_or("missing Roadmap cross-check section")?;
    assert!(
        qw < main && main < road,
        "section order must be Quick wins → Main → Roadmap"
    );
    assert!(
        text.contains("already planned"),
        "roadmap must label tickets already planned"
    );
    assert!(
        text.contains("**new**") || text.contains("`new`"),
        "roadmap must label genuinely new tickets"
    );
    assert!(
        text.contains("road-to-1.0"),
        "roadmap cross-check must reference road-to-1.0.md"
    );
    let road_file = repo_root().join("book/src/project/road-to-1.0.md");
    assert!(
        road_file.is_file(),
        "roadmap anchor missing: {}",
        road_file.display()
    );
    // 1.0-only (if present) must not precede main tickets
    if let Some(one) = text.find("1.0-only") {
        assert!(
            one > main,
            "1.0-only items must not be mixed into quick wins/main"
        );
    }
    Ok(())
}

#[test]
fn review_tickets_each_ticket_has_required_fields_and_cite() -> Result<(), Box<dyn Error>> {
    let text = fs::read_to_string(review_tickets_path())?;
    let mut tickets: Vec<(String, String)> = Vec::new();
    let mut cur_id: Option<String> = None;
    let mut body = String::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("## T-") {
            if let Some(id) = cur_id.take() {
                tickets.push((id, std::mem::take(&mut body)));
            }
            let num = rest.split(':').next().unwrap_or("");
            cur_id = Some(format!("T-{num}"));
            body.clear();
        } else if cur_id.is_some() {
            if line.starts_with("# ") {
                if let Some(id) = cur_id.take() {
                    tickets.push((id, std::mem::take(&mut body)));
                }
            } else {
                body.push_str(line);
                body.push('\n');
            }
        }
    }
    if let Some(id) = cur_id {
        tickets.push((id, body));
    }

    assert!(
        tickets.len() >= 10,
        "expected a full hand-off list, found {} tickets",
        tickets.len()
    );

    let required = [
        "Type:",
        "Stage:",
        "Priority",
        "Effort",
        "Symptom:",
        "Change:",
        "Acceptance criteria:",
        "Verification plan:",
    ];
    let cite = regex_lite_cite();

    for (id, body) in &tickets {
        for field in required {
            assert!(body.contains(field), "{id} missing required field {field}");
        }
        let num: u32 = id.trim_start_matches("T-").parse()?;
        // Code-facing 0.1.13 tickets need file:line evidence.
        if num < 100 {
            assert!(
                cite.is_match(body),
                "{id} missing path:line citation (e.g. sbe/src/foo.rs:123)"
            );
        }
        if body.contains("Type: PERF") {
            let lower = body.to_lowercase();
            assert!(
                lower.contains("mechanism")
                    || lower.contains("criterion")
                    || lower.contains("no-lto")
                    || lower.contains("evidence"),
                "{id} PERF ticket needs mechanism/evidence language"
            );
        }
        if body.contains("Type: DOCS") {
            assert!(
                body.contains("book/")
                    || body.contains("README")
                    || body.contains("rustdoc")
                    || body.contains("road-to-1.0")
                    || body.contains("api-freeze")
                    || body.contains("feature-matrix")
                    || body.contains("generated-code")
                    || body.contains("aeron-try-claim")
                    || body.contains("encode-decode")
                    || body.contains("nullval"),
                "{id} DOCS ticket needs a concrete book/README/rustdoc location"
            );
        }
    }

    // At least one quick-win PERF and one DOCS among 0.1.13 tickets.
    let zero = tickets
        .iter()
        .filter(|(id, _)| {
            id.trim_start_matches("T-")
                .parse::<u32>()
                .map(|n| n < 100)
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    assert!(
        zero.iter().any(|(_, b)| b.contains("Type: PERF")),
        "quick/main list should include at least one PERF ticket"
    );
    assert!(
        zero.iter().any(|(_, b)| b.contains("Type: DOCS")),
        "quick/main list should include at least one DOCS ticket"
    );
    Ok(())
}

/// Minimal path:line detector without a regex crate dependency.
fn regex_lite_cite() -> Cite {
    Cite
}

struct Cite;
impl Cite {
    fn is_match(&self, body: &str) -> bool {
        // e.g. sbe/src/codegen/foo.rs:394 or book/src/x.md:28
        for token in body.split_whitespace() {
            let t = token.trim_matches(|c: char| {
                !c.is_ascii_alphanumeric()
                    && c != '/'
                    && c != '.'
                    && c != '_'
                    && c != '-'
                    && c != ':'
            });
            if let Some((path, line)) = t.rsplit_once(':') {
                if (path.ends_with(".rs") || path.ends_with(".md"))
                    && line.chars().next().is_some_and(|c| c.is_ascii_digit())
                    && path.contains('/')
                {
                    return true;
                }
            }
        }
        // also match table cells like `sbe/tests/golden/car_example.rs:1173`
        body.contains(".rs:") || body.contains(".md:")
    }
}
