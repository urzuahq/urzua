// Verifies: ADR-74
use super::*;

const SHA: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn ok(raw: &str) -> Locator {
    parse_locator(raw).unwrap_or_else(|e| panic!("{raw}: {e}"))
}

fn sym(parts: &[&str]) -> Scope {
    Scope::Symbol(
        parts
            .iter()
            .map(
                |p| match p.strip_prefix('<').and_then(|p| p.strip_suffix('>')) {
                    Some(t) => Segment::Trait(t.to_string()),
                    None => Segment::Name(p.to_string()),
                },
            )
            .collect(),
    )
}

// ---- grammar and parse order ------------------------------------------------

#[test]
fn a_file_level_pin_keeps_its_adr_67_meaning() {
    let l = ok(&format!("src/a.rs@sha256:{SHA}"));
    assert_eq!(l.path, "src/a.rs");
    assert_eq!(l.scope, None);
    assert_eq!(l.anchor, Some(Anchor::Pin(SHA.to_string())));
}

#[test]
fn an_unpinned_unscoped_locator_is_just_a_path() {
    let l = ok("./src/../src/a.rs");
    assert_eq!(l.path, "src/a.rs");
    assert_eq!((l.scope, l.anchor), (None, None));
}

#[test]
fn a_symbol_scope_sits_between_path_and_pin() {
    let l = ok(&format!("src/config.rs#RuleSetting@sha256:{SHA}"));
    assert_eq!(l.path, "src/config.rs");
    assert_eq!(l.scope, Some(sym(&["RuleSetting"])));
    assert_eq!(l.anchor, Some(Anchor::Pin(SHA.to_string())));
    assert_eq!(l.spelling, "src/config.rs#RuleSetting");
}

#[test]
fn qualified_symbols_parse_members_and_trait_impls() {
    assert_eq!(
        ok("a.rs#RecordTypeConfig::excludes").scope,
        Some(sym(&["RecordTypeConfig", "excludes"]))
    );
    assert_eq!(
        ok("a.rs#RuleSetting::<Deserialize>::deserialize").scope,
        Some(sym(&["RuleSetting", "<Deserialize>", "deserialize"]))
    );
    assert_eq!(
        ok("a.rs#RuleSetting::<Deserialize>").scope,
        Some(sym(&["RuleSetting", "<Deserialize>"]))
    );
    assert_eq!(ok("a.rs#a.b-c_d").scope, Some(sym(&["a.b-c_d"])));
}

#[test]
fn a_range_is_l_n_or_l_n_dash_l_m() {
    assert_eq!(
        ok(".github/workflows/ci.yml#L10-L40").scope,
        Some(Scope::Range { start: 10, end: 40 })
    );
    assert_eq!(
        ok("a.rs#L10").scope,
        Some(Scope::Range { start: 10, end: 10 })
    );
    assert_eq!(
        ok("a.rs#L7-L7").scope,
        Some(Scope::Range { start: 7, end: 7 })
    );
}

#[test]
fn l_followed_by_a_digit_is_a_range_or_malformed_never_a_symbol() {
    for bad in [
        "a.rs#L10x",
        "a.rs#L0",
        "a.rs#L010",
        "a.rs#L40-L10",
        "a.rs#L10-40",
        "a.rs#L1-L",
        "a.rs#L1-L0",
        "a.rs#L99999999999999999999",
    ] {
        assert!(
            matches!(parse_locator(bad), Err(LocatorError::MalformedRange(_))),
            "{bad}: {:?}",
            parse_locator(bad)
        );
    }
    // `L` not followed by a digit is an ordinary symbol.
    assert_eq!(ok("a.rs#Lexer").scope, Some(sym(&["Lexer"])));
    assert_eq!(ok("a.rs#L_1").scope, Some(sym(&["L_1"])));
}

