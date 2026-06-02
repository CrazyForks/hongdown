//! MDX mode: protect MDX-only constructs from comrak's CommonMark parsing.
//!
//! comrak is a plain CommonMark + GFM parser with no knowledge of MDX's
//! JavaScript/JSX constructs — ESM `import`/`export` statements, JSX elements
//! and fragments, and `{…}` expressions.  Left to comrak, those constructs are
//! parsed as ordinary paragraph text and then corrupted by Hongdown's
//! punctuation transforms (straight quotes become curly) and Markdown escaping
//! (`*` and `` ` `` get backslash-escaped), with their indentation collapsed.
//!
//! This module implements a protective pre/post-processing pass.  Before the
//! source is handed to comrak, [`protect`] detects the constructs comrak
//! mangles and replaces each with a unique HTML-comment placeholder, recording
//! the original text.  comrak emits HTML comments verbatim (as `HtmlBlock` /
//! `HtmlInline`), so the placeholders survive serialization untouched while the
//! surrounding Markdown prose is formatted normally.  [`restore`] then swaps the
//! placeholders back for their original text.
//!
//! The detection is AST-guided: only the source spans of `Paragraph` and
//! `Heading` nodes are scanned.  Content inside fenced/indented code blocks,
//! inline code spans, math, front matter, and HTML blocks never becomes
//! paragraph text in comrak's AST, so it is naturally left untouched — we only
//! ever touch what comrak would otherwise corrupt.

use comrak::nodes::{NodeValue, Sourcepos};
use comrak::{Arena, Options as ComrakOptions, parse_document};

/// A protected region: the placeholder token that stands in for an MDX
/// construct during formatting, and the original verbatim text to restore.
pub(crate) struct Protected {
    token: String,
    original: String,
}

/// Detect MDX constructs in `source` and replace each with a unique HTML-comment
/// placeholder, returning the protected source and the replacement map.
///
/// Returns `None` when there is nothing to protect (the caller then formats the
/// original source normally).  This is also the do-no-harm fallback: any
/// construct that cannot be delimited confidently is simply left unprotected
/// rather than guessed at.
pub(crate) fn protect(
    source: &str,
    comrak_options: &ComrakOptions,
) -> Option<(String, Vec<Protected>)> {
    let arena = Arena::new();
    let root = parse_document(&arena, source, comrak_options);
    let line_starts = build_line_starts(source);

    // Collect absolute byte ranges of every construct found in a scanned span.
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for node in root.descendants() {
        let (is_target, sourcepos) = {
            let data = node.data.borrow();
            let is_target = matches!(data.value, NodeValue::Paragraph | NodeValue::Heading(_));
            (is_target, data.sourcepos)
        };
        if !is_target {
            continue;
        }
        let Some((start, end)) = span_bytes(source, &line_starts, sourcepos) else {
            continue;
        };
        let span = &source[start..end];
        for (relative_start, relative_end) in find_constructs(span) {
            ranges.push((start + relative_start, start + relative_end));
        }
    }

    if ranges.is_empty() {
        return None;
    }

    ranges.sort_by_key(|&(start, _)| start);

    let nonce = choose_nonce(source);
    let mut protected = String::with_capacity(source.len());
    let mut map = Vec::with_capacity(ranges.len());
    let mut cursor = 0;
    for (index, &(start, end)) in ranges.iter().enumerate() {
        // Defensively skip any range that overlaps an already-protected one.
        if start < cursor {
            continue;
        }
        protected.push_str(&source[cursor..start]);
        let token = format!("<!--hongdown-mdx:{nonce}:{index}-->");
        protected.push_str(&token);
        map.push(Protected {
            token,
            original: source[start..end].to_string(),
        });
        cursor = end;
    }
    protected.push_str(&source[cursor..]);

    Some((protected, map))
}

/// Replace every placeholder token in `output` with its original text.
pub(crate) fn restore(output: &str, map: &[Protected]) -> String {
    let mut result = output.to_string();
    for protected in map {
        result = result.replace(&protected.token, &protected.original);
    }
    result
}

