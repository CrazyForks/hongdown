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
//! surrounding Markdown prose is formatted normally.  [`Protection::restore`]
//! then swaps the placeholders back for their original text.
//!
//! The detection is AST-guided: only the source spans of `Paragraph` nodes are
//! scanned.  Content inside fenced/indented code blocks, inline code spans,
//! math, front matter, headings, and HTML blocks never becomes paragraph text
//! in comrak's AST, so it is naturally left untouched — we only ever touch what
//! comrak would otherwise corrupt.

use comrak::nodes::{NodeValue, Sourcepos};
use comrak::{Arena, Options as ComrakOptions, parse_document};

/// A protected region: the placeholder token that stands in for an MDX
/// construct during formatting, and the original verbatim text to restore.
pub(crate) struct Protected {
    token: String,
    original: String,
}

/// The result of [`protect`]: the source to format (with MDX constructs replaced
/// by placeholders), the replacement map, and a line map from the protected
/// source back to the original.
pub(crate) struct Protection {
    /// The protected source to hand to comrak.
    pub source: String,
    /// The placeholder → original replacements.
    replacements: Vec<Protected>,
    /// For each protected line `n` (1-indexed), the original line number it came
    /// from.  Multi-line constructs collapse to a single-line placeholder, so
    /// without this a warning after one would be reported several lines early.
    original_lines: Vec<usize>,
}

impl Protection {
    /// Replace every placeholder token in `output` with its original text.
    pub(crate) fn restore(&self, output: &str) -> String {
        let mut result = output.to_string();
        for protected in &self.replacements {
            result = result.replace(&protected.token, &protected.original);
        }
        result
    }

    /// Map a 1-indexed line number in the protected source back to the original
    /// source.
    ///
    /// This is exact for block-level constructs (each protected line corresponds
    /// to one original line).  For the rare case of a multi-line construct
    /// embedded inline within a paragraph, lines after it on the *same* protected
    /// line map to the line's start, which can be a few lines early — a minor
    /// imprecision in warning positions, not in the formatted output.
    pub(crate) fn original_line(&self, protected_line: usize) -> usize {
        if protected_line == 0 {
            return protected_line;
        }
        self.original_lines
            .get(protected_line - 1)
            .copied()
            .unwrap_or(protected_line)
    }
}

/// Detect MDX constructs in `source` and replace each with a unique HTML-comment
/// placeholder, returning a [`Protection`].
///
/// Returns `None` when there is nothing to protect (the caller then formats the
/// original source normally).  This is also the do-no-harm fallback: any
/// construct that cannot be delimited confidently is simply left unprotected
/// rather than guessed at.
pub(crate) fn protect(source: &str, comrak_options: &ComrakOptions) -> Option<Protection> {
    let arena = Arena::new();
    let root = parse_document(&arena, source, comrak_options);
    let line_starts = build_line_starts(source);

    // Collect absolute byte ranges of every construct found in a scanned span.
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for node in root.descendants() {
        let (is_target, sourcepos) = {
            let data = node.data.borrow();
            // Only paragraphs are scanned.  Headings are skipped: their inline
            // serialization (sentence case, width measurement, anchor handling)
            // does not preserve placeholder comments, and skipping them also
            // keeps explicit `{#anchor}` identifiers from being misdetected.
            let is_target = matches!(data.value, NodeValue::Paragraph);
            (is_target, data.sourcepos)
        };
        if !is_target {
            continue;
        }
        let Some((start, end)) = span_bytes(source, &line_starts, sourcepos) else {
            continue;
        };
        let span = &source[start..end];

        // Some inline constructs are owned by comrak's own syntax and must not be
        // scanned: code spans and math are emitted verbatim; links/images have
        // their own `[…](…)` bracket/paren structure (a `{…}` in link text would
        // otherwise be protected, desyncing it from its reference definition);
        // and a single-line inline HTML tag (which comrak preserves) can carry
        // `{…}` inside a quoted attribute.  A *multi-line* inline HTML tag is NOT
        // skipped — comrak does not round-trip it cleanly, so it is left for the
        // JSX scanner to protect.  Collect the skip ranges (span-local).
        let mut skips: Vec<(usize, usize)> = Vec::new();
        for inner in node.descendants() {
            let (is_owned, inner_sourcepos) = {
                let data = inner.data.borrow();
                let pos = data.sourcepos;
                let is_owned = match data.value {
                    NodeValue::Code(_)
                    | NodeValue::Math(_)
                    | NodeValue::Link(_)
                    | NodeValue::Image(_) => true,
                    NodeValue::HtmlInline(_) => pos.start.line == pos.end.line,
                    _ => false,
                };
                (is_owned, pos)
            };
            if !is_owned {
                continue;
            }
            if let Some((inner_start, inner_end)) =
                span_bytes(source, &line_starts, inner_sourcepos)
                && inner_start >= start
                && inner_end <= end
            {
                skips.push((inner_start - start, inner_end - start));
            }
        }
        skips.sort_by_key(|&(skip_start, _)| skip_start);

        for (relative_start, relative_end) in find_constructs(span, &skips) {
            ranges.push((start + relative_start, start + relative_end));
        }
    }

    if ranges.is_empty() {
        return None;
    }

    ranges.sort_by_key(|&(start, _)| start);

    let nonce = choose_nonce(source);
    let mut protected = String::with_capacity(source.len());
    let mut replacements = Vec::with_capacity(ranges.len());
    // Line map: `original_lines[n - 1]` is the original line for protected line n.
    let mut original_lines = vec![1usize];
    let mut original_line = 1usize;
    let mut cursor = 0;
    for (index, &(start, end)) in ranges.iter().enumerate() {
        // Defensively skip any range that overlaps an already-protected one.
        if start < cursor {
            continue;
        }
        // Copy the prefix verbatim, advancing the line map one entry per newline.
        let prefix = &source[cursor..start];
        protected.push_str(prefix);
        for _ in 0..count_newlines(prefix) {
            original_line += 1;
            original_lines.push(original_line);
        }
        // The placeholder (no newlines) stays on the current protected line, but
        // the construct it replaces may span several original lines; account for
        // those so later lines map back correctly.
        let token = format!("<!--hongdown-mdx:{nonce}:{index}-->");
        protected.push_str(&token);
        original_line += count_newlines(&source[start..end]);
        replacements.push(Protected {
            token,
            original: source[start..end].to_string(),
        });
        cursor = end;
    }
    let tail = &source[cursor..];
    protected.push_str(tail);
    for _ in 0..count_newlines(tail) {
        original_line += 1;
        original_lines.push(original_line);
    }

    Some(Protection {
        source: protected,
        replacements,
        original_lines,
    })
}