#[test]
fn a_commit_anchor_is_seven_to_sixty_four_lowercase_hex() {
    assert_eq!(
        ok("a.rs@abcdef1").anchor,
        Some(Anchor::Commit("abcdef1".into()))
    );
    let long = "a".repeat(64);
    assert_eq!(
        ok(&format!("a.rs#L3@{long}")).anchor,
        Some(Anchor::Commit(long))
    );
    assert_eq!(ok("a.rs#foo@abcdef1").scope, Some(sym(&["foo"])));
    // Six digits, uppercase, or 65 hex are not an anchor: they stay in the path.
    for not_anchor in [
        "a.rs@abcdef",
        "a.rs@ABCDEF1",
        &format!("a.rs@{}", "a".repeat(65)),
    ] {
        let l = ok(not_anchor);
        assert_eq!(l.anchor, None, "{not_anchor}");
        assert_eq!(l.path, not_anchor);
    }
}

#[test]
fn both_anchors_on_one_locator_is_malformed() {
    for bad in [
        format!("a.rs@abcdef1@sha256:{SHA}"),
        format!("a.rs@sha256:{SHA}@abcdef1"),
    ] {
        assert!(
            matches!(parse_locator(&bad), Err(LocatorError::BothAnchors)),
            "{bad}: {:?}",
            parse_locator(&bad)
        );
    }
}

#[test]
fn a_bad_pin_is_malformed_not_a_path() {
    for bad in [
        "s.md@sha256:ABC",
        "s.md@sha256:",
        &format!("s.md@sha256:{}", "A".repeat(64)),
        &format!("s.md@sha256:{}", "a".repeat(63)),
    ] {
        assert!(
            matches!(parse_locator(bad), Err(LocatorError::MalformedPin(_))),
            "{bad}: {:?}",
            parse_locator(bad)
        );
    }
}

#[test]
fn the_scope_is_the_text_after_the_last_hash_only_when_it_matches() {
    // Not a scope: the whole text is the path (a file named with a `#`).
    for path in [
        "notes#1.md",
        "a#b c.md",
        "x#",
        "a.rs#<Trait>::f",
        "a.rs#A::<std::fmt::Display>::f",
        "a.rs#1abc",
        "a.rs#.hidden",
        "a.rs#A::",
        "a.rs#A::<>",
    ] {
        let l = ok(path);
        assert_eq!((l.scope, l.path.as_str()), (None, path), "{path}");
    }
    // The last `#` wins.
    let l = ok("dir#1/a.rs#Foo");
    assert_eq!(
        (l.path.as_str(), l.scope),
        ("dir#1/a.rs", Some(sym(&["Foo"])))
    );
}

#[test]
fn a_glob_cannot_carry_a_scope() {
    assert!(matches!(
        parse_locator("src/**#Foo"),
        Err(LocatorError::GlobWithScope(_))
    ));
    assert!(matches!(
        parse_locator("src/*.rs#L1-L2"),
        Err(LocatorError::GlobWithScope(_))
    ));
}

#[test]
fn the_re_pin_spelling_is_the_text_before_the_anchor_as_written() {
    let l = ok(&format!("./src/a.rs#L3-L9@sha256:{SHA}"));
    assert_eq!(l.spelling, "./src/a.rs#L3-L9");
    assert_eq!(l.path, "src/a.rs");
}

#[test]
fn a_scope_displays_as_written() {
    assert_eq!(sym(&["A", "<T>", "f"]).to_string(), "A::<T>::f");
    assert_eq!(Scope::Range { start: 3, end: 9 }.to_string(), "L3-L9");
    assert_eq!(Scope::Range { start: 3, end: 3 }.to_string(), "L3");
}

