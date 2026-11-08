//! What a pinned locator hashes to now, for drift and re-baselining to share
//! (`ADR-74` point 9): a file-level pin hashes the raw bytes of the file, a
//! scoped one the normalised bytes of the span its scope resolves to.

use std::path::Path;

use urzua_core::config::{Config, LocatorKind};
use urzua_core::fix::{PinRead, PinSpan};
use urzua_core::locator::{self, FileText, Locator, ScopeError};

use crate::file_texts::FileTexts;

// Implements: ADR-74 (the pinned bytes of a scope)
pub(crate) fn read_pin(
    repo_root: &Path,
    config: &Config,
    texts: &FileTexts,
    locator: &Locator,
) -> PinRead {
    let Some(scope) = &locator.scope else {
        if let Some(read) = read_files_pin(repo_root, config, texts, &locator.path) {
            return read;
        }
        if !is_tracked_file(texts, &locator.path) {
            return PinRead::Unreadable;
        }
        return match urzua_io::sha256_hex_of_file(&repo_root.join(&locator.path)) {
            Some(hash) => PinRead::Current {
                hash,
                span: None,
                files: Vec::new(),
            },
            None => PinRead::Unreadable,
        };
    };
    if !is_tracked_file(texts, &locator.path) {
        return PinRead::Unreadable;
    }
    let text = match &*texts.get(&locator.path) {
        FileText::Missing => return PinRead::Unreadable,
        FileText::NotUtf8 => return PinRead::Failed(ScopeError::NotUtf8.to_string()),
        FileText::Text(text) => text.clone(),
    };
    let resolved = locator::select_resolver(&config.locator_resolvers, &locator.path)
        .and_then(|resolver| locator::resolve_scope(&text, resolver, scope));
    match resolved {
        Err(e) => PinRead::Failed(e.to_string()),
        Ok(span) => {
            let pinned = locator::span_text(&text, span);
            PinRead::Current {
                hash: urzua_io::sha256_hex_of_str(&pinned),
                span: Some(PinSpan {
                    start: span.start,
                    end: span.end,
                    text: pinned,
                }),
                files: Vec::new(),
            }
        }
    }
}

/// Whether `path` is a file inside the repository that git tracks: the same
/// presence `embodiment.locator-exists` requires, so a pin is never hashed from
/// a path outside the repository or one only the working tree has.
fn is_tracked_file(texts: &FileTexts, path: &str) -> bool {
    texts.confined(path).is_ok()
        && texts
            .locator_files(path)
            .is_ok_and(|located| located.kind == Some(LocatorKind::File))
}

// BUG-176: a pin on a directory or glob hashes the sorted paths and content hashes of its files
/// What a directory or glob locator that the repository lets be pinned hashes
/// to now: the hash of the sorted list of (path, file content hash) for every
/// tracked file it matches, with those files. `None` when `path` is not such a
/// locator (a file, a kind the config does not declare pinnable, or a path that
/// names nothing), so the caller reads it as it always has.
pub(crate) fn read_files_pin(
    repo_root: &Path,
    config: &Config,
    texts: &FileTexts,
    path: &str,
) -> Option<PinRead> {
    let (_, covered) = pinned_kind(config, texts, path)?;
    let mut pairs = Vec::new();
    for file in &covered {
        let Some(hash) = urzua_io::sha256_hex_of_file(&repo_root.join(file)) else {
            return Some(PinRead::Unreadable);
        };
        pairs.push((file.clone(), hash));
    }
    if pairs.is_empty() {
        return Some(PinRead::Unreadable);
    }
    Some(PinRead::Current {
        hash: urzua_io::sha256_hex_of_str(&locator::files_pin_text(&pairs)),
        span: None,
        files: covered,
    })
}

/// The kind and covered files of `path` when it is a directory or glob that
/// `locator_kinds` accepts and `pinnable` lets carry a pin.
pub(crate) fn pinned_kind(
    config: &Config,
    texts: &FileTexts,
    path: &str,
) -> Option<(LocatorKind, Vec<String>)> {
    let opts: urzua_core::rules::LocatorExistsOpts =
        config.rule_opts(urzua_core::rules::RULE_EMBODIMENT_LOCATOR_EXISTS);
    let located = texts.locator_files(path).ok()?;
    let kind = located.kind?;
    (kind != LocatorKind::File
        && opts.locator_kinds.contains(&kind)
        && opts.pinnable.contains(&kind))
    .then_some((kind, located.files))
}