/// Byte offsets of the start of each line in `source` (1-indexed line `n` lives
/// at `line_starts[n - 1]`).
fn build_line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (index, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            starts.push(index + 1);
        }
    }
    starts
}

/// Convert a node's [`Sourcepos`] to an absolute `[start, end)` byte range in
/// `source`.  comrak reports 1-indexed, byte-based line/column positions with an
/// inclusive end column.  Returns `None` if the position is unset or does not
/// land on character boundaries (do-no-harm: skip the node).
fn span_bytes(source: &str, line_starts: &[usize], sourcepos: Sourcepos) -> Option<(usize, usize)> {
    let start_line = sourcepos.start.line;
    let end_line = sourcepos.end.line;
    if start_line == 0
        || end_line == 0
        || start_line > line_starts.len()
        || end_line > line_starts.len()
    {
        return None;
    }
    let start =
        line_starts[start_line - 1].checked_add(sourcepos.start.column.saturating_sub(1))?;
    let end = line_starts[end_line - 1].checked_add(sourcepos.end.column)?;
    if start > end || end > source.len() {
        return None;
    }
    if !source.is_char_boundary(start) || !source.is_char_boundary(end) {
        return None;
    }
    Some((start, end))
}

/// Find MDX construct ranges (byte offsets relative to `span`) within a single
/// scanned span.  Scanning is bounded to the span: a construct that is not
/// fully contained in it is not matched.  In particular an ESM statement with a
/// blank line inside its body is split by comrak into separate paragraphs, so it
/// is left unprotected (a documented limitation) rather than scanning into
/// unrelated later content.
///
/// Currently detects ESM `import`/`export` statements; JSX elements and `{…}`
/// expressions are added incrementally.
fn find_constructs(span: &str) -> Vec<(usize, usize)> {
    let bytes = span.as_bytes();
    let mut ranges = Vec::new();
    let mut index = 0;
    let mut at_line_start = true;
    while index < span.len() {
        if at_line_start {
            // CommonMark paragraphs allow up to three leading spaces.
            let mut scan = index;
            let mut spaces = 0;
            while scan < span.len() && bytes[scan] == b' ' && spaces < 3 {
                scan += 1;
                spaces += 1;
            }
            if let Some(end) = scan_esm(span, scan) {
                ranges.push((scan, end));
                index = end;
                at_line_start = bytes.get(index) == Some(&b'\n');
                continue;
            }
        }
        if bytes[index] == b'\n' {
            at_line_start = true;
            index += 1;
        } else {
            at_line_start = false;
            index += char_len(bytes[index]);
        }
    }
    ranges
}

/// Keywords that may follow `export` to form an export declaration.
const EXPORT_DECLARATIONS: &[&str] = &[
    "default",
    "const",
    "let",
    "var",
    "function",
    "class",
    "async",
    "type",
    "interface",
    "enum",
    "abstract",
    "declare",
    "namespace",
    "module",
];

/// Whether `byte` can appear inside a JavaScript identifier.
fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$'
}

/// Read the identifier word starting at `pos` (empty if `pos` is not an
/// identifier start).
fn read_word(source: &str, pos: usize) -> &str {
    let bytes = source.as_bytes();
    let mut end = pos;
    while end < source.len() && is_ident_byte(bytes[end]) {
        end += 1;
    }
    &source[pos..end]
}

/// Whether `word` occurs at byte offset `at` as a standalone token (bounded by
/// non-identifier bytes on both sides).
fn is_word_at(bytes: &[u8], at: usize, word: &[u8]) -> bool {
    if at + word.len() > bytes.len() || &bytes[at..at + word.len()] != word {
        return false;
    }
    let before_ok = at == 0 || !is_ident_byte(bytes[at - 1]);
    let after_ok = at + word.len() >= bytes.len() || !is_ident_byte(bytes[at + word.len()]);
    before_ok && after_ok
}

