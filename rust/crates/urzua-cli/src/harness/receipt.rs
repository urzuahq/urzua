//! The receipt of a gated move a person approved at the agent host's prompt
//! (`ADR-102`, amendment of 2026-11-17), and the text that prompt shows.
//!
//! Nothing is stored: approving the host's prompt writes the edit, receipt
//! included, and declining writes nothing. The receipt is a revision-log row in
//! the record, written in the same edit. It is text: anything able to edit the
//! record can write it, so it is never evidence of who approved.

use std::path::Path;

use urzua_core::config::Config;

/// How a receipt row begins, so a reader at a skim does not take it for a
/// verification, and so the local check can find it.
pub(crate) const MARK: &str = "Recorded as approved in an agent session, not verified";
/// How a receipt row ends.
pub(crate) const END: &str = "the pull request check does not rely on it.";

/// What a receipt row says about the move, right after [`MARK`]: the record's
/// id, then `<field> moved from '<from>' to '<to>'`.
pub(crate) fn move_phrase(id: &str, field: &str, from: &str, to: &str) -> String {
    format!("{MARK}: {id} {field} moved from '{from}' to '{to}'")
}

/// Characters that change how text displays without showing themselves, or
/// render blank: bidirectional controls, zero-width and joiner characters,
/// invisible operators, the soft hyphen, the combining grapheme joiner, the
/// Mongolian variation selectors and vowel separator, the Hangul and Braille
/// blank letters, the variation selectors, the tag block and the interlinear
/// annotation controls.
fn invisible(c: char) -> bool {
    matches!(c,
        '\u{00AD}' | '\u{034F}' | '\u{061C}' | '\u{115F}' | '\u{1160}' | '\u{17B4}' | '\u{17B5}'
        | '\u{180B}'..='\u{180F}' | '\u{2800}' | '\u{3164}' | '\u{FE00}'..='\u{FE0F}' | '\u{FFA0}'
        | '\u{E0100}'..='\u{E01EF}' | '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}'
        | '\u{2028}' | '\u{2029}' | '\u{2060}'..='\u{2064}' | '\u{2066}'..='\u{2069}'
        | '\u{FEFF}' | '\u{FFF9}'..='\u{FFFB}' | '\u{E0000}'..='\u{E007F}')
}

/// A combining mark (the Unicode blocks of combining diacritics), which stacks
/// on the character before it.
fn combining(c: char) -> bool {
    matches!(c,
        '\u{0300}'..='\u{036F}' | '\u{1AB0}'..='\u{1AFF}' | '\u{1DC0}'..='\u{1DFF}'
        | '\u{20D0}'..='\u{20FF}' | '\u{FE20}'..='\u{FE2F}')
}

/// `text` as it may appear in a prompt: control and invisible characters,
/// quotes and line breaks dropped, at most two combining marks on a character,
/// runs of space collapsed, at most `max` characters.
pub(crate) fn sanitize(text: &str, max: usize) -> String {
    clean(text, max, true)
}

/// Text from the configuration (a group, a gate, a field name) as the prompt
/// may show it: as [`sanitize`], quotes kept, since urzua's own sentence quotes
/// names with them and the configuration is a protected path.
pub(crate) fn sanitize_config(text: &str, max: usize) -> String {
    clean(text, max, false)
}

