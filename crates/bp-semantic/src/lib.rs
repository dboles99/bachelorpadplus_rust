//! Deterministic extraction: titles, keywords, summaries, outlines.
//!
//! specs.md section 10 describes three layers — deterministic extraction,
//! local statistical models, and optional generative providers. **This crate
//! is layer one and only layer one.** It reads the document in front of it
//! and reports what is demonstrably there.
//!
//! That constraint is ADR-0006, not modesty: core behaviour must work with
//! every semantic and cloud feature disabled, so naming a note cannot depend
//! on a model being loaded or a network being up. Everything here is a pure
//! function of the text, which also means every rule is testable and every
//! answer explainable — the user can see *why* their note got that title.

#![forbid(unsafe_code)]

/// Crate identity used by workspace smoke tests and diagnostics.
pub const CRATE_NAME: &str = "bp-semantic";

/// Longest title we will propose. Long enough to be descriptive, short
/// enough to leave room for the date and extension inside a filename.
pub const MAX_TITLE_CHARS: usize = 60;

/// Words too common to describe anything.
///
/// Deliberately small and English-only. A large list would silently discard
/// terms that matter in a technical note, and a wrong keyword is worse than a
/// dull one.
const STOPWORDS: &[&str] = &[
    "a", "an", "the", "and", "or", "but", "if", "then", "else", "for", "of", "to", "in", "on",
    "at", "by", "with", "from", "as", "is", "are", "was", "were", "be", "been", "being", "it",
    "its", "this", "that", "these", "those", "there", "here", "we", "you", "they", "he", "she",
    "i", "not", "no", "yes", "do", "does", "did", "have", "has", "had", "will", "would", "can",
    "could", "should", "may", "might", "must", "so", "than", "too", "very", "just", "also", "into",
    "over", "under", "about", "after", "before", "when", "while", "how", "what", "why", "which",
    "who", "all", "any", "some", "more", "most", "other", "such", "only", "own", "same",
];

/// A heading found in a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    /// 1 for `#`, 2 for `##`, and so on.
    pub level: usize,
    pub text: String,
    /// 1-based line the heading is on.
    pub line: usize,
}

/// Propose a human-readable title for a document.
///
/// In order of preference: the first Markdown heading, then the first
/// non-empty line. Returns `None` for a document with nothing in it — an
/// empty note has no title, and inventing one would be worse than admitting
/// that.
pub fn suggest_title(text: &str) -> Option<String> {
    let from_heading = outline(text).into_iter().next().map(|h| h.text);
    let candidate = from_heading.or_else(|| {
        lines(text)
            .map(str::trim)
            .find(|l| !l.is_empty())
            .map(clean_line)
    })?;

    let title = tidy(&candidate);
    (!title.is_empty()).then_some(title)
}

/// The lines of `text` without their breaks, on the editor's rule (decision
/// D6): a line ends at LF, CRLF or a bare CR, and nothing else. Shaped like
/// `str::lines`, which does not end a line at a bare CR: nothing for the
/// empty line after a trailing break, and nothing at all for empty text.
///
/// `bp-buffer` holds the rule; this crate does not depend on it, so it is
/// restated here and `tests/integration` holds the two to it. It is what
/// keeps a heading's line number the one the gutter shows.
fn lines(text: &str) -> impl Iterator<Item = &str> {
    let mut rest = Some(text).filter(|t| !t.is_empty());
    std::iter::from_fn(move || {
        let current = rest?;
        let Some(end) = current.find(['\n', '\r']) else {
            rest = None;
            return Some(current);
        };
        // Both break bytes are ASCII, so every cut is a character boundary.
        let len = if current[end..].starts_with("\r\n") {
            2
        } else {
            1
        };
        rest = Some(&current[end + len..]).filter(|r| !r.is_empty());
        Some(&current[..end])
    })
}

