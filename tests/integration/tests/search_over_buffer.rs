//! Seam: `bp-search`'s matches against `bp-buffer`'s rope and `bp-editor`'s
//! positions.
//!
//! Three crates, three ways of naming a place in a document, and no test
//! anywhere that they name the same place. `bp-search` works in character
//! offsets over a `&str`; `bp-buffer` works in character offsets over a rope;
//! `bp-editor` turns those into a 1-based line and column for the status bar
//! and the "go to line" box. Each is internally consistent. The defect lives
//! between them, and the two document features that expose it are the two
//! this editor explicitly supports: multi-byte characters and CRLF.
//!
//! What a user sees when this breaks is a find that highlights the wrong
//! text, or a results panel whose line numbers scroll the editor to the wrong
//! place -- both of which look like the search being "a bit off" rather than
//! like a bug with a cause.
//!
//! The fixtures are built from a small alphabet spelled out below, so nothing
//! that could belong to anybody reaches a failure report.

mod common;

use common::no_files;

use bp_buffer::{Buffer, Position};
use bp_editor::Editor;
use bp_search::{Query, count, find_all, next_match, plan_replace_all, previous_match};
use proptest::prelude::*;

/// Tokens a document is assembled from.
///
/// `é` is two bytes, `中` three and `😀` four, so any place a byte offset was
/// used where a character offset was meant lands inside one of them. `\r\n`
/// is here because a CRLF document is two characters per line break and one
/// line break, and something has to reconcile that. A bare `\r` is a line
/// break too, and a form feed and U+2028 are not (decision D6); a `\r`
/// drawn before a `\n` makes a CRLF, which is the pairing most likely to be
/// counted twice.
const TOKENS: &[&str] = &[
    "needle", "a", " ", "é", "中", "😀", "\n", "\r\n", "\r", "\x0C", "\u{2028}", "haystack", "\t",
];

fn document() -> impl Strategy<Value = String> {
    proptest::collection::vec(proptest::sample::select(TOKENS), 0..60)
        .prop_map(|parts| parts.concat())
}

/// The patterns worth searching for in the documents above.
fn pattern() -> impl Strategy<Value = String> {
    proptest::sample::select(&["needle", "é", "中", "😀", "a", " ", "haystack", "e"][..])
        .prop_map(str::to_owned)
}