#[test]
fn realized_by_locators_keeps_category_order_and_skips_text_before_the_first_category() {
    // Text before the first category prefix belongs to no entry. After one, a
    // comma followed by anything but a category stays in the path (`BUG-129`).
    let v = format!("doc:x.md, spec:s.md, code:a.rs#Foo@sha256:{SHA}, test:t.rs#L1-L2");
    let got = realized_by_locators(&v);
    let cats: Vec<&str> = got.iter().map(|e| e.category.as_str()).collect();
    assert_eq!(cats, ["spec", "code", "test"]);
    assert_eq!(got[1].parsed.as_ref().unwrap().scope, Some(sym(&["Foo"])));
    assert_eq!(got[2].raw, "t.rs#L1-L2");
}

// ---- resolvers --------------------------------------------------------------

fn rust() -> LocatorResolver {
    yaml_serde::from_str(
        r#"
name: rust
glob: "**/*.rs"
start: '^\s*(pub(\([^)]*\))?\s+)?((const|async|unsafe)\s+)*(fn|struct|enum|trait|union)\s+{symbol}\b'
end: brace_balanced
leading: '^\s*(#\[|///|//!)'
container:
  start: '^\s*impl(<.*?>)?\s+{symbol}(<[^{]*>)?\s*(where\b.*|\{.*)?$'
  end: brace_balanced
  spans: all
member:
  start: '^\s*(pub(\([^)]*\))?\s+)?((const|async|unsafe)\s+)*fn\s+{symbol}\b'
  end: brace_balanced
trait_container:
  start: '^\s*impl(<.*?>)?\s+([A-Za-z_][A-Za-z0-9_]*::)*{trait}(<[^{]*>)?\s+for\s+{symbol}\b'
  end: brace_balanced
  spans: all
"#,
    )
    .unwrap()
}

fn span(text: &str, scope: &str) -> Result<(usize, usize), ScopeError> {
    let s = ok(&format!("f.rs#{scope}")).scope.unwrap();
    resolve_scope(text, Some(&rust()), &s).map(|s| (s.start, s.end))
}

const RUST_SRC: &str = "\
use std::fmt;

/// A doc line.
#[derive(Debug)]
pub struct Alpha {
    a: u32,
}

impl Alpha {
    pub fn new() -> Self {
        Alpha { a: 0 }
    }

    fn helper(&self) {
        fn nested() {}
        nested();
    }
}

fn free() {
    let x = 1;
}

impl<T> Alpha {
    #[inline]
    pub fn second(&self) -> u32 {
        self.a
    }
}

impl fmt::Display for Alpha {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, \"alpha\")
    }
}

impl From<u8> for Alpha {
    fn from(a: u8) -> Self { Alpha { a: a as u32 } }
}

impl From<u16> for Alpha {
    fn from(a: u16) -> Self { Alpha { a: a as u32 } }
}
";

#[test]
fn a_plain_symbol_spans_its_leading_lines_and_balanced_body() {
    // doc line and attribute are leading; the span ends at the closing brace.
    assert_eq!(span(RUST_SRC, "Alpha"), Ok((3, 7)));
    assert_eq!(span(RUST_SRC, "free"), Ok((20, 22)));
}

#[test]
fn a_missing_symbol_is_reported() {
    assert!(matches!(
        span(RUST_SRC, "Nope"),
        Err(ScopeError::NotFound(_))
    ));
}

#[test]
fn an_ambiguous_symbol_is_reported_with_its_lines_never_first_match() {
    let src = "fn dup() {}\n\nfn dup() {\n}\n";
    assert_eq!(
        span(src, "dup"),
        Err(ScopeError::Ambiguous { lines: vec![1, 3] })
    );
}

#[test]
fn a_member_is_found_across_every_inherent_impl_block() {
    assert_eq!(span(RUST_SRC, "Alpha::new"), Ok((10, 12)));
    // In the second `impl<T> Alpha`, with its attribute as a leading line.
    assert_eq!(span(RUST_SRC, "Alpha::second"), Ok((25, 28)));
}