/// Strip the punctuation and markup that decorate a line but do not describe
/// it, so a title taken from prose reads like a title.
fn clean_line(line: &str) -> String {
    let line = line
        .trim_start_matches(['#', '>', '-', '*', '=', '/', ';'])
        .trim();
    // Common comment and front-matter openers.
    let line = line
        .trim_start_matches("//")
        .trim_start_matches("--")
        .trim_start_matches("<!--")
        .trim_end_matches("-->")
        .trim();
    line.to_owned()
}

/// Collapse whitespace and truncate on a word boundary.
fn tidy(text: &str) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= MAX_TITLE_CHARS {
        return collapsed;
    }

    // Cut at the last space inside the budget so a title never ends
    // mid-word; fall back to a hard cut for text with no spaces at all.
    let budget: String = collapsed.chars().take(MAX_TITLE_CHARS).collect();
    match budget.rfind(' ') {
        Some(space) if space > MAX_TITLE_CHARS / 2 => budget[..space].trim_end().to_owned(),
        _ => budget.trim_end().to_owned(),
    }
}

/// Markdown headings, in document order.
///
/// ATX style (`#`) only; setext underlining is ambiguous with horizontal
/// rules and prose, and guessing wrong puts nonsense in the outline.
///
/// Code is not prose: a `# setup` comment inside a fenced block, or in a
/// block indented as code, is a line of Python or shell, and listing it
/// beside the real headings makes the outline lie about the document. The
/// rules follow CommonMark closely enough to be right in practice -- see
/// `Fence` and `block_indent` -- without pulling in a Markdown parser
/// for the sake of one list.
pub fn outline(text: &str) -> Vec<Heading> {
    let mut open: Option<Fence> = None;
    let mut headings = Vec::new();

    // Enumerating every line, including the ones skipped, is what keeps
    // `Heading::line` the line the user sees in the gutter.
    for (index, raw) in lines(text).enumerate() {
        // Four columns in is indented code outside a fence, and cannot be a
        // closing fence inside one, so either way it is nothing to us.
        let Some(line) = block_indent(raw) else {
            continue;
        };

        if let Some(fence) = open {
            if fence.is_closed_by(line) {
                open = None;
            }
            continue;
        }
        if let Some(fence) = Fence::opened_by(line) {
            open = Some(fence);
            continue;
        }
        if let Some(heading) = atx_heading(line, index + 1) {
            headings.push(heading);
        }
    }
    // A fence still open here simply ran to the end of the document, which
    // is what CommonMark says an unclosed fence does: nothing after it was
    // ever a heading.
    headings
}

/// An open fenced code block: the character that opened it and how many.
///
/// Both matter for closing it. A fence is closed only by a run of the *same*
/// character at least as long as the opener, which is exactly what lets a
/// document show a three-backtick example inside a four-backtick block --
/// the inner ``` is content, not the end.
#[derive(Debug, Clone, Copy)]
struct Fence {
    marker: u8,
    len: usize,
}

/// The shortest run of backticks or tildes that opens a fence.
const MIN_FENCE: usize = 3;

impl Fence {
    /// The fence `line` opens, if it opens one.
    ///
    /// `line` has already had its (at most three columns of) indentation
    /// removed by `block_indent`.
    fn opened_by(line: &str) -> Option<Self> {
        let marker = *line.as_bytes().first()?;
        if marker != b'`' && marker != b'~' {
            return None;
        }
        let len = marker_run(line, marker);
        if len < MIN_FENCE {
            return None;
        }
        // A backtick in the info string means this is inline code that
        // happens to start a line (```a` b```), not a fence. Tilde fences
        // have no such restriction.
        let info = line.get(len..).unwrap_or_default();
        if marker == b'`' && info.contains('`') {
            return None;
        }
        Some(Self { marker, len })
    }

    /// Whether `line` closes this fence: the same character, at least as
    /// many of it, and nothing after but spaces or tabs.
    fn is_closed_by(self, line: &str) -> bool {
        let len = marker_run(line, self.marker);
        len >= self.len
            && line
                .get(len..)
                .is_some_and(|rest| rest.trim_matches([' ', '\t']).is_empty())
    }
}