proptest! {
    #![proptest_config(no_files(256))]

    /// **A match's range names the same characters in the rope that it names
    /// in the string.**
    ///
    /// This is the byte-versus-character property. `bp-search` documents its
    /// offsets as character offsets precisely so they can be handed to the
    /// editor to select a match; if either side ever counted bytes, a
    /// document containing `é` would slice somewhere else -- and on the rope
    /// side, in the middle of a character.
    #[test]
    fn a_match_names_the_same_characters_in_the_rope(
        text in document(),
        pattern in pattern(),
    ) {
        let buffer = Buffer::from_text(&text);
        let matches = find_all(&text, &Query::literal(&pattern)).expect("literal query");

        for m in &matches {
            prop_assert!(
                m.range.end <= buffer.len_chars(),
                "a match ends at character {} of a {} character buffer",
                m.range.end,
                buffer.len_chars()
            );
            prop_assert_eq!(
                buffer.slice(m.range.clone()),
                m.text.clone(),
                "the rope holds something else at characters {}..{}",
                m.range.start,
                m.range.end
            );
        }
    }

    /// The count and the list agree. Two implementations of "how many", and
    /// the status bar shows one while the panel lists the other.
    #[test]
    fn counting_agrees_with_listing(text in document(), pattern in pattern()) {
        let query = Query::literal(&pattern);
        prop_assert_eq!(
            count(&text, &query).expect("count"),
            find_all(&text, &query).expect("find").len()
        );
    }

    /// **A match's reported line is the line the buffer puts it on**, for
    /// documents whose line breaks are LF and CRLF.
    ///
    /// Restricted to those two; the version below also mixes in bare CRs,
    /// which have ended a line everywhere since D6 was answered.
    #[test]
    fn a_matchs_line_number_agrees_with_the_buffer(
        parts in proptest::collection::vec(
            proptest::sample::select(&["needle", "a", "é", "中", "😀", "\n", "\r\n", " "][..]),
            0..60,
        ),
        pattern in pattern(),
    ) {
        let text: String = parts.concat();
        let buffer = Buffer::from_text(&text);

        for m in find_all(&text, &Query::literal(&pattern)).expect("find") {
            prop_assert_eq!(
                m.line,
                buffer.position_of(m.range.start).line,
                "search says line {} for a match at character {}, the buffer says line {}",
                m.line,
                m.range.start,
                buffer.position_of(m.range.start).line
            );
        }
    }

    /// **Going to a match's line and column puts the caret on the match.**
    ///
    /// The journey a "find in files" result actually makes: search reports a
    /// line, the editor is asked for it, and the caret has to land on the
    /// text the user was shown. `go_to_position` clamps rather than refusing,
    /// so an off-by-one here does not error -- it silently lands elsewhere,
    /// which is why it needs asserting rather than trusting.
    #[test]
    fn going_to_a_matchs_position_lands_on_the_match(
        parts in proptest::collection::vec(
            proptest::sample::select(&["needle", "a", "é", "中", "😀", "\n", "\r\n", " "][..]),
            0..60,
        ),
        pattern in pattern(),
    ) {
        let text: String = parts.concat();
        let buffer = Buffer::from_text(&text);
        let mut editor = Editor::new(&text);

        for m in find_all(&text, &Query::literal(&pattern)).expect("find") {
            let position = buffer.position_of(m.range.start);
            let in_range = editor.go_to_position(Position::new(position.line, position.column));
            prop_assert!(in_range, "a position the buffer produced was out of range");
            prop_assert_eq!(
                editor.cursor(),
                m.range.start,
                "the caret landed at character {} instead of {}",
                editor.cursor(),
                m.range.start
            );
        }
    }

    /// **`next_match` and `previous_match` reach every match and no other
    /// place.** Stepping forward from each match's own start lands on a real
    /// match, and stepping forward from the last one wraps to the first.
    #[test]
    fn stepping_through_matches_stays_on_matches(
        text in document(),
        pattern in pattern(),
    ) {
        let matches = find_all(&text, &Query::literal(&pattern)).expect("find");
        prop_assume!(!matches.is_empty());

        for m in &matches {
            let forward = next_match(&matches, m.range.start).expect("a match");
            prop_assert_eq!(forward.range.start, m.range.start);

            let back = previous_match(&matches, m.range.start);
            let back = back.expect("a match");
            prop_assert!(
                matches.iter().any(|other| other.range == back.range),
                "stepping back left the set of matches"
            );
        }

        let last = matches.last().expect("a match");
        let wrapped = next_match(&matches, last.range.end + 1).expect("a match");
        prop_assert_eq!(
            wrapped.range.start,
            matches[0].range.start,
            "the search did not wrap to the first match"
        );
    }

    /// **A replace plan applied through the rope produces the document the
    /// plan promised.**
    ///
    /// The preview the user confirms is `ReplacePlan::apply`'s string; what
    /// the editor actually does is remove and insert character ranges in the
    /// rope. Two implementations of the same edit, and the only reason they
    /// agree is that the offsets mean the same thing in both. Applied back to
    /// front so that earlier ranges are still valid after a later edit.
    #[test]
    fn a_replace_plan_applied_to_the_rope_matches_its_own_preview(
        text in document(),
        pattern in pattern(),
        replacement in proptest::sample::select(&["X", "", "ﬁ", "two words", "😀😀"][..]),
    ) {
        let query = Query::literal(&pattern);
        let plan = plan_replace_all(&text, &query, replacement).expect("plan");
        let expected = plan.replacements.clone();
        let promised = plan.apply();

        let mut buffer = Buffer::from_text(&text);
        for change in expected.iter().rev() {
            prop_assert_eq!(
                buffer.slice(change.range.clone()),
                change.before.clone(),
                "the plan describes text the rope does not hold at {}..{}",
                change.range.start,
                change.range.end
            );
            buffer.remove(change.range.clone());
            buffer.insert(change.range.start, &change.after);
        }

        prop_assert_eq!(
            buffer.to_string(),
            promised,
            "applying the plan to the rope produced a different document from its own preview"
        );
    }
}

