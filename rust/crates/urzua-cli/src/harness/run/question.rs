//! The question route of `in_session: ask` (`ADR-105`): where the host shows
//! the person no prompt for an edit (a permission mode that approves without
//! asking), the refusal of a gated move hands the agent one question whose
//! wording urzua wrote. A hook before the question tool runs refuses a call
//! whose input already carries answers and marks the question as asked; the
//! hook after it reads the answer the host reports and stores an approving one
//! as a one-use approval in the git directory, which the edit tools cannot
//! write. The next edit making the same move, with the same proposed content,
//! in the same session, consumes it and is written with its receipt.
//!
//! The approval is only as good as the answer the host reports: an answer
//! supplied by another hook or by a host program is not told apart from a
//! person's, the same trust the host's permission prompt already carries, and
//! a shell command can write the git directory (the documented accepted risk).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::super::hook::pinned_git;

/// How every urzua question begins; the challenge's nonce follows.
pub(crate) const PREFIX: &str = "urzua approval ";
/// The choice that declines.
pub(crate) const DECLINE: &str = "Decline";
/// Seconds a challenge may wait to be asked and answered.
const ASK_TTL: u64 = 600;
/// Seconds an approval may wait for the edit that consumes it.
pub(crate) const APPROVED_TTL: u64 = 300;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct Challenge {
    pub nonce: String,
    pub rel: String,
    pub id: String,
    pub field: String,
    pub from: String,
    pub to: String,
    /// sha256 of the content the agent proposed, without the receipt.
    pub body: String,
    pub session: String,
    pub created: u64,
    /// `pending`, `asked` (the question tool was called with it) or `approved`.
    pub state: String,
    #[serde(default)]
    pub call: Option<String>,
    #[serde(default)]
    pub answered: Option<u64>,
    /// The host the answering hook ran under, as its environment names it.
    #[serde(default)]
    pub host: Option<String>,
    pub question: String,
    pub approve: String,
}

pub(crate) fn now() -> u64 {
    u64::try_from(urzua_io::now_unix()).unwrap_or(0)
}

