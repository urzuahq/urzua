//! Edit-in-place for the two commands that write into an existing record
//! (`urzua fix --apply`, `urzua migrate ids --apply`).
//!
//! Both used to rebuild the file as `lines().join("\n") + "\n"`, which
//! rewrote a CRLF file to LF, forced a trailing newline onto a file that had
//! none, and so turned a one-line change into a whole-file diff. A
//! `LineDoc` keeps every line's own terminator, so replacing or inserting
//! one line leaves every other byte of the file as it was.

/// A file as lines, each carrying its own original terminator (`"\n"`,
/// `"\r\n"`, or `""` for a final line with none).
#[derive(Debug, Clone)]
pub struct LineDoc {
    lines: Vec<(String, String)>,
}

impl LineDoc {
    /// Splits on `\n`, treating a `\r` just before it as part of the
    /// terminator -- the same segmentation `str::lines` uses, so a 1-indexed
    /// line number computed from `lines()` addresses the same line here.
    pub fn parse(content: &str) -> Self {
        let lines = content
            .split_inclusive('\n')
            .map(|seg| {
                let body = seg.strip_suffix('\n').unwrap_or(seg);
                let body = if body.len() != seg.len() {
                    body.strip_suffix('\r').unwrap_or(body)
                } else {
                    body
                };
                (body.to_string(), seg[body.len()..].to_string())
            })
            .collect();
        LineDoc { lines }
    }

    /// The text of the 1-indexed line `n`, without its terminator.
    pub fn line(&self, n: usize) -> Option<&str> {
        self.lines.get(n.checked_sub(1)?).map(|(b, _)| b.as_str())
    }

    pub fn find_line(&self, pred: impl Fn(&str) -> bool) -> Option<usize> {
        self.lines.iter().position(|(b, _)| pred(b)).map(|i| i + 1)
    }

    /// The terminator this file uses for new lines: the first one present,
    /// `"\n"` for a file with none.
    fn dominant_eol(&self) -> String {
        self.lines
            .iter()
            .map(|(_, e)| e.as_str())
            .find(|e| !e.is_empty())
            .unwrap_or("\n")
            .to_string()
    }

    /// Replaces the body of the 1-indexed line `n`, keeping its terminator.
    pub fn replace_line(&mut self, n: usize, text: String) {
        self.lines[n - 1].0 = text;
    }

    /// Inserts `text` as a new line immediately after the 1-indexed line
    /// `after` (`0` inserts at the top). A file without a trailing newline
    /// stays without one: inserting after its last line gives that line a
    /// terminator and leaves the new last line bare.
    pub fn insert_after(&mut self, after: usize, text: String) {
        let eol = self.dominant_eol();
        if after == self.lines.len() && after > 0 && self.lines[after - 1].1.is_empty() {
            self.lines[after - 1].1 = eol;
            self.lines.push((text, String::new()));
        } else {
            self.lines.insert(after, (text, eol));
        }
    }
}

impl std::fmt::Display for LineDoc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (body, eol) in &self.lines {
            f.write_str(body)?;
            f.write_str(eol)?;
        }
        Ok(())
    }
}

/// Everything before the first `|` of a table row: the blockquote marker
/// and spacing (`"> "`, `">"`, `""`) a sibling row must repeat to stay in
/// the same table. `None` when the line holds no `|`.
pub fn table_row_prefix(row: &str) -> Option<&str> {
    row.find('|').map(|i| &row[..i])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_every_terminator_shape() {
        for s in [
            "",
            "a",
            "a\n",
            "a\r\nb\r\n",
            "a\r\nb",
            "a\nb\r\nc\n",
            "\n\n",
        ] {
            assert_eq!(LineDoc::parse(s).to_string(), s, "{s:?}");
        }
    }

    #[test]
    fn insert_keeps_crlf_and_the_absent_trailing_newline() {
        let mut d = LineDoc::parse("a\r\nb\r\nc");
        d.insert_after(1, "x".into());
        d.insert_after(4, "z".into());
        assert_eq!(d.to_string(), "a\r\nx\r\nb\r\nc\r\nz");
    }

    #[test]
    fn replace_keeps_the_terminator() {
        let mut d = LineDoc::parse("a\r\nb\r\n");
        d.replace_line(2, "B".into());
        assert_eq!(d.to_string(), "a\r\nB\r\n");
    }

    #[test]
    fn row_prefix() {
        assert_eq!(table_row_prefix("> | a |"), Some("> "));
        assert_eq!(table_row_prefix(">| a |"), Some(">"));
        assert_eq!(table_row_prefix("| a |"), Some(""));
        assert_eq!(table_row_prefix("none"), None);
    }
}