/// How many times `marker` repeats at the start of `line`.
fn marker_run(line: &str, marker: u8) -> usize {
    line.bytes().take_while(|b| *b == marker).count()
}

/// `line` without its indentation, or `None` if it is indented four columns
/// or more.
///
/// CommonMark allows a heading or a fence up to three spaces of indentation;
/// at four the line is an indented code block (or the continuation of a
/// paragraph), and neither is a heading. A tab advances to the next multiple
/// of four, as CommonMark counts it, so a single leading tab is already
/// code. The one case this gets wrong is a heading nested inside a list
/// item, which CommonMark measures from the item's content rather than the
/// margin; that is rare in notes, whereas `    # comment` in pasted code is
/// not.
fn block_indent(line: &str) -> Option<&str> {
    const CODE_INDENT: usize = 4;
    let mut column = 0;
    for (offset, c) in line.char_indices() {
        match c {
            ' ' => column += 1,
            '\t' => column = (column / CODE_INDENT + 1) * CODE_INDENT,
            _ => {
                return (column < CODE_INDENT).then(|| line.get(offset..)).flatten();
            }
        }
        if column >= CODE_INDENT {
            return None;
        }
    }
    // Blank or whitespace-only: nothing to outline, but nothing that ends
    // or opens a fence either.
    Some("")
}

/// The ATX heading on `line`, which has already had its indentation
/// removed.
fn atx_heading(line: &str, number: usize) -> Option<Heading> {
    if !line.starts_with('#') {
        return None;
    }
    let level = marker_run(line, b'#');
    if level > 6 {
        return None;
    }
    // A heading needs a space after its hashes; `#hashtag` is not one.
    let rest = line.get(level..)?.strip_prefix(' ')?.trim();
    (!rest.is_empty()).then(|| Heading {
        level,
        text: rest.trim_end_matches('#').trim().to_owned(),
        line: number,
    })
}

/// The most frequent meaningful words, most common first.
///
/// Frequency only — no weighting, no corpus. It is crude, and being crude is
/// the point: the user can see why each word was chosen.
pub fn keywords(text: &str, limit: usize) -> Vec<String> {
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for word in text.split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_') {
        let word = word.trim_matches(['-', '_']).to_lowercase();
        // Two letters is the shortest that carries meaning ("id", "os");
        // one-letter tokens are noise.
        if word.chars().count() < 2 || STOPWORDS.contains(&word.as_str()) {
            continue;
        }
        // Skip bare numbers: a note full of dates should not be "keyworded"
        // with them.
        if word.chars().all(|c| c.is_numeric()) {
            continue;
        }
        *counts.entry(word).or_default() += 1;
    }

    let mut ranked: Vec<(String, usize)> = counts.into_iter().collect();
    // Frequency first, then alphabetically, so the result is stable rather
    // than dependent on hash order.
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked.into_iter().take(limit).map(|(w, _)| w).collect()
}

/// The first paragraph, collapsed and truncated.
///
/// A paragraph rather than the first N characters: cutting mid-sentence
/// produces a summary that reads like a fault.
pub fn summary(text: &str, max_chars: usize) -> String {
    let mut paragraph = String::new();
    for line in lines(text) {
        let trimmed = line.trim();
        // Skip headings, so the summary is the prose under the title rather
        // than the title again.
        if trimmed.is_empty() || trimmed.starts_with('#') {
            if paragraph.is_empty() {
                continue;
            }
            break;
        }
        if !paragraph.is_empty() {
            paragraph.push(' ');
        }
        paragraph.push_str(trimmed);
    }

    let collapsed = paragraph.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= max_chars {
        return collapsed;
    }
    let budget: String = collapsed.chars().take(max_chars).collect();
    match budget.rfind(' ') {
        Some(space) if space > max_chars / 2 => format!("{}…", budget[..space].trim_end()),
        _ => format!("{}…", budget.trim_end()),
    }
}