/// `<git common dir>/urzua/approvals` of the repository at `root`.
pub(crate) fn store(root: &Path) -> Option<PathBuf> {
    let out = pinned_git(root)
        .args(["rev-parse", "--git-common-dir"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let dir = String::from_utf8(out.stdout).ok()?;
    let dir = PathBuf::from(dir.trim_end_matches('\n'));
    let dir = if dir.is_absolute() {
        dir
    } else {
        root.join(dir)
    };
    Some(dir.join("urzua").join("approvals"))
}

fn live(c: &Challenge, at: u64) -> bool {
    match (c.state.as_str(), c.answered) {
        ("approved", Some(a)) => at.saturating_sub(a) <= APPROVED_TTL,
        ("pending" | "asked", _) => at.saturating_sub(c.created) <= ASK_TTL,
        _ => false,
    }
}

fn file(dir: &Path, nonce: &str) -> Option<PathBuf> {
    (!nonce.is_empty() && nonce.chars().all(|c| c.is_ascii_hexdigit()))
        .then(|| dir.join(format!("{nonce}.json")))
}

pub(crate) fn load(dir: &Path, nonce: &str) -> Option<Challenge> {
    let text = std::fs::read_to_string(file(dir, nonce)?).ok()?;
    serde_json::from_str(&text).ok()
}

fn all(dir: &Path) -> Vec<Challenge> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .filter_map(|t| serde_json::from_str(&t).ok())
        .collect()
}

pub(crate) fn save(dir: &Path, c: &Challenge) -> Result<(), String> {
    let path = file(dir, &c.nonce).ok_or("the challenge has no usable nonce")?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let tmp = dir.join(format!(".{}.tmp", c.nonce));
    let text = serde_json::to_string(c).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

pub(crate) fn remove(dir: &Path, nonce: &str) {
    if let Some(p) = file(dir, nonce) {
        let _ = std::fs::remove_file(p);
    }
}

fn prune(dir: &Path, at: u64) {
    for c in all(dir) {
        if !live(&c, at) {
            remove(dir, &c.nonce);
        }
    }
}

/// The move a challenge is for, each value already checked against the
/// configuration by the caller.
pub(crate) struct Move<'a> {
    pub rel: &'a str,
    pub id: &'a str,
    pub field: &'a str,
    pub from: &'a str,
    pub to: &'a str,
    pub gate: &'a str,
    pub group: &'a str,
}

/// Stores a pending challenge for `mv` and returns it with the exact input of
/// the question tool the agent is to call. An open challenge for the same
/// record and session is replaced.
pub(crate) fn issue(
    dir: &Path,
    nonce: &str,
    mv: &Move<'_>,
    body: &str,
    session: &str,
) -> Result<(Challenge, serde_json::Value), String> {
    let at = now();
    prune(dir, at);
    for c in all(dir) {
        if c.rel == mv.rel && c.session == session && c.state != "approved" {
            remove(dir, &c.nonce);
        }
    }
    let Move {
        rel,
        id,
        field,
        from,
        to,
        gate,
        group,
    } = *mv;
    let shown_rel = super::super::receipt::sanitize(rel, 120);
    let shown_from = super::super::receipt::sanitize(from, 40);
    let question = format!(
        "{PREFIX}{nonce}: approve {id} moving {field} from '{shown_from}' to '{to}'? {gate}: only the decider group '{group}' may decide. Approving lets the agent's next edit of {shown_rel} write the content it proposed, with a revision-log row recording this approval by you. Decline unless you are in '{group}' and mean to make this move."
    );
    let approve = format!("Approve {nonce}");
    let challenge = Challenge {
        nonce: nonce.to_string(),
        rel: rel.to_string(),
        id: id.to_string(),
        field: field.to_string(),
        from: from.to_string(),
        to: to.to_string(),
        body: body.to_string(),
        session: session.to_string(),
        created: at,
        state: "pending".to_string(),
        call: None,
        answered: None,
        host: None,
        question: question.clone(),
        approve: approve.clone(),
    };
    save(dir, &challenge)?;
    let input = serde_json::json!({
        "questions": [{
            "question": question,
            "header": "urzua",
            "options": [
                {"label": approve, "description": format!("Move {id} to '{to}' with a receipt row")},
                {"label": DECLINE, "description": "Leave the record as it is"},
            ],
            "multiSelect": false,
        }]
    });
    Ok((challenge, input))
}

/// Takes the approval for this move so no other edit can use it: approved,
/// unexpired, and for the same record, statuses, proposed content and session.
/// Claiming is a rename, so of two edits racing for one approval one gets it.
pub(crate) fn consume(
    dir: &Path,
    (rel, from, to): (&str, &str, &str),
    body: &str,
    session: &str,
) -> Option<Challenge> {
    let at = now();
    let found = all(dir).into_iter().find(|c| {
        c.state == "approved"
            && live(c, at)
            && c.rel == rel
            && c.from == from
            && c.to == to
            && c.body == body
            && c.session == session
    })?;
    let path = file(dir, &found.nonce)?;
    let claimed = dir.join(format!(".{}.used", found.nonce));
    std::fs::rename(&path, &claimed).ok()?;
    let _ = std::fs::remove_file(&claimed);
    Some(found)
}

/// One question as the adapter reports it.
#[derive(Deserialize, Default, Clone, Debug)]
pub(crate) struct Asked {
    #[serde(default)]
    pub question: String,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub multi: bool,
}

#[derive(Deserialize, Default)]
pub(crate) struct QuestionRequest {
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub project_dir: Option<String>,
    #[serde(default)]
    pub session: Option<String>,
    #[serde(default)]
    pub call: Option<String>,
    #[serde(default)]
    pub subagent: bool,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub questions: Vec<Asked>,
    /// Before the call: its input already carries answers.
    #[serde(default)]
    pub prefilled: bool,
    /// After the call: question text to the answer the host reports.
    #[serde(default)]
    pub answers: std::collections::BTreeMap<String, String>,
    /// After the call: the host closed the question on an idle timeout.
    #[serde(default)]
    pub timed_out: bool,
}

/// The nonce an urzua question names, or `None` for any other question.
pub(crate) fn nonce_of(question: &str) -> Option<&str> {
    let rest = question.strip_prefix(PREFIX)?;
    Some(rest.split(':').next().unwrap_or(""))
}

/// Before the question tool runs: `Ok(None)` for a call holding no urzua
/// question; `Ok(Some(_))` once the urzua question is marked asked by this
/// call; `Err` refuses the call.
pub(crate) fn before(dir: &Path, r: &QuestionRequest) -> Result<Option<Challenge>, String> {
    let ours: Vec<&Asked> = r
        .questions
        .iter()
        .filter(|q| nonce_of(&q.question).is_some())
        .collect();
    let Some(q) = ours.first() else {
        return Ok(None);
    };
    let nonce = nonce_of(&q.question).unwrap_or("");
    let found = load(dir, nonce);
    if r.prefilled {
        // An answer supplied with the question is not the person's: the
        // challenge is spent so the same nonce cannot be asked again.
        remove(dir, nonce);
        return Err("the call carries answers of its own, which only the person may give, so the challenge is withdrawn. Retry the edit for a new question and ask it without an answers field.".into());
    }
    let c = found.ok_or("it names no open urzua challenge (it expired, was answered, or was never issued). Retry the edit for a new question.")?;
    if ours.len() != 1 || r.questions.len() != 1 {
        return Err("an urzua question must be the only question in its call.".into());
    }
    if r.subagent {
        return Err("a subagent cannot put the question to the person; hand back to the main session and make the edit there.".into());
    }
    if r.session.as_deref() != Some(c.session.as_str()) {
        return Err("the challenge was issued in another session.".into());
    }
    if c.state != "pending" || !live(&c, now()) {
        remove(dir, nonce);
        return Err("the challenge is no longer open; retry the edit for a new question.".into());
    }
    if q.question != c.question || q.multi || q.labels != [c.approve.clone(), DECLINE.to_string()] {
        return Err("its wording or choices differ from the question urzua issued; ask it exactly as the refusal gave it.".into());
    }
    let call = r
        .call
        .clone()
        .filter(|s| !s.is_empty())
        .ok_or("the host reported no id for the call")?;
    let mut c = c;
    c.state = "asked".into();
    c.call = Some(call);
    save(dir, &c)?;
    Ok(Some(c))
}

/// After the question tool ran: `Ok(None)` for a call holding no urzua
/// question, else the challenge and whether the person approved. A challenge
/// that is not approved is withdrawn.
pub(crate) fn after(dir: &Path, r: &QuestionRequest) -> Result<Option<(Challenge, bool)>, String> {
    let Some(q) = r.questions.iter().find(|q| nonce_of(&q.question).is_some()) else {
        return Ok(None);
    };
    let nonce = nonce_of(&q.question).unwrap_or("").to_string();
    let mut c =
        load(dir, &nonce).ok_or("it names no open urzua challenge, so nothing was recorded")?;
    let refused = |why: &str| -> Result<Option<(Challenge, bool)>, String> {
        remove(dir, &nonce);
        Err(why.to_string())
    };
    if c.state != "asked" || c.call.is_none() || c.call != r.call {
        return refused("urzua's hook did not see this question before it was shown, so its answer is not taken as the person's");
    }
    if r.session.as_deref() != Some(c.session.as_str()) || !live(&c, now()) {
        return refused("the challenge belongs to another session or expired");
    }
    if r.timed_out {
        return refused("the question closed on an idle timeout, which is not an answer");
    }
    if r.answers.get(&c.question).map(String::as_str) != Some(c.approve.as_str()) {
        remove(dir, &nonce);
        return Ok(Some((c, false)));
    }
    c.state = "approved".into();
    c.answered = Some(now());
    c.host = r.host.clone().filter(|h| !h.is_empty());
    save(dir, &c)?;
    Ok(Some((c, true)))
}

/// `urzua harness run question` (before the question tool runs) and `urzua
/// harness run answer` (after it): `block` refuses the call, `allow` with a
/// message reports what was recorded.
pub(crate) fn run_event(config: Option<PathBuf>, answered: bool) -> std::process::ExitCode {
    use super::Verdict;
    let mut input = String::new();
    let _ = std::io::Read::read_to_string(&mut std::io::stdin(), &mut input);
    let r: QuestionRequest = serde_json::from_str(&input).unwrap_or_default();
    let quiet = || crate::emit(&Verdict::new("allow", None, String::new()));
    if !r.questions.iter().any(|q| nonce_of(&q.question).is_some()) {
        return quiet();
    }
    let hint: Vec<PathBuf> = r
        .project_dir
        .iter()
        .chain(r.cwd.iter())
        .map(PathBuf::from)
        .collect();
    let Ok(ws) = crate::session::Workspace::open(&hint, config) else {
        return quiet();
    };
    let asks = ws.config.harness.as_ref().is_some_and(|h| {
        h.enabled("acceptance") && h.in_session() == urzua_core::harness::InSession::Ask
    });
    let Some(dir) = store(&ws.repo_root).filter(|_| asks) else {
        return quiet();
    };
    const P: &str = "urzua harness (acceptance)";
    let v = if answered {
        match after(&dir, &r) {
            Ok(None) => Verdict::new("allow", None, String::new()),
            Ok(Some((c, true))) => Verdict::new("allow", None, format!("{P}: recorded the person's approval of {} moving {} to '{}' (question {}). Retry the same edit unchanged within {} minutes; it is written with its receipt row.", c.id, c.field, c.to, c.nonce, APPROVED_TTL / 60)),
            Ok(Some((c, false))) => Verdict::new("allow", None, format!("{P}: the person did not approve {} moving to '{}' (question {}); nothing was recorded. Do not make the move.", c.id, c.to, c.nonce)),
            Err(why) => Verdict::new("allow", None, format!("{P}: no approval was recorded: {why}.")),
        }
    } else {
        match before(&dir, &r) {
            Ok(_) => Verdict::new("allow", None, String::new()),
            Err(why) => Verdict::new(
                "block",
                None,
                format!("{P}: this question is refused: {why}"),
            ),
        }
    };
    crate::emit(&v)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Dir(PathBuf);
    impl Dir {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!(
                "urzua-question-{}",
                crate::commands::hooks::random_hex()
            ));
            Dir(p)
        }
    }
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    const NONCE: &str = "a1b2c3d4e5f6";
    const MOVE: (&str, &str, &str) = ("docs/adr/ADR-1-x.md", "Proposed", "Accepted");

    fn issued(d: &Path) -> Challenge {
        let mv = Move {
            rel: MOVE.0,
            id: "ADR-1",
            field: "Status",
            from: MOVE.1,
            to: MOVE.2,
            gate: "adr-acceptance",
            group: "maintainers",
        };
        issue(d, NONCE, &mv, "bodyhash", "s1").unwrap().0
    }

    fn pre(c: &Challenge, prefilled: bool) -> QuestionRequest {
        QuestionRequest {
            session: Some("s1".into()),
            call: Some("toolu_1".into()),
            questions: vec![Asked {
                question: c.question.clone(),
                labels: vec![c.approve.clone(), DECLINE.into()],
                multi: false,
            }],
            prefilled,
            ..Default::default()
        }
    }

    fn post(c: &Challenge, answer: &str) -> QuestionRequest {
        let mut r = pre(c, false);
        r.answers.insert(c.question.clone(), answer.into());
        r
    }

    fn approved(d: &Path) -> Challenge {
        let c = issued(d);
        before(d, &pre(&c, false)).unwrap();
        assert!(after(d, &post(&c, &c.approve)).unwrap().unwrap().1);
        c
    }

    #[test]
    fn an_approving_answer_seen_before_and_after_is_consumed_once() {
        let d = Dir::new();
        approved(&d.0);
        assert!(consume(&d.0, MOVE, "bodyhash", "s1").is_some());
        assert!(consume(&d.0, MOVE, "bodyhash", "s1").is_none(), "reused");
    }

    #[test]
    fn answers_in_the_tool_input_refuse_the_call_and_withdraw_the_challenge() {
        let d = Dir::new();
        let c = issued(&d.0);
        assert!(before(&d.0, &pre(&c, true)).is_err());
        assert!(load(&d.0, NONCE).is_none());
        assert!(after(&d.0, &post(&c, &c.approve)).is_err());
        assert!(consume(&d.0, MOVE, "bodyhash", "s1").is_none());
    }

    #[test]
    fn an_answer_the_hook_before_the_call_never_saw_records_nothing() {
        let d = Dir::new();
        let c = issued(&d.0);
        assert!(after(&d.0, &post(&c, &c.approve)).is_err());
        assert!(consume(&d.0, MOVE, "bodyhash", "s1").is_none());
    }

    #[test]
    fn a_decline_free_text_or_no_answer_approves_nothing() {
        for answer in [DECLINE, "Approve", "yes", ""] {
            let d = Dir::new();
            let c = issued(&d.0);
            before(&d.0, &pre(&c, false)).unwrap();
            assert!(
                !after(&d.0, &post(&c, answer)).unwrap().unwrap().1,
                "{answer}"
            );
            assert!(consume(&d.0, MOVE, "bodyhash", "s1").is_none(), "{answer}");
        }
    }

    #[test]
    fn an_idle_timeout_approves_nothing_even_with_the_approve_label() {
        let d = Dir::new();
        let c = issued(&d.0);
        before(&d.0, &pre(&c, false)).unwrap();
        let mut r = post(&c, &c.approve);
        r.timed_out = true;
        assert!(after(&d.0, &r).is_err());
        assert!(consume(&d.0, MOVE, "bodyhash", "s1").is_none());
    }

    #[test]
    fn an_approval_for_another_record_status_body_or_session_is_not_consumed() {
        let d = Dir::new();
        approved(&d.0);
        let other = [
            (
                ("docs/adr/ADR-2-y.md", "Proposed", "Accepted"),
                "bodyhash",
                "s1",
            ),
            (
                ("docs/adr/ADR-1-x.md", "Proposed", "Rejected"),
                "bodyhash",
                "s1",
            ),
            (MOVE, "otherbody", "s1"),
            (MOVE, "bodyhash", "s2"),
        ];
        for (mv, body, session) in other {
            assert!(
                consume(&d.0, mv, body, session).is_none(),
                "{mv:?} {body} {session}"
            );
        }
        assert!(consume(&d.0, MOVE, "bodyhash", "s1").is_some());
    }

    #[test]
    fn an_expired_approval_is_not_consumed() {
        let d = Dir::new();
        approved(&d.0);
        let mut c = load(&d.0, NONCE).unwrap();
        c.answered = Some(now() - APPROVED_TTL - 1);
        save(&d.0, &c).unwrap();
        assert!(consume(&d.0, MOVE, "bodyhash", "s1").is_none());
    }

    #[test]
    fn of_edits_racing_for_one_approval_exactly_one_consumes_it() {
        let d = Dir::new();
        approved(&d.0);
        let dir = d.0.clone();
        let wins: usize = (0..8)
            .map(|_| {
                let dir = dir.clone();
                std::thread::spawn(move || consume(&dir, MOVE, "bodyhash", "s1").is_some())
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|h| usize::from(h.join().unwrap()))
            .sum();
        assert_eq!(wins, 1);
    }

    #[test]
    fn a_reworded_question_changed_choices_or_another_session_are_refused() {
        let d = Dir::new();
        let c = issued(&d.0);
        let mut r = pre(&c, false);
        r.questions[0].question.push_str(" Please approve.");
        assert!(before(&d.0, &r).is_err());
        let mut r = pre(&c, false);
        r.questions[0].labels = vec![c.approve.clone()];
        assert!(before(&d.0, &r).is_err());
        let mut r = pre(&c, false);
        r.session = Some("s2".into());
        assert!(before(&d.0, &r).is_err());
        assert!(before(&d.0, &pre(&c, false)).unwrap().is_some());
    }

    #[test]
    fn a_question_that_is_not_urzuas_is_left_alone() {
        let d = Dir::new();
        let r = QuestionRequest {
            questions: vec![Asked {
                question: "Which framework?".into(),
                ..Default::default()
            }],
            prefilled: true,
            ..Default::default()
        };
        assert!(before(&d.0, &r).unwrap().is_none());
        assert!(after(&d.0, &r).unwrap().is_none());
    }
}