#[test]
fn a_member_repeated_across_spans_is_ambiguous() {
    let src = "impl A {\n    fn f(&self) {}\n}\nimpl A {\n    fn f(&self) {}\n}\n";
    assert_eq!(
        span(src, "A::f"),
        Err(ScopeError::Ambiguous { lines: vec![2, 5] })
    );
}

#[test]
fn a_member_matches_at_direct_child_depth_only() {
    // `nested` is a fn inside `helper`: not a candidate of `Alpha`.
    assert!(matches!(
        span(RUST_SRC, "Alpha::nested"),
        Err(ScopeError::NotFound(_))
    ));
    assert_eq!(span(RUST_SRC, "Alpha::helper"), Ok((14, 17)));
}

#[test]
fn the_container_must_exist_and_a_trait_impl_is_not_an_inherent_container() {
    assert!(matches!(
        span(RUST_SRC, "Missing::new"),
        Err(ScopeError::NotFound(_))
    ));
    // `fmt` lives only in `impl Display for Alpha`.
    assert!(matches!(
        span(RUST_SRC, "Alpha::fmt"),
        Err(ScopeError::NotFound(_))
    ));
}

#[test]
fn a_trait_impl_member_and_the_whole_impl_are_addressed_with_angle_brackets() {
    // A path before the trait name is allowed (`fmt::Display`).
    assert_eq!(span(RUST_SRC, "Alpha::<Display>::fmt"), Ok((32, 34)));
    assert_eq!(span(RUST_SRC, "Alpha::<Display>"), Ok((31, 35)));
}

#[test]
fn several_impls_of_one_generic_trait_make_a_member_ambiguous() {
    assert_eq!(
        span(RUST_SRC, "Alpha::<From>::from"),
        Err(ScopeError::Ambiguous {
            lines: vec![38, 42]
        })
    );
    assert_eq!(
        span(RUST_SRC, "Alpha::<From>"),
        Err(ScopeError::Ambiguous {
            lines: vec![37, 41]
        })
    );
}

#[test]
fn a_member_of_a_member_is_unsupported() {
    assert!(matches!(
        span(RUST_SRC, "Alpha::helper::nested"),
        Err(ScopeError::Unsupported(_))
    ));
}

#[test]
fn a_resolver_without_container_or_trait_container_reports_the_forms_unsupported() {
    let mut r = rust();
    r.container = None;
    r.trait_container = None;
    let q = ok("f.rs#Alpha::new").scope.unwrap();
    assert!(matches!(
        resolve_scope(RUST_SRC, Some(&r), &q),
        Err(ScopeError::Unsupported(_))
    ));
    let t = ok("f.rs#Alpha::<Display>::fmt").scope.unwrap();
    assert!(matches!(
        resolve_scope(RUST_SRC, Some(&r), &t),
        Err(ScopeError::Unsupported(_))
    ));
}

#[test]
fn an_unbalanced_item_has_no_end() {
    assert_eq!(
        span("fn open() {\n    if x {\n}\n", "open"),
        Err(ScopeError::EndNotFound)
    );
}

#[test]
fn a_brace_in_a_string_ends_the_span_early_and_that_is_not_detected() {
    // Documented limit of `brace_balanced`: blind to braces in strings. The
    // span stops at line 3 and line 4 is unpinned; `explain` shows the reviewer.
    let src = "fn s() {\n    let a = \"}\";\n}\n    more();\n}\n";
    assert_eq!(span(src, "s"), Ok((1, 2)));
}

#[test]
fn no_symbol_without_a_resolver() {
    let s = ok("f.txt#Foo").scope.unwrap();
    assert!(matches!(
        resolve_scope("Foo", None, &s),
        Err(ScopeError::NoResolver)
    ));
}