// --- named cases ---------------------------------------------------------

#[test]
fn a_match_after_multi_byte_text_is_selected_correctly() {
    // Eight characters, seventeen bytes, before the match. A byte offset here
    // would select from the middle of the run.
    let text = "é中😀 é中😀 needle";
    let buffer = Buffer::from_text(text);

    let m = &find_all(text, &Query::literal("needle")).expect("find")[0];

    assert_eq!(m.range.start, text.chars().count() - "needle".len());
    assert_eq!(buffer.slice(m.range.clone()), "needle");
    assert_eq!(buffer.position_of(m.range.start), Position::new(1, 9));
}

#[test]
fn a_match_on_the_third_line_of_a_crlf_document_is_reported_as_line_three() {
    let text = "one\r\ntwo\r\nneedle\r\nfour\r\n";
    let buffer = Buffer::from_text(text);

    let m = &find_all(text, &Query::literal("needle")).expect("find")[0];

    assert_eq!(m.line, 3, "search disagrees with the document");
    assert_eq!(buffer.position_of(m.range.start), Position::new(3, 1));
    assert_eq!(buffer.slice(m.range.clone()), "needle");
}

#[test]
fn searching_an_empty_document_finds_nothing_rather_than_everything() {
    assert!(
        find_all("", &Query::literal("needle"))
            .expect("find")
            .is_empty()
    );
    // And an empty pattern is an empty find box, not a match at every offset.
    assert!(
        find_all("some text", &Query::literal(""))
            .expect("find")
            .is_empty()
    );
}

// --- what a line is (decision D6) ------------------------------------------

/// `bp-search` and `bp-buffer` agree on what a line is.
///
/// They used not to. `bp_search` counted line starts at `'\n'` only, while
/// `bp_buffer::Buffer` was a `ropey::Rope` with ropey's default
/// `unicode_lines`, which also broke lines at a bare `\r`, `\x0B`, `\x0C`,
/// U+0085, U+2028 and U+2029. For a document containing any of those a find
/// reported one line number and the caret another, and Find in Files scrolled
/// the editor to the wrong line. A bare `\r` is not exotic: it is what a file
/// written on a classic Mac, or by a program that drew a progress bar,
/// contains.
///
/// The owner's answer (D6) is the Notepad rule: a line ends at LF, CRLF or a
/// bare CR, and nothing else, everywhere. The rope is built with ropey's
/// `cr_lines`, and `bp-search` counts the same set. So a bare CR moves the
/// needle to line 3 in both, and every other separator leaves it on line 1
/// in both.
#[test]
fn search_and_the_buffer_agree_on_lines_containing_any_line_break() {
    for (text, breaks, line) in [
        ("one\rtwo\rneedle", "a bare carriage return", 3),
        ("one\r\ntwo\rneedle", "a CRLF then a bare CR", 3),
        ("one\r\rneedle", "two bare carriage returns", 3),
        ("one\u{2028}two\u{2028}needle", "U+2028 LINE SEPARATOR", 1),
        (
            "one\u{2029}two\u{2029}needle",
            "U+2029 PARAGRAPH SEPARATOR",
            1,
        ),
        ("one\u{0085}two\u{0085}needle", "U+0085 NEXT LINE", 1),
        ("one\x0Btwo\x0Bneedle", "a vertical tab", 1),
        ("one\x0Ctwo\x0Cneedle", "a form feed", 1),
    ] {
        let buffer = Buffer::from_text(text);
        let m = &find_all(text, &Query::literal("needle")).expect("find")[0];

        assert_eq!(
            m.line,
            buffer.position_of(m.range.start).line,
            "search and the buffer disagree about a document separated by {breaks}"
        );
        assert_eq!(m.line, line, "{breaks} is not a break of the kind D6 says");
        assert_eq!(
            bp_buffer::line_count(text),
            buffer.len_lines(),
            "bp-buffer's own two line counts disagree about {breaks}"
        );
    }
}