/// If an ESM `import`/`export` statement starts at `start` within `span`, return
/// the byte offset just past its end (before the terminating newline).
///
/// The statement is consumed across the lines of the span while bracket depth is
/// positive, so multi-line imports and `export const … = {…}` objects are
/// handled.  Scanning is bounded to the span: an object whose body contains a
/// blank line is split by comrak into separate paragraphs and therefore is not
/// fully contained in any one span, so it is left unprotected (a documented
/// limitation) rather than scanning into unrelated later content.  String
/// literals, line comments (`//`), and block comments (`/* … */`) are skipped so
/// that `{`/`}`/`<`/`>` inside them do not affect the brace-depth scan.
///
/// To avoid protecting ordinary prose that merely begins with the word
/// "import"/"export", the statement form is validated: `export` must be followed
/// by `{`, `*`, or a declaration keyword (`const`, `default`, `function`, …);
/// `import` must be a side-effect string import or carry a `from` clause whose
/// specifier is a string.  Returns `None` when the form is not valid ESM, or when
/// the brackets are still unbalanced at the end of the span (do-no-harm: leave
/// ambiguous input unprotected rather than guess).
fn scan_esm(span: &str, start: usize) -> Option<usize> {
    let bytes = span.as_bytes();
    let rest = &span[start..];
    let is_import = rest.starts_with("import");
    if !is_import && !rest.starts_with("export") {
        return None;
    }
    let after = start + "import".len();
    // The character after the keyword must be plausible ESM, not the tail of an
    // identifier ("important", "exports") or sentence punctuation ("import.").
    match bytes.get(after) {
        Some(b' ' | b'\t' | b'{' | b'*' | b'(' | b'"' | b'\'') => {}
        _ => return None,
    }
    // The first meaningful token after the keyword (skipping spaces/tabs).
    let mut token = after;
    while token < span.len() && matches!(bytes[token], b' ' | b'\t') {
        token += 1;
    }
    if token >= span.len() || bytes[token] == b'\n' {
        // Bare keyword line ("import"/"export" with nothing after it) is prose.
        return None;
    }
    let first = bytes[token];

    // Validate the statement form and decide whether a `from` clause is required.
    let mut needs_from = false;
    if is_import {
        match first {
            b'"' | b'\'' => {}      // side-effect import: `import "module"`
            b'(' => return None,    // dynamic `import(...)` is an expression, not flow ESM
            _ => needs_from = true, // named / default / namespace import
        }
    } else if first != b'{' && first != b'*' {
        // `export <declaration>`: the word must be a real declaration keyword.
        if !EXPORT_DECLARATIONS.contains(&read_word(span, token)) {
            return None;
        }
    }

    let mut index = after;
    let mut depth: i32 = 0;
    let mut saw_from = false;
    while index < span.len() {
        match bytes[index] {
            b'"' | b'\'' | b'`' => {
                index = skip_string(span, index)?;
            }
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                // Line comment: skip to (but not past) the end of the line so the
                // newline arm still sees it for the depth check.
                index += 2;
                while index < span.len() && bytes[index] != b'\n' {
                    index += char_len(bytes[index]);
                }
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                // Block comment: skip to the closing `*/`.
                index += 2;
                while index < span.len()
                    && !(bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/'))
                {
                    index += char_len(bytes[index]);
                }
                if index < span.len() {
                    index += 2;
                }
            }
            b'{' | b'(' | b'[' => {
                depth += 1;
                index += 1;
            }
            b'}' | b')' | b']' => {
                depth -= 1;
                index += 1;
            }
            b'\n' => {
                if depth <= 0 {
                    return finish_esm(index, needs_from, saw_from);
                }
                index += 1;
            }
            b'f' if needs_from && !saw_from && is_word_at(bytes, index, b"from") => {
                // A genuine `from` clause is followed by the module string
                // specifier, e.g. `from "./x"`.  Requiring the string rejects
                // ordinary prose such as "import these ideas from elsewhere."
                let mut probe = index + 4;
                while probe < span.len() && matches!(bytes[probe], b' ' | b'\t') {
                    probe += 1;
                }
                if matches!(bytes.get(probe), Some(b'"' | b'\'' | b'`')) {
                    saw_from = true;
                }
                index += 4;
            }
            _ => index += char_len(bytes[index]),
        }
    }
    // Reached the end of the span: complete only if brackets balanced out.
    if depth <= 0 {
        finish_esm(span.len(), needs_from, saw_from)
    } else {
        None
    }
}