#[test]
fn a_range_needs_no_resolver_and_must_lie_inside_the_file() {
    let r = ok("f.txt#L2-L3").scope.unwrap();
    let s = resolve_scope("a\nb\nc\n", None, &r).unwrap();
    assert_eq!((s.start, s.end), (2, 3));
    // The trailing newline is not a line: the file has three lines.
    let past = ok("f.txt#L2-L4").scope.unwrap();
    assert_eq!(
        resolve_scope("a\nb\nc\n", None, &past),
        Err(ScopeError::RangeBeyondEnd { lines: 3 })
    );
    let past = ok("f.txt#L1").scope.unwrap();
    assert_eq!(
        resolve_scope("", None, &past),
        Err(ScopeError::RangeBeyondEnd { lines: 0 })
    );
}

fn yaml_resolver(end: &str) -> LocatorResolver {
    yaml_serde::from_str(&format!(
        "name: y\nglob: '**/*.yaml'\nstart: '^\\s*{{symbol}}:'\nend: {end}\n"
    ))
    .unwrap()
}

fn resolve_with(
    r: &LocatorResolver,
    text: &str,
    scope: &str,
) -> Result<(usize, usize), ScopeError> {
    let s = ok(&format!("f.yaml#{scope}")).scope.unwrap();
    resolve_scope(text, Some(r), &s).map(|s| (s.start, s.end))
}

#[test]
fn indent_ends_before_the_next_line_indented_no_deeper() {
    let t = "a:\n  x: 1\n\n  y:\n    z: 2\n\nb:\n  w: 3\n";
    let r = yaml_resolver("indent");
    // Trailing blank lines are not part of the span.
    assert_eq!(resolve_with(&r, t, "a"), Ok((1, 5)));
    assert_eq!(resolve_with(&r, t, "y"), Ok((4, 5)));
    assert_eq!(resolve_with(&r, t, "b"), Ok((7, 8)));
    assert_eq!(resolve_with(&r, t, "x"), Ok((2, 2)));
}

#[test]
fn heading_ends_before_the_next_heading_of_equal_or_higher_level() {
    let t = "# T\n\n## One\ntext\n### Sub\nmore\n\n## Two\nx\n";
    let mut r = yaml_resolver("heading");
    r.start = "^#+\\s+{symbol}$".to_string();
    assert_eq!(resolve_with(&r, t, "One"), Ok((3, 6)));
    assert_eq!(resolve_with(&r, t, "Sub"), Ok((5, 6)));
    assert_eq!(resolve_with(&r, t, "T"), Ok((1, 9)));
    assert_eq!(resolve_with(&r, t, "Two"), Ok((8, 9)));
    // A start line that is not a heading has no heading end.
    let mut r2 = r.clone();
    r2.start = "^{symbol}$".to_string();
    assert_eq!(resolve_with(&r2, t, "text"), Err(ScopeError::EndNotFound));
}

#[test]
fn blank_line_ends_before_the_next_blank_line_or_the_end_of_file() {
    let t = "k1:\n  v\n\nk2:\n  w\n";
    let r = yaml_resolver("blank_line");
    assert_eq!(resolve_with(&r, t, "k1"), Ok((1, 2)));
    assert_eq!(resolve_with(&r, t, "k2"), Ok((4, 5)));
}

#[test]
fn semicolon_ends_at_the_first_line_ending_in_a_semicolon_at_its_own_depth() {
    let src =
        "impl A {\n    const X: Foo = Foo {\n        a: 1;\n    };\n    const Y: u8 = 3;\n}\n";
    let mut r = rust();
    r.member.as_mut().unwrap().start = "^\\s*const\\s+{symbol}\\b".to_string();
    r.member.as_mut().unwrap().end = EndKind::Semicolon;
    let one = |sym: &str| {
        let s = ok(&format!("f.rs#A::{sym}")).scope.unwrap();
        resolve_scope(src, Some(&r), &s).map(|s| (s.start, s.end))
    };
    // The `;` inside the braces at line 3 is deeper than the item.
    assert_eq!(one("X"), Ok((2, 4)));
    assert_eq!(one("Y"), Ok((5, 5)));
    let unterminated = "impl A {\n    const Z: u8 = 3\n}\n";
    let s = ok("f.rs#A::Z").scope.unwrap();
    assert_eq!(
        resolve_scope(unterminated, Some(&r), &s),
        Err(ScopeError::EndNotFound)
    );
}