/// The same agreement through the two places a user actually meets it: Find
/// in Files names a line, and the editor's caret, once the file is open and
/// the match selected, is on that line.
#[test]
fn a_bare_cr_document_reports_the_same_line_in_find_in_files_and_in_the_editor() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let text = "first\rsecond\r\nthird\x0Cstill third\rthe needle\r";
    std::fs::write(dir.path().join("mac.txt"), text).expect("write the fixture");

    let report = bp_search::search_dir(dir.path(), &Query::literal("needle")).expect("search");
    let hit = &report.hits[0];

    let mut editor = Editor::new(text);
    editor.select(hit.offset, hit.offset + "needle".chars().count());
    assert_eq!(
        hit.line, 4,
        "Find in Files counts the bare CRs and the CRLF once"
    );
    assert_eq!(
        editor.position().line,
        hit.line,
        "the editor puts the caret on a different line from the one Find in Files named"
    );
    assert_eq!(hit.preview, "the needle", "the preview is of that line");

    // And Go to Line with that number lands on the line that holds it.
    assert!(editor.go_to_position(Position::new(hit.line, 1)));
    let line = editor.buffer().line(hit.line - 1);
    assert_eq!(bp_buffer::breaks::strip(&line), "the needle");
}

/// Every crate that numbers lines, against the rope, on one document that
/// uses each kind of break and each separator that is not one.
///
/// `bp-secrets`, `bp-semantic` and `bp-redaction` do not depend on
/// `bp-buffer` -- they are leaf crates on purpose -- so each restates the
/// rule. This is what holds them to it: a scan result, an outline entry or a
/// metadata row that names a line the gutter numbers differently sends the
/// user to the wrong place exactly as Find in Files used to.
#[test]
fn every_crate_that_numbers_lines_agrees_with_the_buffer() {
    // A fabricated AWS key id, as in `bp-secrets`' own tests: well formed and
    // never issued.
    let lines = [
        "intro\x0Cstill intro\u{2028}and more\u{0085}and more",
        "# Heading",
        "author: Daniel Boles",
        "id AKIA3G7QVHBRN2WPKZ5F",
        "the needle",
    ];
    for breaks in [
        ["\r", "\r\n", "\n", "\r"],
        ["\n", "\n", "\n", "\n"],
        ["\r\n", "\r", "\r", "\r\n"],
    ] {
        let mut text = String::new();
        for (line, end) in lines.iter().zip(breaks.iter().chain([&""])) {
            text.push_str(line);
            text.push_str(end);
        }
        let buffer = Buffer::from_text(&text);
        let line_of_byte = |byte: usize| buffer.position_of(text[..byte].chars().count()).line;
        let what = format!("breaks {breaks:?}");

        assert_eq!(buffer.len_lines(), 5, "{what}");
        assert_eq!(bp_semantic::statistics(&text).lines, 5, "{what}");

        let heading = &bp_semantic::outline(&text)[0];
        assert_eq!(heading.line, 2, "{what}");
        assert_eq!(heading.line, line_of_byte(text.find("# Heading").unwrap()));

        let author = &bp_redaction::inspect(&text)[0];
        assert_eq!(author.line, 3, "{what}");
        assert_eq!(author.line, line_of_byte(author.start));

        let key = &bp_secrets::scan(&text)[0];
        assert_eq!(key.line, 4, "{what}");
        assert_eq!(key.line, line_of_byte(text.find("AKIA").unwrap()));

        let needle = &find_all(&text, &Query::literal("needle")).unwrap()[0];
        assert_eq!(needle.line, 5, "{what}");
        assert_eq!(needle.line, buffer.position_of(needle.range.start).line);
    }
}