/// Count the `\n` bytes in `text`.
fn count_newlines(text: &str) -> usize {
    text.bytes().filter(|&b| b == b'\n').count()
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
/// Detects ESM `import`/`export` statements (recognized at line starts), JSX
/// elements/fragments, and `{…}` expressions (recognized anywhere).
///
/// `skips` lists span-local ranges of inline code spans and math that comrak
/// already preserves verbatim; their contents are never scanned, so the
/// `{`/`<`/`}` they contain cannot be mistaken for MDX constructs.  `skips` must
/// be sorted by start offset.
fn find_constructs(span: &str, skips: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let bytes = span.as_bytes();
    let mut ranges = Vec::new();
    let mut index = 0;
    let mut at_line_start = true;
    while index < span.len() {
        // Jump over inline code/math that comrak emits verbatim.
        if let Some(skip_end) = skip_range_end(skips, index) {
            index = skip_end;
            at_line_start = bytes.get(index) == Some(&b'\n');
            continue;
        }
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
        // JSX elements and fragments can appear anywhere (flow or inline).
        if bytes[index] == b'<' {
            match scan_jsx(span, index) {
                JsxScan::Matched(end) => {
                    ranges.push((index, end));
                    index = end;
                    at_line_start = bytes.get(index) == Some(&b'\n');
                    continue;
                }
                JsxScan::GiveUp => break,
                JsxScan::NoMatch => {}
            }
        }
        // Bare `{…}` expressions can appear anywhere (flow or inline).
        if bytes[index] == b'{' {
            match scan_expression(span, index) {
                ExprScan::Matched(end) => {
                    ranges.push((index, end));
                    index = end;
                    at_line_start = bytes.get(index) == Some(&b'\n');
                    continue;
                }
                ExprScan::GiveUp => {
                    // Leave the rest of this span to comrak so no nested `{…}` is
                    // protected in isolation.
                    break;
                }
                ExprScan::NoMatch => {}
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

/// If `index` falls within one of the sorted, non-overlapping `skips` ranges,
/// return that range's end offset.
fn skip_range_end(skips: &[(usize, usize)], index: usize) -> Option<usize> {
    for &(start, end) in skips {
        if start > index {
            break;
        }
        if index < end {
            return Some(end);
        }
    }
    None
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
    let after = start
        + if is_import {
            "import".len()
        } else {
            "export".len()
        };
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
    } else if first == b'*' {
        // `export * …` is always a re-export and requires a `from` clause, so
        // prose such as "export *all* the things" is not mistaken for ESM.
        needs_from = true;
    } else if first != b'{' {
        // `export <declaration>`: the word must be a real declaration keyword.
        if !EXPORT_DECLARATIONS.contains(&read_word(span, token)) {
            return None;
        }
    }

    const VALUE: u8 = b'a';
    let mut index = after;
    let mut depth: i32 = 0;
    let mut saw_from = false;
    // The most recent significant token, used (as in `skip_braces`) to tell a
    // regex literal from a division operator.
    let mut prev: Option<u8> = None;
    let mut prev_word: Option<&str> = None;
    while index < span.len() {
        let byte = bytes[index];
        if is_ident_byte(byte) {
            let after_member_access = prev == Some(b'.');
            let word_start = index;
            while index < span.len() && is_ident_byte(bytes[index]) {
                index += 1;
            }
            let word = &span[word_start..index];
            // A genuine `from` clause is followed by the module string specifier,
            // e.g. `from "./x"` (which may sit on the next line).  Requiring the
            // string rejects prose such as "import these ideas from elsewhere."
            if needs_from && !saw_from && !after_member_access && word == "from" {
                let mut probe = index;
                while probe < span.len() && matches!(bytes[probe], b' ' | b'\t' | b'\n' | b'\r') {
                    probe += 1;
                }
                if matches!(bytes.get(probe), Some(b'"' | b'\'' | b'`')) {
                    saw_from = true;
                    // Jump to the specifier so an intervening newline is not
                    // mistaken for the end of the statement.
                    index = probe;
                    prev = Some(VALUE);
                    prev_word = None;
                    continue;
                }
            }
            prev = Some(VALUE);
            prev_word = if after_member_access {
                None
            } else {
                Some(word)
            };
            continue;
        }
        match byte {
            b'"' | b'\'' | b'`' => {
                index = skip_string(span, index)?;
                prev = Some(VALUE);
                prev_word = None;
            }
            b'/' => {
                if let Some(after) = skip_comment(span, index) {
                    index = after; // comments do not change the preceding token
                } else if prev == Some(b'}') {
                    // Ambiguous regex-or-division after `}`; do not guess.
                    return None;
                } else if allows_regex(prev, prev_word) {
                    index = skip_regex(span, index).unwrap_or(index + 1);
                    prev = Some(VALUE);
                    prev_word = None;
                } else {
                    index += 1;
                    prev = Some(b'/');
                    prev_word = None;
                }
            }
            b'{' | b'(' | b'[' => {
                depth += 1;
                index += 1;
                prev = Some(byte);
                prev_word = None;
            }
            b'}' | b')' | b']' => {
                depth -= 1;
                index += 1;
                prev = Some(byte);
                prev_word = None;
            }
            b'\n' => {
                if depth <= 0 {
                    return finish_esm(index, needs_from, saw_from);
                }
                index += 1; // inside brackets: leave the preceding token unchanged
            }
            b' ' | b'\t' | b'\r' => index += 1, // whitespace: leave prev unchanged
            other => {
                index += char_len(other);
                prev = Some(other);
                prev_word = None;
            }
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

/// If a `//` line comment or `/* … */` block comment starts at `start`, return
/// the byte offset just past it.  A line comment stops at — but does not consume
/// — the terminating newline (a line comment that runs to the end of the span is
/// complete).  Returns `None` when no comment starts there, or when a block
/// comment is left unterminated (no closing `*/`).
fn skip_comment(span: &str, start: usize) -> Option<usize> {
    let bytes = span.as_bytes();
    match (bytes.get(start), bytes.get(start + 1)) {
        (Some(b'/'), Some(b'/')) => {
            let mut index = start + 2;
            while index < span.len() && bytes[index] != b'\n' {
                index += char_len(bytes[index]);
            }
            Some(index)
        }
        (Some(b'/'), Some(b'*')) => {
            let mut index = start + 2;
            while index < span.len()
                && !(bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/'))
            {
                index += char_len(bytes[index]);
            }
            if index < span.len() {
                Some(index + 2) // consume the closing `*/`
            } else {
                None // unterminated block comment
            }
        }
        _ => None,
    }
}

/// Whether `byte` can appear in a JSX tag or component name (after the first
/// letter): identifiers, member access (`Foo.Bar`), and hyphenated custom
/// element names.
fn is_tag_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'$')
}

/// Whether `byte` can start a JSX tag or component name.  Like a JavaScript
/// identifier, a name may begin with a letter, `_`, or `$`.
fn is_tag_name_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || matches!(byte, b'_' | b'$')
}

/// A parsed JSX opening tag (or fragment opener `<>`).
struct OpenTag<'a> {
    /// Byte offset just past the tag's closing `>` or `/>`.
    end: usize,
    /// The tag/component name (empty for a fragment).
    name: &'a str,
    /// Whether the tag is self-closing (`/>`).
    self_closing: bool,
    /// Whether this is a fragment opener `<>`.
    is_fragment: bool,
    /// Whether the tag carries a `{…}` expression attribute.
    has_expression: bool,
    /// Whether the tag spans more than one line.
    multiline: bool,
}

/// Outcome of scanning a JSX opening tag.
enum OpenTagScan<'a> {
    /// A well-formed opening tag (or fragment opener).
    Parsed(OpenTag<'a>),
    /// A protect-worthy tag (it carries a `{…}` attribute or spans lines) that
    /// could not be delimited; the caller should give up on the whole construct.
    GiveUp,
    /// Not a protect-worthy opening tag here.
    NoMatch,
}

/// Parse a JSX opening tag (or fragment `<>`) starting at `start` (a `<`).
/// String literals and `{…}` expression attributes are skipped so that
/// `>`/`{`/`}` inside them do not terminate the tag.  Returns [`OpenTagScan`]:
/// once the tag is seen to be protect-worthy (a `{…}` attribute or a newline)
/// but cannot be completed, the result is `GiveUp` rather than `NoMatch`, so the
/// caller never partially scans inside it.
fn scan_open_tag(span: &str, start: usize) -> OpenTagScan<'_> {
    let bytes = span.as_bytes();
    // Fragment opener `<>`.
    if bytes.get(start + 1) == Some(&b'>') {
        return OpenTagScan::Parsed(OpenTag {
            end: start + 2,
            name: "",
            self_closing: false,
            is_fragment: true,
            has_expression: false,
            multiline: false,
        });
    }
    // A tag name must begin with a letter, `_`, or `$`.
    if !bytes.get(start + 1).is_some_and(|&b| is_tag_name_start(b)) {
        return OpenTagScan::NoMatch;
    }
    let name_start = start + 1;
    let mut index = name_start;
    while index < span.len() && is_tag_name_byte(bytes[index]) {
        index += 1;
    }
    let name = &span[name_start..index];

    let mut has_expression = false;
    let mut multiline = false;
    // Whether failure to complete should give up (protect-worthy) or no-match.
    let give_up_on_failure = |has_expression: bool, multiline: bool| {
        if has_expression || multiline {
            OpenTagScan::GiveUp
        } else {
            OpenTagScan::NoMatch
        }
    };
    while index < span.len() {
        match bytes[index] {
            b'>' => {
                return OpenTagScan::Parsed(OpenTag {
                    end: index + 1,
                    name,
                    self_closing: false,
                    is_fragment: false,
                    has_expression,
                    multiline,
                });
            }
            b'/' if bytes.get(index + 1) == Some(&b'>') => {
                return OpenTagScan::Parsed(OpenTag {
                    end: index + 2,
                    name,
                    self_closing: true,
                    is_fragment: false,
                    has_expression,
                    multiline,
                });
            }
            b'{' => {
                has_expression = true;
                match skip_braces(span, index) {
                    BraceScan::Closed(after) => index = after,
                    _ => return OpenTagScan::GiveUp,
                }
            }
            b'"' | b'\'' | b'`' => match skip_string(span, index) {
                Some(after) => index = after,
                None => return give_up_on_failure(has_expression, multiline),
            },
            b'\n' => {
                multiline = true;
                index += 1;
            }
            byte => index += char_len(byte),
        }
    }
    give_up_on_failure(has_expression, multiline)
}

/// Read a JSX closing tag `</name>` (or fragment close `</>`) starting at `start`
/// (a `<` followed by `/`).  Returns the tag name (empty for a fragment) and the
/// byte offset just past the `>`.
fn read_close_tag(span: &str, start: usize) -> Option<(&str, usize)> {
    let bytes = span.as_bytes();
    let mut index = start + 2;
    let name_start = index;
    while index < span.len() && is_tag_name_byte(bytes[index]) {
        index += 1;
    }
    let name = &span[name_start..index];
    while index < span.len() && matches!(bytes[index], b' ' | b'\t' | b'\n') {
        index += 1;
    }
    if bytes.get(index) != Some(&b'>') {
        return None;
    }
    Some((name, index + 1))
}

/// Outcome of scanning a `{…}` region.
enum BraceScan {
    /// The braces balanced; the value is the byte offset just past the `}`.
    Closed(usize),
    /// A `/` after `}` made regex-vs-division undecidable; give up on the region.
    Ambiguous,
    /// The braces never balanced within the span.
    Unbalanced,
}

/// Skip a balanced `{…}` expression starting at `start` (a `{`).  String
/// literals, comments, and regex literals inside are skipped so their braces do
/// not affect nesting.  Returns [`BraceScan::Closed`] with the byte offset just
/// past the matching `}`, [`BraceScan::Ambiguous`] when a `/` after `}` cannot be
/// classified, or [`BraceScan::Unbalanced`] when the braces never balance.
fn skip_braces(span: &str, start: usize) -> BraceScan {
    let bytes = span.as_bytes();
    let mut depth = 0i32;
    let mut index = start;
    // The most recent significant token, used to tell a regex literal from a
    // division operator: `prev` is the last significant byte (`VALUE` stands for
    // identifiers/numbers/strings/regexes), and `prev_word` is the last
    // identifier word (so regex-position keywords like `return` are recognized).
    const VALUE: u8 = b'a';
    let mut prev: Option<u8> = None;
    let mut prev_word: Option<&str> = None;
    while index < span.len() {
        let byte = bytes[index];
        if is_ident_byte(byte) {
            // Consume a whole identifier/number word as one token.  A word right
            // after `.` (or `?.`) is a property name, not a keyword, so it must
            // not enable regex position (e.g. `obj.return / …`).
            let after_member_access = prev == Some(b'.');
            let word_start = index;
            while index < span.len() && is_ident_byte(bytes[index]) {
                index += 1;
            }
            prev = Some(VALUE);
            prev_word = if after_member_access {
                None
            } else {
                Some(&span[word_start..index])
            };
            continue;
        }
        match byte {
            b'{' => {
                depth += 1;
                index += 1;
                prev = Some(b'{');
                prev_word = None;
            }
            b'}' => {
                depth -= 1;
                index += 1;
                if depth == 0 {
                    return BraceScan::Closed(index);
                }
                prev = Some(b'}');
                prev_word = None;
            }
            b'"' | b'\'' | b'`' => {
                let Some(after) = skip_string(span, index) else {
                    return BraceScan::Unbalanced;
                };
                index = after;
                prev = Some(VALUE); // a string is a value: a following `/` is division
                prev_word = None;
            }
            b'/' => {
                if let Some(after) = skip_comment(span, index) {
                    index = after; // comments do not change the preceding token
                } else if prev == Some(b'}') {
                    // A `/` right after `}` is genuinely ambiguous without a JS
                    // parser: regex after a block close, or division after an
                    // object literal.  Rather than guess (and risk miscounting a
                    // `}` inside a regex character class), give up and leave the
                    // whole expression to comrak.
                    return BraceScan::Ambiguous;
                } else if allows_regex(prev, prev_word) {
                    if let Some(after) = skip_regex(span, index) {
                        index = after;
                        prev = Some(VALUE); // a regex is a value: a following `/` is division
                    } else {
                        index += 1;
                        prev = Some(b'/');
                    }
                    prev_word = None;
                } else {
                    // Division.
                    index += 1;
                    prev = Some(b'/');
                    prev_word = None;
                }
            }
            b' ' | b'\t' | b'\n' | b'\r' => index += 1,
            other => {
                prev = Some(other);
                prev_word = None;
                index += char_len(other);
            }
        }
    }
    BraceScan::Unbalanced
}

/// Keywords after which a `/` begins a regex literal (expression position).
fn is_regex_keyword(word: &str) -> bool {
    matches!(
        word,
        "return"
            | "throw"
            | "yield"
            | "await"
            | "typeof"
            | "void"
            | "delete"
            | "in"
            | "of"
            | "instanceof"
            | "new"
            | "do"
            | "case"
            | "else"
    )
}

/// Whether a `/` following the most recent token begins a regex literal (rather
/// than a division operator).  A regex appears in "expression position": at the
/// start, after an operator or opening bracket, or after a regex-position
/// keyword such as `return`.  A `/` after a value (identifier, number, string,
/// `)`, `]`) is division.  The genuinely ambiguous `/`-after-`}` case is handled
/// separately by the caller (which gives up rather than guess).
///
/// This is a heuristic; a misclassified division is only attempted as a regex,
/// and if no valid regex terminator follows, [`skip_regex`] returns `None` and
/// the caller falls back to treating the `/` as division.
fn allows_regex(prev: Option<u8>, prev_word: Option<&str>) -> bool {
    match prev {
        None => true,
        // A value token: only a regex-position keyword allows a regex.
        Some(b'a') => prev_word.is_some_and(is_regex_keyword),
        Some(byte) => matches!(
            byte,
            b'(' | b','
                | b'='
                | b'['
                | b'{'
                | b':'
                | b';'
                | b'!'
                | b'&'
                | b'|'
                | b'?'
                | b'+'
                | b'-'
                | b'*'
                | b'%'
                | b'^'
                | b'~'
                | b'<'
                | b'>'
                | b'/'
        ),
    }
}

/// Skip a JavaScript regex literal starting at `start` (a `/`).  Returns the byte
/// offset just past the closing `/` and any flags.  A `/` inside a `[…]`
/// character class does not close the regex.  Returns `None` if the literal does
/// not terminate on its line (regex literals cannot span lines).
fn skip_regex(span: &str, start: usize) -> Option<usize> {
    let bytes = span.as_bytes();
    let mut index = start + 1;
    let mut in_class = false;
    while index < span.len() {
        match bytes[index] {
            b'\\' => {
                index += 1;
                if index < span.len() {
                    index += char_len(bytes[index]);
                }
            }
            b'[' => {
                in_class = true;
                index += 1;
            }
            b']' => {
                in_class = false;
                index += 1;
            }
            b'/' if !in_class => {
                index += 1;
                while index < span.len() && bytes[index].is_ascii_alphabetic() {
                    index += 1;
                }
                return Some(index);
            }
            b'\n' => return None,
            byte => index += char_len(byte),
        }
    }
    None
}

/// Outcome of scanning for a JSX element or fragment at a given `<`.
#[derive(Debug, PartialEq, Eq)]
enum JsxScan {
    /// A protectable construct ending just past the byte offset.
    Matched(usize),
    /// A protect-worthy construct that could not be delimited; skip the rest of
    /// the span rather than risk protecting part of it.
    GiveUp,
    /// Not a protectable construct here (advance one character and continue).
    NoMatch,
}

/// Classify a JSX element or fragment starting at `start` (a `<`).
///
/// A construct is protected only when comrak misparses it: a fragment (`<>…</>`),
/// or an element whose opening tag carries a `{…}` expression attribute or spans
/// multiple lines.  A plain single-line HTML-shaped tag (e.g. `<Chart x="y" />`),
/// an autolink (`<https://…>`), or a stray `<` in prose is [`JsxScan::NoMatch`]
/// (comrak handles it, and the caller keeps scanning for nested constructs).
///
/// Container elements are protected as a whole (opening tag through the matching
/// closing tag), consistent with comrak's existing treatment of tag-based JSX;
/// the embedded JSX is preserved rather than reformatted.  String literals and
/// `{…}` expressions (which may themselves contain `<`/`>`) are skipped while
/// matching tags.  Once the root is known to be protect-worthy, any failure to
/// delimit it returns [`JsxScan::GiveUp`] so no nested fragment is protected on
/// its own.
fn scan_jsx(span: &str, start: usize) -> JsxScan {
    let bytes = span.as_bytes();
    // Must be able to begin a JSX construct: `<>` or `<` + tag name.
    match bytes.get(start + 1) {
        Some(b'>') => {}
        Some(&byte) if is_tag_name_start(byte) => {}
        _ => return JsxScan::NoMatch,
    }

    // The protection decision depends only on the *root* construct: comrak
    // corrupts a fragment, or a tag with a `{…}` attribute or multi-line opener.
    // A plain root tag (even a container) is preserved by comrak as-is, so it is
    // left alone — the caller then scans its children, where any nested
    // expression/JSX is protected independently and prose stays formatted.
    let root = match scan_open_tag(span, start) {
        OpenTagScan::Parsed(tag) => tag,
        OpenTagScan::GiveUp => return JsxScan::GiveUp,
        OpenTagScan::NoMatch => return JsxScan::NoMatch,
    };
    if !(root.is_fragment || root.has_expression || root.multiline) {
        return JsxScan::NoMatch;
    }
    if root.self_closing {
        return JsxScan::Matched(root.end);
    }

    // Protected container: scan to the matching closing tag, tracking nesting.
    // Nested tags' own attributes do not affect the (already decided) protection.
    // Any failure to delimit gives up, since the root is protect-worthy.
    let mut stack: Vec<&str> = vec![root.name];
    let mut index = root.end;
    while index < span.len() {
        match bytes[index] {
            b'<' if bytes.get(index + 1) == Some(&b'/') => {
                let Some((name, after)) = read_close_tag(span, index) else {
                    return JsxScan::GiveUp;
                };
                match stack.pop() {
                    Some(open) if open == name => {}
                    _ => return JsxScan::GiveUp, // mismatched or stray close tag
                }
                index = after;
                if stack.is_empty() {
                    return JsxScan::Matched(index);
                }
            }
            b'<' if matches!(bytes.get(index + 1), Some(b'>'))
                || bytes.get(index + 1).is_some_and(|&b| is_tag_name_start(b)) =>
            {
                match scan_open_tag(span, index) {
                    OpenTagScan::Parsed(tag) => {
                        index = tag.end;
                        if !tag.self_closing {
                            stack.push(tag.name);
                        }
                    }
                    _ => return JsxScan::GiveUp,
                }
            }
            b'<' => index += 1, // a literal `<` in element children
            b'{' => {
                // A `{…}` expression among element children.
                match skip_braces(span, index) {
                    BraceScan::Closed(after) => index = after,
                    _ => return JsxScan::GiveUp,
                }
            }
            byte => index += char_len(byte),
        }
    }
    JsxScan::GiveUp
}

/// Outcome of scanning for a bare `{…}` expression at a given `{`.
#[derive(Debug, PartialEq, Eq)]
enum ExprScan {
    /// A protectable expression ending just past the byte offset.
    Matched(usize),
    /// An ambiguous expression: skip the rest of the span rather than risk
    /// protecting part of it.
    GiveUp,
    /// Not a protectable expression here (advance one character and continue).
    NoMatch,
}

/// Classify a bare `{…}` JSX expression starting at `start` (a `{`).
///
/// A heading anchor `{#identifier}` (a Hongdown feature) and an unbalanced `{`
/// are [`ExprScan::NoMatch`].  A `{…}` whose internal `/`-after-`}` cannot be
/// classified is [`ExprScan::GiveUp`] (the caller then stops scanning the span so
/// no nested fragment is protected in isolation).
fn scan_expression(span: &str, start: usize) -> ExprScan {
    // `{#…}` at the end of a heading is an explicit anchor, not an expression.
    if span.as_bytes().get(start + 1) == Some(&b'#') {
        return ExprScan::NoMatch;
    }
    match skip_braces(span, start) {
        BraceScan::Closed(end) => ExprScan::Matched(end),
        BraceScan::Ambiguous => ExprScan::GiveUp,
        BraceScan::Unbalanced => ExprScan::NoMatch,
    }
}

/// Skip a string literal that starts at `start` (a `"`, `'`, or `` ` `` quote).
/// Returns the byte offset just past the closing quote, or `None` if the string
/// is unterminated.
///
/// In a template literal (`` ` ``), a `${…}` interpolation is skipped as a
/// balanced expression so that backticks or braces inside it (including nested
/// template literals) do not terminate the string early.
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
            b'$' if quote == b'`' && bytes.get(index + 1) == Some(&b'{') => {
                // Template literal interpolation `${ … }`.
                match skip_braces(span, index + 1) {
                    BraceScan::Closed(after) => index = after,
                    _ => return None,
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
    fn protect_source(source: &str) -> Option<Protection> {
        protect(source, &comrak_options())
    }

    /// The end offset of a matched bare expression at the start of `span`, or
    /// `None` for give-up/no-match.
    fn expr_end(span: &str) -> Option<usize> {
        match scan_expression(span, 0) {
            ExprScan::Matched(end) => Some(end),
            _ => None,
        }
    }

    /// The end offset of a matched JSX construct at the start of `span`, or `None`
    /// for give-up/no-match.
    fn jsx_end(span: &str) -> Option<usize> {
        match scan_jsx(span, 0) {
            JsxScan::Matched(end) => Some(end),
            _ => None,
        }
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
    fn scan_esm_skips_regex_with_brace() {
        // A `}` inside a regex character class must not be counted as a brace,
        // which would otherwise end a multi-line object early.
        let span = "export const config = {\n  re: /[}]/,\n  x: 1,\n};";
        assert_eq!(scan_esm(span, 0), Some(span.len()));
    }

    #[test]
    fn scan_esm_division_not_regex() {
        // `/` after a value is division, not a regex; the statement still ends
        // at its newline.
        let span = "export const ratio = a / b;";
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
    fn scan_esm_export_star_requires_from() {
        // `export *` is a re-export and always needs a `from` clause, so prose
        // using `*` for emphasis is not mistaken for ESM.
        assert_eq!(scan_esm("export *all* the things.", 0), None);
        let reexport = "export * from \"./mod.js\";";
        assert_eq!(scan_esm(reexport, 0), Some(reexport.len()));
        let reexport_ns = "export * as ns from \"./mod.js\";";
        assert_eq!(scan_esm(reexport_ns, 0), Some(reexport_ns.len()));
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
    fn scan_esm_from_clause_across_newline() {
        // The module specifier may sit on the next line after `from`; the
        // statement must continue across that newline.
        let span = "import Chart from\n  \"./chart.js\";";
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
    fn scan_jsx_self_closing_with_expression_attr() {
        let span = "<Chart data={data} />";
        assert_eq!(jsx_end(span), Some(span.len()));
    }

    #[test]
    fn scan_jsx_tag_name_starting_with_underscore_or_dollar() {
        // JSX component names may begin with `_` or `$`, like JS identifiers.
        let underscore = "<_Component data={x} />";
        assert_eq!(jsx_end(underscore), Some(underscore.len()));
        let dollar = "<$El data={x} />";
        assert_eq!(jsx_end(dollar), Some(dollar.len()));
        // A container whose `_`-named opener carries an expression attribute is
        // protected, and its `</_Box>` close is matched.
        let container = "<_Box id={i}>text</_Box>";
        assert_eq!(jsx_end(container), Some(container.len()));
    }

    #[test]
    fn scan_jsx_plain_self_closing_not_protected() {
        // A complete single-line tag is already preserved verbatim by comrak.
        assert_eq!(jsx_end("<Chart data=\"x\" />"), None);
    }

    #[test]
    fn scan_jsx_multiline_self_closing() {
        let span = "<PackageManagerTabs\n  command={{ npm: \"x\" }}\n/>";
        assert_eq!(jsx_end(span), Some(span.len()));
    }

    #[test]
    fn scan_jsx_multiline_open_tag_without_expression() {
        let span = "<Chart\n  data=\"x\"\n/>";
        assert_eq!(jsx_end(span), Some(span.len()));
    }

    #[test]
    fn scan_jsx_fragment() {
        let span = "<>hello</>";
        assert_eq!(jsx_end(span), Some(span.len()));
    }

    #[test]
    fn scan_jsx_container_with_expression_attr() {
        // A `{{…}}` object attribute is not valid inline HTML, so the whole
        // container is protected (a simple `{x}` attribute would instead be left
        // to comrak as inline HTML — see the integration tests).
        let span = "<Tabs value={{ id }}>content</Tabs>";
        assert_eq!(jsx_end(span), Some(span.len()));
    }

    #[test]
    fn scan_jsx_plain_container_not_protected() {
        // comrak preserves the tags; any expression children are handled
        // separately by the expression scanner.
        assert_eq!(jsx_end("<Tabs>content</Tabs>"), None);
    }

    #[test]
    fn scan_jsx_plain_container_with_braced_child_not_protected() {
        // The root tag is plain, so the whole container is not protected even
        // though a child has an expression attribute — the child is protected
        // separately and the prose between tags stays formattable.
        assert_eq!(jsx_end("<Note>x <Badge count={n} /></Note>"), None);
    }

    #[test]
    fn scan_jsx_double_brace_attribute() {
        let span = "<Foo style={{ color: \"red\" }} />";
        assert_eq!(jsx_end(span), Some(span.len()));
    }

    #[test]
    fn scan_jsx_nested_self_closing_in_braced_container() {
        let span = "<Foo bar={1}><Baz/></Foo>";
        assert_eq!(jsx_end(span), Some(span.len()));
    }

    #[test]
    fn scan_jsx_expression_attribute_with_angle_brackets() {
        // The `<Bar/>` lives inside an attribute expression; the element is
        // protected as a whole via its `{…}` attribute.
        let span = "<Foo render={() => <Bar/>} />";
        assert_eq!(jsx_end(span), Some(span.len()));
    }

    #[test]
    fn scan_jsx_attribute_string_with_angle_and_brace_not_protected() {
        // `>` and `{` live in a quoted attribute, so comrak parses the tag fine.
        assert_eq!(jsx_end("<Foo title=\"a > b {c}\" />"), None);
    }

    #[test]
    fn scan_jsx_rejects_comparison_and_non_tag() {
        assert_eq!(jsx_end("< b"), None);
        assert_eq!(jsx_end("<3 ideas"), None);
        assert_eq!(jsx_end("</close>"), None);
    }

    #[test]
    fn scan_jsx_rejects_autolink() {
        assert_eq!(jsx_end("<https://example.com>"), None);
        assert_eq!(jsx_end("<user@example.com>"), None);
    }

    #[test]
    fn scan_jsx_unterminated_gives_up() {
        // A protect-worthy container with no matching close cannot be delimited;
        // give up rather than partially protect.
        assert_eq!(scan_jsx("<Foo bar={1}>no close", 0), JsxScan::GiveUp);
    }

    #[test]
    fn scan_expression_simple() {
        let span = "{count}";
        assert_eq!(expr_end(span), Some(span.len()));
    }

    #[test]
    fn scan_expression_member_access() {
        let span = "{user.name}";
        assert_eq!(expr_end(span), Some(span.len()));
    }

    #[test]
    fn scan_expression_nested_braces() {
        let span = "{outer {inner} done}";
        assert_eq!(expr_end(span), Some(span.len()));
    }

    #[test]
    fn scan_expression_comment_with_quotes() {
        let span = "{/* a comment with \"quotes\" */}";
        assert_eq!(expr_end(span), Some(span.len()));
    }

    #[test]
    fn scan_expression_string_with_brace() {
        // A `}` inside a string must not close the expression early.
        let span = "{label(\"a } b\")}";
        assert_eq!(expr_end(span), Some(span.len()));
    }

    #[test]
    fn scan_expression_heading_anchor_excluded() {
        // `{#id}` is a Hongdown heading anchor, not an MDX expression.
        assert_eq!(scan_expression("{#my-id}", 0), ExprScan::NoMatch);
    }

    #[test]
    fn scan_expression_unbalanced_returns_none() {
        assert_eq!(scan_expression("{ unbalanced", 0), ExprScan::NoMatch);
    }

    #[test]
    fn scan_expression_regex_with_brace_in_class() {
        // A `}` inside a regex character class must not close the expression.
        let span = "{value.replace(/[}]/g, \"x\")}";
        assert_eq!(expr_end(span), Some(span.len()));
    }

    #[test]
    fn scan_expression_regex_with_brace_quantifier() {
        let span = "{/\\d{2,4}/.test(s)}";
        assert_eq!(expr_end(span), Some(span.len()));
    }

    #[test]
    fn scan_expression_division_is_not_regex() {
        // `/` after a value is division, not a regex; the expression still
        // balances normally.
        let span = "{a / b / c}";
        assert_eq!(expr_end(span), Some(span.len()));
    }

    #[test]
    fn skip_regex_handles_character_class() {
        let span = "/[}/]/g";
        assert_eq!(skip_regex(span, 0), Some(span.len()));
    }

    #[test]
    fn scan_expression_regex_after_keyword() {
        // A regex right after `return` (a keyword) must be recognized, so the
        // `}` in its character class does not end the expression early.
        let span = "{() => { return /[}]/.test(s); }}";
        assert_eq!(expr_end(span), Some(span.len()));
    }

    #[test]
    fn scan_expression_division_after_identifier_keyword_lookalike() {
        // `returnValue` is an identifier, not the `return` keyword, so `/` is
        // division — the expression still balances.
        let span = "{returnValue / 2}";
        assert_eq!(expr_end(span), Some(span.len()));
    }

    #[test]
    fn scan_expression_keyword_as_property_is_division() {
        // `return` used as a property name (`obj.return`) is not a keyword, so the
        // following `/` is division and the later regex's `}` is handled.
        let span = "{obj.return / /[}]/.source}";
        assert_eq!(expr_end(span), Some(span.len()));
    }

    #[test]
    fn scan_expression_slash_after_brace_gives_up() {
        // A `/` right after `}` is ambiguous (regex after a block close vs.
        // division after an object literal); we give up rather than guess.
        assert_eq!(
            scan_expression("{() => { if (x) {} /[}]/.test(s); }}", 0),
            ExprScan::GiveUp
        );
        assert_eq!(
            scan_expression("{{a:1} / b / /[}]/.source}", 0),
            ExprScan::GiveUp
        );
    }

    #[test]
    fn scan_expression_division_after_object_literal() {
        let span = "{foo({a:1}) / 2}";
        assert_eq!(expr_end(span), Some(span.len()));
    }

    #[test]
    fn protect_gives_up_on_jsx_with_ambiguous_child_without_partial() {
        // A protect-worthy container whose child expression is ambiguous must not
        // have the `{{ ok }}` in its opening tag protected in isolation.
        let source = "<Foo prop={{ ok }}>{() => { if (x) {} /[}]/.test(s); }}</Foo>\n";
        assert!(protect_source(source).is_none());
    }

    #[test]
    fn protect_gives_up_on_ambiguous_expression_without_partial() {
        // The inner `{a:1}` of an ambiguous expression must not be protected on
        // its own when the whole expression is given up.
        assert!(protect_source("Obj {{a:1} / b / /[}]/.source} weird.\n").is_none());
        // A real expression before the ambiguous one is still protected; the
        // inner braces of the ambiguous one are not.
        let protection =
            protect_source("Pre {realA} then {{a:1} / b / /[}]/.x} end.\n").expect("realA");
        assert_eq!(protection.replacements.len(), 1);
        assert_eq!(protection.replacements[0].original, "{realA}");
    }

    #[test]
    fn protect_protects_multiline_opener_without_expression() {
        // A multi-line opener with no `{…}` attribute is multi-line inline HTML
        // that comrak does not round-trip, so it must be protected (its
        // single-line `HtmlInline` skip exclusion does not apply).
        let source = "<Chart\n  data=\"value\"\n/>\n";
        let protection = protect_source(source).expect("should protect");
        assert_eq!(protection.replacements.len(), 1);
        assert_eq!(
            protection.replacements[0].original,
            "<Chart\n  data=\"value\"\n/>"
        );
    }

    #[test]
    fn protect_skips_braces_in_inline_code() {
        // `{x}` inside an inline code span is preserved by comrak verbatim, so it
        // must not be protected.
        assert!(protect_source("Use `{x}` in code.\n").is_none());
    }

    #[test]
    fn protect_skips_braces_in_math() {
        // Braces inside `$…$` math are part of the formula, not an expression.
        assert!(protect_source("The value $\\frac{1}{2}$ is a half.\n").is_none());
    }

    #[test]
    fn protect_records_expression_verbatim() {
        let source = "Hello {user.name}, welcome!\n";
        let protection = protect_source(source).expect("should protect");
        assert_eq!(protection.replacements.len(), 1);
        assert_eq!(protection.replacements[0].original, "{user.name}");
    }

    #[test]
    fn skip_string_handles_escapes() {
        let span = r#""a\"b""#; // "a\"b"
        assert_eq!(skip_string(span, 0), Some(span.len()));
    }

    #[test]
    fn skip_string_template_literal_with_nested_interpolation() {
        // A `${…}` interpolation, including a nested template literal inside it,
        // must not end the outer template early.
        let span = "`a ${`b`} c`";
        assert_eq!(skip_string(span, 0), Some(span.len()));
        let nested = "`x ${`y ${z} w`} v`";
        assert_eq!(skip_string(nested, 0), Some(nested.len()));
    }

    #[test]
    fn scan_expression_template_literal_with_nested_braces() {
        // The expression brace count stays balanced across a template literal
        // whose interpolation contains its own braces.
        let span = "{`${`a`} ${ {k: 1} }`}";
        assert_eq!(expr_end(span), Some(span.len()));
    }

    #[test]
    fn skip_comment_block_and_line() {
        // A closed block comment is skipped past its `*/`.
        assert_eq!(skip_comment("/* x */y", 0), Some("/* x */".len()));
        // A line comment that runs to the end of the span is complete.
        assert_eq!(skip_comment("// done", 0), Some("// done".len()));
        // An unterminated block comment is not a complete comment.
        assert_eq!(skip_comment("/* x", 0), None);
        // Not a comment.
        assert_eq!(skip_comment("/ x", 0), None);
    }

    #[test]
    fn protect_returns_none_without_constructs() {
        assert!(protect_source("Just some prose.\n\nMore prose.\n").is_none());
    }

    #[test]
    fn protect_records_import_verbatim() {
        let source = "import { Chart } from \"./chart.js\";\n\nProse.\n";
        let protection = protect_source(source).expect("should protect");
        assert_eq!(protection.replacements.len(), 1);
        assert_eq!(
            protection.replacements[0].original,
            "import { Chart } from \"./chart.js\";"
        );
        assert!(protection.source.contains("<!--hongdown-mdx:"));
        assert!(protection.source.contains("Prose."));
        assert!(!protection.source.contains("import {"));
    }

    #[test]
    fn restore_round_trips() {
        let source = "export const meta = { author: 'Hong Minhee' };\n";
        let protection = protect_source(source).expect("should protect");
        let restored = protection.restore(&protection.source);
        assert!(restored.contains("export const meta = { author: 'Hong Minhee' };"));
    }

    #[test]
    fn original_line_maps_back_across_multiline_construct() {
        // A 3-line import collapses to a single-line placeholder; lines after it
        // must map back to their original line numbers.
        let source = "import {\n  a,\n} from \"x\";\n\nProse line.\n";
        let protection = protect_source(source).expect("should protect");
        // "Prose line." is original line 6; in the protected source the import is
        // one line, so it is on protected line 4.
        assert_eq!(protection.original_line(4), 6);
        // Lines within/before the placeholder are unchanged.
        assert_eq!(protection.original_line(1), 1);
    }

    #[test]
    fn protect_skips_code_blocks() {
        // An import inside a fenced code block is not paragraph text, so it must
        // not be protected.
        let source = "~~~~ js\nimport x from \"y\";\n~~~~\n";
        assert!(protect_source(source).is_none());
    }
}