#[test]
fn an_indent_container_matches_members_at_its_child_indent() {
    let mut r = yaml_resolver("indent");
    r.container = Some(ContainerDecl {
        start: "^{symbol}:".to_string(),
        end: EndKind::Indent,
        spans: Spans::One,
    });
    r.member = Some(MemberDecl {
        start: "^\\s+{symbol}:".to_string(),
        end: EndKind::Indent,
    });
    let t = "a:\n  x:\n    deep: 1\n  y: 2\nb:\n  x: 3\n";
    assert_eq!(resolve_with(&r, t, "a::x"), Ok((2, 3)));
    assert_eq!(resolve_with(&r, t, "a::y"), Ok((4, 4)));
    // `deep` is a grandchild of `a`.
    assert!(matches!(
        resolve_with(&r, t, "a::deep"),
        Err(ScopeError::NotFound(_))
    ));
}

#[test]
fn a_container_without_spans_all_must_match_exactly_one_span() {
    let mut r = rust();
    r.container.as_mut().unwrap().spans = Spans::One;
    let s = ok("f.rs#Alpha::new").scope.unwrap();
    assert!(matches!(
        resolve_scope(RUST_SRC, Some(&r), &s),
        Err(ScopeError::Ambiguous { .. })
    ));
}

#[test]
fn the_symbol_is_regex_escaped_before_substitution() {
    let src = "fn a_b() {}\nfn axb() {}\n";
    let mut r = rust();
    r.start = "^fn\\s+{symbol}\\b".to_string();
    let s = ok("f.rs#a.b").scope.unwrap();
    // `.` is a literal dot, not "any character".
    assert!(matches!(
        resolve_scope(src, Some(&r), &s),
        Err(ScopeError::NotFound(_))
    ));
}

// ---- hashed bytes ------------------------------------------------------------

#[test]
fn the_pinned_bytes_drop_one_cr_per_line_join_with_lf_and_end_without_a_newline() {
    let text = "a\r\nb\r\r\nc\n";
    let s = Span { start: 1, end: 3 };
    // One trailing CR per line only: `b\r\r` keeps one.
    assert_eq!(span_text(text, s), "a\nb\r\nc");
}

#[test]
fn a_bom_on_line_one_is_dropped_and_lines_still_count_from_one() {
    let text = "\u{feff}fn a() {\n}\n";
    assert_eq!(span_text(text, Span { start: 1, end: 2 }), "fn a() {\n}");
    let s = ok("f.rs#a").scope.unwrap();
    assert_eq!(
        resolve_scope(text, Some(&rust()), &s).map(|s| (s.start, s.end)),
        Ok((1, 2))
    );
}

#[test]
fn crlf_and_lf_files_give_the_same_pinned_bytes() {
    let lf = "fn a() {\n    1\n}\n";
    let crlf = "fn a() {\r\n    1\r\n}\r\n";
    let s = Span { start: 1, end: 3 };
    assert_eq!(span_text(lf, s), span_text(crlf, s));
}

// ---- resolver config ---------------------------------------------------------

#[test]
fn a_well_formed_resolver_set_validates() {
    assert_eq!(
        validate_resolvers(&[rust(), yaml_resolver("indent")]),
        Ok(())
    );
}

