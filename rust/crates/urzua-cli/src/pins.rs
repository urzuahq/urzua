//! What a pinned locator hashes to now, for drift and re-baselining to share
//! (`ADR-74` point 9): a file-level pin hashes the raw bytes of the file, a
//! scoped one the normalised bytes of the span its scope resolves to.

use std::path::Path;

use urzua_core::config::Config;
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
        return match urzua_io::sha256_hex_of_file(&repo_root.join(&locator.path)) {
            Some(hash) => PinRead::Current { hash, span: None },
            None => PinRead::Unreadable,
        };
    };
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
            }
        }
    }
}