/// What a document is made of, for the status bar and the inspector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Statistics {
    pub characters: usize,
    /// Characters excluding whitespace.
    pub characters_no_whitespace: usize,
    pub words: usize,
    pub lines: usize,
    pub paragraphs: usize,
}

/// Count characters, words, lines and paragraphs in `text`.
///
/// The shell hands this the whole document -- after every edit, for the
/// status bar -- so this is one pass over the characters rather than
/// `chars().count()`, `split_whitespace().count()`, `lines().count()` and a
/// separate paragraph scan run back to back over the same text.
pub fn statistics(text: &str) -> Statistics {
    let mut characters = 0usize;
    let mut characters_no_whitespace = 0usize;
    let mut words = 0usize;
    // A document is at least one line even when it is empty -- the caret
    // has to be somewhere -- so this starts at one, not zero.
    let mut lines = 1usize;
    let mut paragraphs = 0usize;

    // A word is a maximal run of alphanumeric characters. A hyphen or
    // apostrophe extends the current word only when it sits between two
    // alphanumerics, so "state-of-the-art" and "don't" each count once;
    // anything else (leading, trailing, or doubled) is ordinary punctuation
    // and ends the word instead of gluing it to whatever follows. Telling
    // those apart needs one character of memory -- was the previous
    // character such a candidate joiner -- rather than lookahead, which is
    // what keeps this a single pass.
    #[derive(PartialEq)]
    enum Word {
        Outside,
        Inside,
        /// Just saw a possible joiner right after `Inside`; not confirmed
        /// until we see what comes next.
        Joining,
    }
    let mut word = Word::Outside;

    // A paragraph is a run of non-blank lines; a line that is empty or
    // whitespace-only separates paragraphs rather than belonging to one.
    // Several blank lines in a row still separate exactly two paragraphs,
    // not several -- a gap is a gap, and counting each blank line would
    // turn "two gaps between three paragraphs" into a number nobody asked
    // for, so `in_paragraph` only flips once per run either way.
    let mut line_has_content = false;
    let mut in_paragraph = false;

    let mut stream = text.chars().peekable();
    while let Some(mut c) = stream.next() {
        // `\r` is line-ending encoding, not content. Half of a CRLF is
        // dropped, and a bare one is a line break in its own right (decision
        // D6) and read as `\n`, so the character stream for a CRLF or a
        // classic Mac document is identical to its LF equivalent and every
        // field below agrees between them, not just the line count.
        if c == '\r' {
            if stream.peek() == Some(&'\n') {
                continue;
            }
            c = '\n';
        }

        characters += 1;
        if !c.is_whitespace() {
            characters_no_whitespace += 1;
        }

        if c.is_alphanumeric() {
            if word == Word::Outside {
                words += 1;
            }
            word = Word::Inside;
        } else if matches!(c, '-' | '\'' | '\u{2019}') && word == Word::Inside {
            word = Word::Joining;
        } else {
            word = Word::Outside;
        }

        if c == '\n' {
            // Must agree with `bp_buffer::line_count`'s convention: a
            // trailing newline starts a new, empty line, so "a\n" is two
            // lines, not the one `str::lines()` would report. The two
            // implementations must never disagree, or the editor's gutter
            // and this crate's status bar would number the document
            // differently.
            lines += 1;
            if line_has_content {
                if !in_paragraph {
                    paragraphs += 1;
                }
                in_paragraph = true;
            } else {
                in_paragraph = false;
            }
            line_has_content = false;
        } else if !c.is_whitespace() {
            line_has_content = true;
        }
    }
    // The final line has no trailing '\n' to trigger the check above.
    if line_has_content && !in_paragraph {
        paragraphs += 1;
    }

    Statistics {
        characters,
        characters_no_whitespace,
        words,
        lines,
        paragraphs,
    }
}