#[test]
fn resolver_load_errors_are_never_first_wins() {
    let bad = |f: &dyn Fn(&mut LocatorResolver)| {
        let mut r = rust();
        f(&mut r);
        validate_resolvers(&[r]).unwrap_err()
    };
    assert!(bad(&|r| r.start = "^fn\\s+x".into()).contains("{symbol}"));
    assert!(bad(&|r| r.start = "^fn\\s+({symbol}".into()).contains("pattern"));
    assert!(bad(&|r| r.glob = "src/[".into()).contains("glob"));
    assert!(bad(&|r| r.leading = Some("(".into())).contains("leading"));
    assert!(bad(&|r| r.container.as_mut().unwrap().start = "^impl".into()).contains("{symbol}"));
    assert!(bad(&|r| r.container.as_mut().unwrap().end = EndKind::Semicolon).contains("container"));
    assert!(
        bad(&|r| r.trait_container.as_mut().unwrap().start = "^impl {symbol}".into())
            .contains("{trait}")
    );
    assert!(bad(&|r| {
        r.container = None;
        r.trait_container = None;
    })
    .contains("member"));
    // Two resolvers declared for the same glob.
    let err = validate_resolvers(&[rust(), rust()]).unwrap_err();
    assert!(err.contains("rust"), "{err}");
}

#[test]
fn the_resolver_for_a_path_is_the_one_whose_glob_covers_it() {
    let rs = [rust(), yaml_resolver("indent")];
    assert_eq!(
        select_resolver(&rs, "a/b.rs").unwrap().unwrap().name,
        "rust"
    );
    assert_eq!(select_resolver(&rs, "c.yaml").unwrap().unwrap().name, "y");
    assert!(select_resolver(&rs, "c.md").unwrap().is_none());
    // Two covering resolvers: an error, never the first.
    let mut other = yaml_resolver("indent");
    other.name = "other".into();
    other.glob = "**/c.yaml".into();
    let rs = [yaml_resolver("indent"), other];
    assert!(matches!(
        select_resolver(&rs, "c.yaml"),
        Err(ScopeError::MultipleResolvers(_))
    ));
}

#[test]
fn every_failure_has_a_reason_that_names_the_cause() {
    let reasons = [
        ScopeError::NoResolver.to_string(),
        ScopeError::NotFound("x".into()).to_string(),
        ScopeError::Ambiguous { lines: vec![1, 2] }.to_string(),
        ScopeError::EndNotFound.to_string(),
        ScopeError::RangeBeyondEnd { lines: 3 }.to_string(),
    ];
    assert!(reasons[0].contains("unsupported"));
    assert!(reasons[2].contains("1, 2") && reasons[2].contains("ambiguous"));
    assert!(reasons[3].contains("end"));
    assert!(reasons[4].contains("3"));
}

/// The rows of SPEC-3's normative vector table that use the `rust` resolver.
#[test]
fn spec_3_vectors_for_the_rust_resolver() {
    type Row = (
        &'static str,
        &'static str,
        Result<(usize, usize), ScopeError>,
    );
    let rows: [Row; 9] = [
        ("/// d\n#[derive(X)]\nstruct A {\n  a: u8,\n}\n", "A", Ok((1, 5))),
        ("fn f() {\n}\nfn f() {\n}\n", "f", Err(ScopeError::Ambiguous { lines: vec![1, 3] })),
        ("fn o() {\n  if x {\n}\n", "o", Err(ScopeError::EndNotFound)),
        ("impl A {\n  fn f(&self) {}\n}\nimpl<T> A {\n  fn g(&self) {}\n}\n", "A::g", Ok((5, 5))),
        (
            "impl A {\n  fn f(&self) {\n    fn n() {}\n  }\n}\n",
            "A::n",
            Err(ScopeError::NotFound("member 'n' not found".into())),
        ),
        ("impl fmt::Display for A {\n  fn fmt(&self) {}\n}\n", "A::<Display>::fmt", Ok((2, 2))),
        ("impl fmt::Display for A {\n  fn fmt(&self) {}\n}\n", "A::<Display>", Ok((1, 3))),
        (
            "impl From<u8> for A {\n  fn from(a: u8) {}\n}\nimpl From<u16> for A {\n  fn from(a: u16) {}\n}\n",
            "A::<From>::from",
            Err(ScopeError::Ambiguous { lines: vec![2, 5] }),
        ),
        (
            "impl A {\n  fn f(&self) {}\n}\n",
            "A::f::g",
            Err(ScopeError::Unsupported(
                "only one container level is supported; a member of a member is not (resolver 'rust')".into(),
            )),
        ),
    ];
    for (text, scope, expected) in rows {
        assert_eq!(span(text, scope), expected, "{scope} in {text:?}");
    }
}