/// Resolve the end of an ESM statement: a statement that requires a `from`
/// clause must have seen one to count as ESM.
fn finish_esm(end: usize, needs_from: bool, saw_from: bool) -> Option<usize> {
    if needs_from && !saw_from {
        None
    } else {
        Some(end)
    }
}

/// Skip a string literal that starts at `start` (a `"`, `'`, or `` ` `` quote).
/// Returns the byte offset just past the closing quote, or `None` if the string
/// is unterminated.
fn skip_string(span: &str, start: usize) -> Option<usize> {
    let bytes = span.as_bytes();
    let quote = bytes[start];
    let mut index = start + 1;
    while index < span.len() {
        match bytes[index] {
            b'\\' => {
                // Skip the backslash and the (possibly multi-byte) escaped char.
                index += 1;
                if index < span.len() {
                    index += char_len(bytes[index]);
                }
            }
            byte if byte == quote => return Some(index + 1),
            byte => index += char_len(byte),
        }
    }
    None
}

/// UTF-8 length in bytes of the character whose leading byte is `byte`.
fn char_len(byte: u8) -> usize {
    if byte < 0x80 {
        1
    } else if byte >> 5 == 0b110 {
        2
    } else if byte >> 4 == 0b1110 {
        3
    } else if byte >> 3 == 0b11110 {
        4
    } else {
        1
    }
}