impl Statistics {
    /// One line for the status bar.
    pub fn summary(&self) -> String {
        format!(
            "{} word{}, {} line{}",
            self.words,
            if self.words == 1 { "" } else { "s" },
            self.lines,
            if self.lines == 1 { "" } else { "s" },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_markdown_heading_becomes_the_title() {
        let text = "# Rust Migration Notes\n\nSome prose here.";
        assert_eq!(suggest_title(text).as_deref(), Some("Rust Migration Notes"));
    }

    #[test]
    fn the_first_line_is_the_fallback() {
        let text = "Meeting with the editors\n\nMore detail below.";
        assert_eq!(
            suggest_title(text).as_deref(),
            Some("Meeting with the editors")
        );
    }

    #[test]
    fn leading_decoration_is_stripped() {
        assert_eq!(suggest_title("// TODO list").as_deref(), Some("TODO list"));
        assert_eq!(
            suggest_title("> A quotation").as_deref(),
            Some("A quotation")
        );
        assert_eq!(suggest_title("- a bullet").as_deref(), Some("a bullet"));
    }

    #[test]
    fn an_empty_document_has_no_title() {
        // Inventing one would be worse than admitting there is none.
        assert_eq!(suggest_title(""), None);
        assert_eq!(suggest_title("   \n\n  \t"), None);
        assert_eq!(
            suggest_title("###"),
            None,
            "decoration alone is not a title"
        );
    }

    #[test]
    fn long_titles_are_cut_on_a_word_boundary() {
        let text = "The quick brown fox jumps over the lazy dog and keeps on running for miles";
        let title = suggest_title(text).unwrap();

        assert!(title.chars().count() <= MAX_TITLE_CHARS);
        assert!(!title.ends_with(' '));
        assert!(
            text.starts_with(&title),
            "a truncated title must still be a prefix of the source"
        );
        assert!(!title.ends_with("runn"), "must not stop mid-word: {title}");
    }

    #[test]
    fn a_title_with_no_spaces_still_fits() {
        let text = "x".repeat(200);
        let title = suggest_title(&text).unwrap();
        assert_eq!(title.chars().count(), MAX_TITLE_CHARS);
    }

    #[test]
    fn whitespace_is_collapsed() {
        assert_eq!(
            suggest_title("Spaced    out\ttitle").as_deref(),
            Some("Spaced out title")
        );
    }

    #[test]
    fn outline_finds_headings_with_levels_and_lines() {
        let text = "# One\n\ntext\n\n## Two\n\n### Three\n";
        let outline = outline(text);

        assert_eq!(outline.len(), 3);
        assert_eq!(
            outline[0],
            Heading {
                level: 1,
                text: "One".to_owned(),
                line: 1
            }
        );
        assert_eq!(outline[1].level, 2);
        assert_eq!(outline[1].line, 5);
        assert_eq!(outline[2].text, "Three");
    }

    #[test]
    fn a_hashtag_is_not_a_heading() {
        // The space after the hashes is what makes it a heading.
        assert!(outline("#hashtag not a heading").is_empty());
        assert_eq!(outline("# real heading").len(), 1);
    }

    #[test]
    fn closing_hashes_are_trimmed() {
        assert_eq!(outline("## Middle ##")[0].text, "Middle");
    }

    #[test]
    fn seven_hashes_is_not_a_heading() {
        assert!(outline("####### too deep").is_empty());
        assert_eq!(outline("###### six is fine").len(), 1);
    }

    /// The text and line of every heading, which is what a fence test cares
    /// about.
    fn texts_and_lines(text: &str) -> Vec<(String, usize)> {
        outline(text)
            .into_iter()
            .map(|h| (h.text, h.line))
            .collect()
    }

    #[test]
    fn a_comment_inside_a_backtick_fence_is_not_a_heading() {
        let text = "# Setup\n\n```python\n# setup\nimport os\n```\n\n## Usage\n";
        assert_eq!(
            texts_and_lines(text),
            vec![("Setup".to_owned(), 1), ("Usage".to_owned(), 8)],
            "the Python comment on line 4 is code, not an outline entry"
        );
    }

    #[test]
    fn a_tilde_fence_hides_its_contents_too() {
        let text = "~~~sh\n# install\n~~~\n# After\n";
        assert_eq!(texts_and_lines(text), vec![("After".to_owned(), 4)]);
    }

    #[test]
    fn a_heading_right_after_a_closing_fence_is_found() {
        let text = "```\ncode\n```\n# Straight After\n";
        assert_eq!(
            texts_and_lines(text),
            vec![("Straight After".to_owned(), 4)],
            "the fence closed on line 3, so line 4 is prose again"
        );
    }

    #[test]
    fn a_shorter_fence_inside_a_longer_one_does_not_close_it() {
        // The usual way to show a fenced example inside a fenced example.
        let text = "````md\n```\n# inner example\n```\n# still inside\n````\n# Out\n";
        assert_eq!(
            texts_and_lines(text),
            vec![("Out".to_owned(), 7)],
            "only four or more backticks close a four-backtick fence"
        );
    }

    #[test]
    fn a_longer_closing_fence_still_closes() {
        let text = "```\n# code\n`````\n# Heading\n";
        assert_eq!(texts_and_lines(text), vec![("Heading".to_owned(), 4)]);
    }

    #[test]
    fn a_fence_is_closed_only_by_its_own_character() {
        let text = "```\n~~~\n# still code\n```\n~~~\n# code again\n~~~\n# Done\n";
        assert_eq!(
            texts_and_lines(text),
            vec![("Done".to_owned(), 8)],
            "tildes inside a backtick fence are content, and vice versa"
        );
    }

    #[test]
    fn a_closing_fence_carries_nothing_but_whitespace() {
        // "``` not a close" has text after it, so the block is still open.
        let text = "```\n``` not a close\n# code\n```  \n# Heading\n";
        assert_eq!(texts_and_lines(text), vec![("Heading".to_owned(), 5)]);
    }

    #[test]
    fn an_unclosed_fence_runs_to_the_end_of_the_document() {
        let text = "# Before\n```\n# not a heading\n\n# nor this\n";
        assert_eq!(texts_and_lines(text), vec![("Before".to_owned(), 1)]);
    }

    #[test]
    fn two_backticks_do_not_open_a_fence() {
        let text = "``\n# Heading\n";
        assert_eq!(texts_and_lines(text), vec![("Heading".to_owned(), 2)]);
    }

    #[test]
    fn a_backtick_in_the_info_string_is_inline_code_not_a_fence() {
        // CommonMark: ```a` b``` is a code span, so nothing was opened.
        let text = "```a` b```\n# Heading\n";
        assert_eq!(texts_and_lines(text), vec![("Heading".to_owned(), 2)]);
    }

    #[test]
    fn fences_and_headings_may_be_indented_up_to_three_spaces() {
        let text = "   ```\n# code\n   ```\n   # Indented Heading\n";
        assert_eq!(
            texts_and_lines(text),
            vec![("Indented Heading".to_owned(), 4)]
        );
    }

    #[test]
    fn four_columns_of_indentation_is_code_not_a_heading() {
        assert!(outline("    # a comment in indented code").is_empty());
        assert!(
            outline("\t# a tab is four columns").is_empty(),
            "a leading tab reaches column four on its own"
        );
        assert_eq!(outline("   # three spaces is still a heading").len(), 1);
    }

    #[test]
    fn an_indented_fence_marker_does_not_close_the_fence() {
        // Four spaces in, the backticks are content of the open block.
        let text = "```\n    ```\n# still code\n```\n# Heading\n";
        assert_eq!(texts_and_lines(text), vec![("Heading".to_owned(), 5)]);
    }

    #[test]
    fn crlf_fences_are_recognised() {
        let text = "```\r\n# code\r\n```\r\n# Heading\r\n";
        assert_eq!(texts_and_lines(text), vec![("Heading".to_owned(), 4)]);
    }

    #[test]
    fn a_fenced_comment_is_not_offered_as_the_title() {
        // The outline feeds the suggested title, so the bug surfaced there
        // too: a note that opens with code would be named after a comment.
        let text = "```python\n# setup\n```\n# Real Title\n";
        assert_eq!(suggest_title(text).as_deref(), Some("Real Title"));
    }

    #[test]
    fn keywords_rank_by_frequency_and_skip_noise() {
        let text = "rust rust rust the the the editor editor migration 2026 2026 2026";
        let keywords = keywords(text, 3);

        assert_eq!(keywords[0], "rust", "most frequent first");
        assert!(!keywords.contains(&"the".to_owned()), "stopword");
        assert!(
            !keywords.iter().any(|k| k == "2026"),
            "a bare number describes nothing"
        );
    }

    #[test]
    fn keywords_are_stable_rather_than_hash_ordered() {
        // Equal frequencies must not reorder between runs.
        let text = "alpha beta gamma alpha beta gamma";
        let first = keywords(text, 3);
        for _ in 0..5 {
            assert_eq!(keywords(text, 3), first);
        }
        assert_eq!(first, vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn the_stopword_list_is_english_and_lowercase() {
        // Guards against a stray word from another language creeping in —
        // it would silently drop a term that matters in someone's notes.
        for word in STOPWORDS {
            assert!(
                word.chars().all(|c| c.is_ascii_lowercase()),
                "{word:?} is not lowercase ASCII"
            );
        }
        let mut sorted = STOPWORDS.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), STOPWORDS.len(), "duplicate stopword");
    }

    #[test]
    fn keywords_are_case_insensitive() {
        let keywords = keywords("Rust rust RUST editor", 1);
        assert_eq!(keywords, vec!["rust"]);
    }

    #[test]
    fn summary_takes_the_first_paragraph_not_the_heading() {
        let text = "# Title\n\nThe first paragraph explains things.\nStill the same paragraph.\n\nA second paragraph.";
        let summary = summary(text, 200);

        assert!(summary.starts_with("The first paragraph"));
        assert!(summary.contains("Still the same paragraph"));
        assert!(!summary.contains("second paragraph"), "stops at the break");
        assert!(!summary.contains("Title"), "the heading is not the summary");
    }

    #[test]
    fn a_long_summary_is_elided_on_a_word_boundary() {
        let text = "word ".repeat(100);
        let summary = summary(&text, 40);

        assert!(summary.ends_with('…'));
        assert!(summary.chars().count() <= 41, "budget plus the ellipsis");
        assert!(!summary.contains("wor…"), "must not cut mid-word");
    }

    #[test]
    fn an_empty_document_summarises_to_nothing() {
        assert_eq!(summary("", 100), "");
        assert_eq!(summary("# Only a heading", 100), "");
    }

    #[test]
    fn extraction_never_panics_on_odd_input() {
        for text in [
            "",
            "\n\n\n",
            "###",
            "\u{0}\u{1}",
            "日本語のテキスト",
            &"x".repeat(10_000),
        ] {
            let _ = suggest_title(text);
            let _ = outline(text);
            let _ = keywords(text, 5);
            let _ = summary(text, 50);
            let _ = statistics(text);
        }
    }

    #[test]
    fn an_empty_document_has_one_empty_line_and_nothing_else() {
        let stats = statistics("");
        assert_eq!(stats.characters, 0);
        assert_eq!(stats.characters_no_whitespace, 0);
        assert_eq!(stats.words, 0, "no words in nothing");
        assert_eq!(stats.lines, 1, "the caret has to be somewhere");
        assert_eq!(stats.paragraphs, 0, "no content, no paragraph");
    }

    #[test]
    fn a_single_word_counts_as_one_of_everything_that_matters() {
        let stats = statistics("hello");
        assert_eq!(stats.characters, 5);
        assert_eq!(stats.words, 1);
        assert_eq!(stats.lines, 1, "no newline at all");
        assert_eq!(stats.paragraphs, 1, "one line of content is one paragraph");
    }

    #[test]
    fn a_trailing_newline_still_counts_as_a_line() {
        // str::lines() would say one; the caret can sit on the second,
        // empty line, and the gutter has to number it.
        let stats = statistics("a\n");
        assert_eq!(stats.lines, 2, "str::lines() says 1, and is wrong here");
    }

    #[test]
    fn bare_cr_documents_count_the_same_as_their_lf_equivalent() {
        // Decision D6: a bare CR is a line break, so a classic Mac file is
        // the same document as its LF form in every count.
        let lf = "# Title\n\nOne para.\nStill it.\n\nTwo\n";
        let cr = lf.replace('\n', "\r");
        assert_eq!(statistics(lf), statistics(&cr));
        assert_eq!(statistics(&cr).lines, 7);
        // And a form feed or U+2028 is not a break.
        assert_eq!(statistics("a\x0Cb\u{2028}c").lines, 1);
    }

    #[test]
    fn an_outline_numbers_headings_on_the_editors_line_rule() {
        let text = "intro\r\x0Cstill intro\r\n# Heading\r";
        let headings = outline(text);
        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].line, 3);
        assert_eq!(
            suggest_title("\r\rTitle here\rbody").as_deref(),
            Some("Title here")
        );
    }