#[test]
fn a_semicolon_ended_item_is_not_swallowed_into_the_next_fn() {
    // `const`, `static`, `type` and `mod x;` end at a `;`, not a brace: under a
    // brace_balanced start they would run on into the next item, so the Rust
    // resolver does not claim them. They fail loudly instead of pinning two items.
    let text = "pub const X: u8 = 1;\npub type T = u8;\npub const fn cf() {\n}\nfn next() {\n}\n";
    assert!(span(text, "X").is_err());
    assert!(span(text, "T").is_err());
    assert_eq!(span(text, "cf"), Ok((3, 4)));
    assert_eq!(span(text, "next"), Ok((5, 6)));
}

// ---- directory and glob locators (BUG-176) -------------------------------------

mod covered_files {
    use crate::config::LocatorKind;
    use crate::locator::{files_pin_text, match_locator_files};
    use std::path::PathBuf;

    fn tracked() -> Vec<PathBuf> {
        [
            "src/a.rs",
            "src/sub/c.rs",
            "src/b.py",
            "docs/x.md",
            "gone/g.rs",
        ]
        .iter()
        .map(PathBuf::from)
        .collect()
    }

    fn matched(path: &str) -> (Option<LocatorKind>, Vec<String>) {
        let m = match_locator_files(path, &tracked(), &|p| {
            p != "gone/g.rs" && tracked().iter().any(|t| t.to_str() == Some(p))
        })
        .unwrap();
        (m.kind, m.files)
    }

    #[test]
    fn a_file_a_directory_and_a_glob_are_told_apart() {
        assert_eq!(
            matched("src/a.rs"),
            (Some(LocatorKind::File), vec!["src/a.rs".to_string()])
        );
        let (kind, files) = matched("src");
        assert_eq!(kind, Some(LocatorKind::Directory));
        assert_eq!(files, ["src/a.rs", "src/b.py", "src/sub/c.rs"]);
        let (kind, files) = matched("src/*.rs");
        assert_eq!(kind, Some(LocatorKind::Glob));
        assert_eq!(files, ["src/a.rs"]);
        assert_eq!(matched("src/**/*.rs").1, ["src/a.rs", "src/sub/c.rs"]);
    }

    #[test]
    fn a_name_that_matches_nothing_has_no_kind_but_a_glob_is_always_a_glob() {
        assert_eq!(matched("nope"), (None, Vec::new()));
        assert_eq!(
            matched("src/a"),
            (None, Vec::new()),
            "a prefix of a name is not a directory"
        );
        assert_eq!(matched("src/*.go"), (Some(LocatorKind::Glob), Vec::new()));
        // A file that is not present (a staged deletion) is not matched.
        assert_eq!(matched("gone"), (None, Vec::new()));
        assert_eq!(matched("."), (None, Vec::new()));
    }

    #[test]
    fn a_glob_that_cannot_compile_is_an_error() {
        assert!(match_locator_files("src/[", &tracked(), &|_| false).is_err());
    }

    #[test]
    fn the_pin_text_is_the_sorted_path_and_hash_lines() {
        let a = ("src/a.rs".to_string(), "aa".to_string());
        let b = ("src/b.rs".to_string(), "bb".to_string());
        let text = files_pin_text(&[b.clone(), a.clone()]);
        assert_eq!(text, "src/a.rs\taa\nsrc/b.rs\tbb\n");
        assert_eq!(text, files_pin_text(&[a, b]));
    }
}