/// Choose a placeholder nonce that does not collide with existing text in
/// `source`.
fn choose_nonce(source: &str) -> String {
    let mut nonce = String::from("x");
    while source.contains(&format!("hongdown-mdx:{nonce}:")) {
        nonce.push('x');
    }
    nonce
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comrak_options() -> ComrakOptions<'static> {
        let mut options = ComrakOptions::default();
        options.extension.front_matter_delimiter = Some("---".to_string());
        options.extension.table = true;
        options.extension.description_lists = true;
        options.extension.alerts = true;
        options.extension.footnotes = true;
        options.extension.tasklist = true;
        options.extension.math_dollars = true;
        options
    }

    /// Protect, then assert the round trip restores the original verbatim text
    /// somewhere in the protected source's replacement map.
    fn protect_source(source: &str) -> Option<(String, Vec<Protected>)> {
        protect(source, &comrak_options())
    }

    #[test]
    fn scan_esm_single_line_import() {
        let span = "import { Chart } from \"./chart.js\";";
        assert_eq!(scan_esm(span, 0), Some(span.len()));
    }

    #[test]
    fn scan_esm_rejects_prose_words() {
        assert_eq!(scan_esm("important things happen", 0), None);
        assert_eq!(scan_esm("exporting is fun", 0), None);
    }

    #[test]
    fn scan_esm_multiline_import() {
        let span = "import {\n  a,\n  b,\n} from \"y\";";
        // Consumes all lines while the brace is open, ending at the final `;`.
        assert_eq!(scan_esm(span, 0), Some(span.len()));
    }

    #[test]
    fn scan_esm_export_with_object() {
        let span = "export const meta = { author: 'Hong Minhee' };";
        assert_eq!(scan_esm(span, 0), Some(span.len()));
    }

    #[test]
    fn scan_esm_string_with_angle_and_brace() {
        // `>` and `{` inside the string must not derail the scan.
        let span = "import x from \"a > b { c\";";
        assert_eq!(scan_esm(span, 0), Some(span.len()));
    }

    #[test]
    fn scan_esm_rejects_bare_keyword_line() {
        // A line that is just the word "export"/"import" is prose, not ESM.
        assert_eq!(scan_esm("export", 0), None);
        assert_eq!(scan_esm("import", 0), None);
        assert_eq!(scan_esm("export   ", 0), None);
    }

    #[test]
    fn scan_esm_skips_brace_in_line_comment() {
        // A `}` inside a `//` comment must not close the object early.
        let span = "export const meta = {\n  // }\n  title: \"Hi\",\n};";
        assert_eq!(scan_esm(span, 0), Some(span.len()));
    }

    #[test]
    fn scan_esm_skips_brace_in_block_comment() {
        let span = "export const meta = {\n  /* } */\n  title: \"Hi\",\n};";
        assert_eq!(scan_esm(span, 0), Some(span.len()));
    }

    #[test]
    fn scan_esm_unbalanced_braces_returns_none() {
        // An import whose brace never closes within the span is ambiguous; leave
        // it unprotected rather than swallow following content.
        let span = "import { a from \"x\"";
        assert_eq!(scan_esm(span, 0), None);
    }

    #[test]
    fn scan_esm_side_effect_import() {
        let span = "import \"./styles.css\";";
        assert_eq!(scan_esm(span, 0), Some(span.len()));
    }

    #[test]
    fn scan_esm_rejects_prose_starting_with_export_word() {
        // "export these results from the team" is prose, not an export.
        assert_eq!(scan_esm("export these results from the team.", 0), None);
    }

    #[test]
    fn scan_esm_rejects_prose_import_without_string_specifier() {
        // "import these ideas from elsewhere." has a `from` not followed by a
        // module string, so it is prose, not an import.
        assert_eq!(scan_esm("import these ideas from elsewhere.", 0), None);
    }

    #[test]
    fn scan_esm_default_import_with_string_from() {
        let span = "import Chart from \"./chart.js\";";
        assert_eq!(scan_esm(span, 0), Some(span.len()));
    }

    #[test]
    fn scan_esm_unterminated_object_in_span_returns_none() {
        // A span that ends mid-object (open brace) is not a complete statement.
        // This is the blank-line-split case: comrak ends the paragraph here, so
        // the object is left unprotected rather than scanning into later prose.
        let span = "export const meta = {\n  a: 1,";
        assert_eq!(scan_esm(span, 0), None);
    }

    #[test]
    fn protect_does_not_swallow_prose_after_unterminated_object() {
        // comrak splits this on the blank line; the first paragraph ends
        // mid-object (depth > 0) so it is not protected, and the prose paragraph
        // — including its stray `}` — is left to normal formatting.
        let source = "export const meta = {\n\nProse with } here.\n";
        assert!(protect_source(source).is_none());
    }

    #[test]
    fn protect_skips_prose_starting_with_keyword() {
        // A paragraph that merely begins with "export"/"import" as prose is not
        // protected.
        assert!(protect_source("export these notes from class today.\n").is_none());
        assert!(protect_source("import more discipline into your life.\n").is_none());
    }

    #[test]
    fn skip_string_handles_escapes() {
        let span = r#""a\"b""#; // "a\"b"
        assert_eq!(skip_string(span, 0), Some(span.len()));
    }

    #[test]
    fn protect_returns_none_without_constructs() {
        assert!(protect_source("Just some prose.\n\nMore prose.\n").is_none());
    }

    #[test]
    fn protect_records_import_verbatim() {
        let source = "import { Chart } from \"./chart.js\";\n\nProse.\n";
        let (protected, map) = protect_source(source).expect("should protect");
        assert_eq!(map.len(), 1);
        assert_eq!(map[0].original, "import { Chart } from \"./chart.js\";");
        assert!(protected.contains("<!--hongdown-mdx:"));
        assert!(protected.contains("Prose."));
        assert!(!protected.contains("import {"));
    }

    #[test]
    fn restore_round_trips() {
        let source = "export const meta = { author: 'Hong Minhee' };\n";
        let (protected, map) = protect_source(source).expect("should protect");
        let restored = restore(&protected, &map);
        assert!(restored.contains("export const meta = { author: 'Hong Minhee' };"));
    }

    #[test]
    fn protect_skips_code_blocks() {
        // An import inside a fenced code block is not paragraph text, so it must
        // not be protected.
        let source = "~~~~ js\nimport x from \"y\";\n~~~~\n";
        assert!(protect_source(source).is_none());
    }
}