    #[test]
    fn crlf_documents_count_the_same_as_their_lf_equivalent() {
        let lf = "first paragraph, two lines,\nstill one paragraph.\n\nsecond paragraph.\n";
        let crlf =
            "first paragraph, two lines,\r\nstill one paragraph.\r\n\r\nsecond paragraph.\r\n";

        assert_eq!(
            statistics(lf),
            statistics(crlf),
            "\\r is encoding, not content"
        );
    }

    #[test]
    fn multibyte_text_counts_characters_not_bytes() {
        let text = "日本語";
        let stats = statistics(text);
        assert_eq!(text.len(), 9, "three characters, nine UTF-8 bytes");
        assert_eq!(
            stats.characters, 3,
            "must count characters, not the byte length"
        );
        assert_eq!(stats.characters_no_whitespace, 3);
    }

    #[test]
    fn hyphenated_and_apostrophised_words_count_once_each() {
        let stats = statistics("state-of-the-art don't rock'n'roll");
        assert_eq!(
            stats.words, 3,
            "a joining hyphen or apostrophe must not split a word in two"
        );
    }

    #[test]
    fn punctuation_that_is_not_a_joiner_still_splits_words() {
        // A hyphen or apostrophe only glues a word together when it sits
        // between two alphanumerics; leading, trailing or doubled ones are
        // ordinary punctuation.
        let stats = statistics("'quoted' word--word -dash- end-");
        assert_eq!(stats.words, 5, "quoted, word, word, dash, end");
    }

    #[test]
    fn several_consecutive_blank_lines_are_still_one_separator() {
        let one_gap = statistics("first paragraph\n\nsecond paragraph");
        let many_gaps = statistics("first paragraph\n\n\n\n\nsecond paragraph");

        assert_eq!(one_gap.paragraphs, 2);
        assert_eq!(
            many_gaps.paragraphs, 2,
            "a run of blank lines is one separator, not several"
        );
    }

    #[test]
    fn a_document_of_only_whitespace_has_no_words_or_paragraphs() {
        let stats = statistics("   \n\t\n   ");
        assert_eq!(stats.words, 0);
        assert_eq!(stats.paragraphs, 0, "blank lines are not a paragraph");
        assert_eq!(stats.characters_no_whitespace, 0);
    }

    #[test]
    fn summary_reads_as_one_line_for_the_status_bar() {
        assert_eq!(statistics("hello").summary(), "1 word, 1 line");
        assert_eq!(statistics("hello\nworld").summary(), "2 words, 2 lines");
    }
}