fn clean(text: &str, max: usize, drop_quotes: bool) -> String {
    let quote = |c: char| {
        "\"'`\u{2018}\u{2019}\u{201A}\u{201B}\u{201C}\u{201D}\u{201E}\u{201F}\u{00AB}\u{00BB}\u{2039}\u{203A}".contains(c)
    };
    let mut kept = String::new();
    let mut marks = 0;
    for c in text.chars() {
        let c = if c.is_whitespace() { ' ' } else { c };
        if c.is_control() || invisible(c) || (drop_quotes && quote(c)) {
            continue;
        }
        if combining(c) {
            marks += 1;
            if marks > 2 {
                continue;
            }
        } else {
            marks = 0;
        }
        kept.push(c);
    }
    let collapsed = kept
        .split(' ')
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if collapsed.chars().count() <= max {
        collapsed
    } else {
        let cut: String = collapsed.chars().take(max.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

/// Whether the record carries a receipt row for its gated move from `from` to
/// `to`. A receipt row for a move is a row of the record's declared revision
/// log, read as that log with its date and class columns, that is in the head
/// copy and not in the base copy the check compares against, whose narrative
/// begins with "Recorded as approved in an agent session, not verified" and
/// then names this record's id, the status the move leaves and the status it
/// enters, and whose date is
/// the date the check reads for that row's change (`day_of`, by its line). The
/// row is text and unverified: the name in it is not checked.
pub(crate) fn carries_receipt(
    config: &Config,
    (record_type, id): (&str, &str),
    field: &str,
    (from, to): (&str, &str),
    (head, base): (&str, &str),
    day_of: &dyn Fn(usize) -> Option<String>,
) -> bool {
    let (log_id, shape) = (
        config.revision_log_id(record_type),
        config.revision_log_shape(record_type),
    );
    let before: std::collections::HashSet<String> =
        urzua_core::rules::revision_log_rows(base, &log_id, &shape)
            .into_iter()
            .map(|r| r.row)
            .collect();
    let phrase = move_phrase(id, field, from, to);
    urzua_core::rules::revision_log_rows(head, &log_id, &shape)
        .into_iter()
        .filter(|r| !before.contains(&r.row))
        .any(|r| {
            r.row
                .split('|')
                .map(str::trim)
                .any(|cell| cell.starts_with(&phrase))
                && day_of(r.line).is_some_and(|d| d == r.date)
        })
}

/// The changed records whose gated move carries a receipt row, only with
/// `acceptance.in_session: ask`. The base copy is the record at the merge
/// base; a row's date is its commit's day (the later of author and committer),
/// or the evaluation date while it is uncommitted.
pub(crate) fn for_check(
    session: &crate::session::Session,
    changes: Option<&urzua_io::GitChanges>,
) -> Option<std::collections::HashSet<std::path::PathBuf>> {
    let config = &session.config;
    let harness = config.harness.as_ref()?;
    if !harness.enabled("acceptance") || harness.in_session() != urzua_core::harness::InSession::Ask
    {
        return None;
    }
    let changes = changes?;
    let root = &session.repo_root;
    let cache = urzua_io::BlameCache::new(root);
    let mut found = std::collections::HashSet::new();
    for record in session
        .records
        .iter()
        .filter(|r| changes.changed.contains(&r.path))
    {
        let field = urzua_core::rules::relation_field_name(
            config,
            &record.record_type,
            urzua_core::config::RelationRole::Status,
        );
        let Some(to) = urzua_core::authority::gate_status(&record.header, field) else {
            continue;
        };
        let Ok(Some(base)) =
            urzua_io::read_at_commit_if_present(root, &changes.merge_base, &record.path)
        else {
            continue;
        };
        let Some(from) = crate::discovery::parse_prospective_record(config, &record.path, &base)
            .ok()
            .and_then(|b| urzua_core::authority::gate_status(&b.header, field).map(str::to_string))
        else {
            continue;
        };
        let Ok(head) = std::fs::read_to_string(root.join(&record.path)) else {
            continue;
        };
        let day_of = |line: usize| -> Option<String> {
            let commit = cache.commit_for_line(&record.path, line, None).ok()??;
            if commit.chars().all(|c| c == '0') {
                return Some(urzua_io::today());
            }
            let t = cache
                .commit_author_unix_time(&commit)
                .into_iter()
                .chain(cache.commit_unix_time(&commit))
                .max()?;
            urzua_io::date_of_unix(t)
        };
        let Some(id) = urzua_core::rules::record_id(record) else {
            continue;
        };
        if carries_receipt(
            config,
            (&record.record_type, &id),
            field,
            (&from, to),
            (&head, &base),
            &day_of,
        ) {
            found.insert(record.path.clone());
        }
    }
    Some(found)
}

/// Who the session says is at the keyboard: the `gh` login, else
/// `git config user.name`. Either is whatever the user's environment reports.
pub(crate) struct Approver {
    pub name: String,
    pub source: &'static str,
}

pub(crate) fn approver(repo_root: &Path, config: &Config) -> Result<Approver, String> {
    let policy = urzua_io::IdentityPolicy {
        asserted: config.attestation.identity == urzua_core::config::AttestationIdentity::Asserted,
        require_verified: config.attestation.require_verified,
    };
    let r = urzua_io::resolve_identity_with(None, repo_root, policy)?;
    Ok(match (r.source, r.verified_login) {
        (urzua_io::IdentitySource::GhVerified, _) => Approver {
            name: r.name,
            source: "gh",
        },
        (_, Some(login)) => Approver {
            name: login,
            source: "gh",
        },
        _ => Approver {
            name: r.name,
            source: "git config",
        },
    })
}

/// `content` with the receipt of an approved move: each field the entered
/// status requires that is absent or blank and that `date.field-valid`
/// declares a date gets `today`, and the revision log gets the row
/// `narrative`. The engine names no field: both come from the configuration.
pub(crate) fn with_receipt(
    config: &Config,
    record_type: &str,
    content: &str,
    narrative: &str,
    to: &str,
    today: &str,
) -> Result<(String, Vec<String>), String> {
    // Worked on with bare line feeds and written back in the file's own ending.
    let crlf = content.contains("\r\n") && !content.replace("\r\n", "").contains('\n');
    let mut text = content.replace("\r\n", "\n");
    let date_fields: Vec<String> = config
        .rules
        .get(urzua_core::rules::RULE_DATE_FIELD_VALID)
        .and_then(|s| s.date_fields.clone())
        .unwrap_or_default();
    let mut filled = Vec::new();
    if let Some(decl) = config
        .record_types
        .get(record_type)
        .and_then(|t| t.status_decl())
    {
        let header = header_values(&text);
        let value = |f: &str| header.iter().find(|(k, _)| k == f).map(|(_, v)| v.clone());
        let blank = |v: &str| matches!(v.trim(), "" | "null" | "~");
        filled = decl
            .required_for(to, &value)
            .into_iter()
            .filter(|t| t.on_status && date_fields.iter().any(|d| d == t.field))
            .filter(|t| value(t.field).is_none_or(|v| blank(&v)))
            .map(|t| t.field.to_string())
            .collect();
        for field in &filled {
            text = set_header_field(&text, field, today)?;
        }
    }
    let log_id = config.revision_log_id(record_type);
    let shape = config.revision_log_shape(record_type);
    let text = urzua_core::fix::append_revision_row(&text, &log_id, &shape, today, narrative)?;
    Ok((
        if crlf {
            text.replace('\n', "\r\n")
        } else {
            text
        },
        filled,
    ))
}

/// The `key: value` lines of the front matter, as written.
fn header_values(text: &str) -> Vec<(String, String)> {
    let mut lines = text.lines();
    if lines.next().map(str::trim_end) != Some("---") {
        return Vec::new();
    }
    lines
        .take_while(|l| l.trim_end() != "---")
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

/// `text` with `field: value` in its front matter: a blank existing line is
/// filled, else the field is added before the closing `---`.
fn set_header_field(text: &str, field: &str, value: &str) -> Result<String, String> {
    let mut out: Vec<String> = text.split('\n').map(str::to_string).collect();
    if out.first().map(|l| l.trim_end()) != Some("---") {
        return Err(format!(
            "the record has no front matter to write '{field}:' in"
        ));
    }
    for i in 1..out.len() {
        let line = out[i].trim_end().to_string();
        if line == "---" {
            out.insert(i, format!("{field}: {value}"));
            return Ok(out.join("\n"));
        }
        if let Some(v) = line.strip_prefix(field).and_then(|r| r.strip_prefix(':')) {
            if matches!(v.trim(), "" | "null" | "~") {
                out[i] = format!("{field}: {value}");
                return Ok(out.join("\n"));
            }
        }
    }
    Err(format!(
        "the record's front matter is not closed, so '{field}:' cannot be written"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_header_field_is_filled_when_blank_and_added_when_absent() {
        let t = "---\nStatus: Accepted\nAccepted-on: null\n---\n# T\n";
        assert_eq!(
            set_header_field(t, "Accepted-on", "2026-11-17").unwrap(),
            "---\nStatus: Accepted\nAccepted-on: 2026-11-17\n---\n# T\n"
        );
        let t = "---\nStatus: Accepted\n---\n# T\n";
        assert_eq!(
            set_header_field(t, "Accepted-on", "2026-11-17").unwrap(),
            "---\nStatus: Accepted\nAccepted-on: 2026-11-17\n---\n# T\n"
        );
        assert!(set_header_field("# T\n", "X", "1").is_err());
    }

    #[test]
    fn blank_rendering_characters_are_dropped_from_a_title() {
        let raw = "A\u{3164}B\u{115F}C\u{2800}D\u{034F}E\u{FE0F}F\u{E0100}G\u{180B}H\u{180F}I";
        assert_eq!(sanitize(raw, 80), "ABCDEFGHI");
    }

    #[test]
    fn a_title_loses_controls_invisibles_quotes_and_breaks_and_is_capped() {
        let raw = "ADR-1: Fix \"typo\"\r\u{1b}[2J\u{202E}mo\u{2060}v\u{AD}e\u{180E}s\u{E0041}\u{2062}\u{2063}\u{FFF9}\nStatus 'x'"
            .to_string()
            + &"a\u{301}\u{301}\u{301}\u{301}".repeat(10)
            + &"a".repeat(3000);
        let t = sanitize(&raw, 80);
        assert!(t.chars().count() <= 80, "{t}");
        assert!(
            !t.chars()
                .any(|c| c.is_control() || invisible(c) || "\"'".contains(c)),
            "{t:?}"
        );
        assert!(
            t.starts_with("ADR-1: Fix typo [2Jmoves Status xa\u{301}\u{301}a\u{301}\u{301}a"),
            "{t:?}"
        );
    }
}
