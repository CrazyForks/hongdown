//! Document-level serialization logic.

use std::sync::LazyLock;

use comrak::nodes::{AstNode, NodeValue};
use comrak::{Arena, parse_document};
use regex::Regex;
use unicode_width::UnicodeWidthStr;

use super::Serializer;
use super::escape;
use super::state::{Directive, FormatSkipMode, ReferenceLink, normalize_reference_key};
use super::wrap;

/// Matches whatever container markers open the line a reference definition
/// starts on.
///
/// A definition keeps defining a document-wide label wherever it is written, so
/// a blockquote marker or a list marker may stand before it.  `- [x]: done` is
/// no task list item — the colon rules that out — and its label is as
/// document-wide as any other.
static DEFINITION_PREFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:[ \t>]|(?:[-*+]|\d{1,9}[.)])[ \t]+)*").unwrap());

/// Matches the container markers a definition's later lines carry.
///
/// Only a blockquote marks each of its lines; a list marks the first and
/// indents the rest.  So a list marker below a definition's first line opens an
/// item of its own, which ends the definition rather than continuing it.
static CONTINUATION_PREFIX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[ \t>]*").unwrap());

/// Matches a list marker that opens a block in the middle of one.
///
/// A bullet does so wherever it stands, but an ordered marker only where it
/// numbers the list from one: `2.` below a line goes on reading as part of it.
static LIST_MARKER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:[-*+]|1[.)])[ \t]").unwrap());

/// A reference definition that a link preserved verbatim depends on.
struct VerbatimReference {
    label: String,
    url: String,
    title: String,
    /// Line of the link that needs it, for reporting.
    line: usize,
}

struct DisableRegionBounds {
    region_start: usize,
    region_end: usize,
    enable_index: Option<usize>,
    disables_file: bool,
}

struct UnresolvedReferenceInline {
    text: String,
    opener_ranges: Vec<(usize, usize)>,
    opaque_ranges: Vec<(usize, usize)>,
    heading_removed_ranges: Vec<(usize, usize)>,
    heading: bool,
    start_line: usize,
    line_columns: Vec<usize>,
    line_offsets: Vec<usize>,
}

struct UndefinedReference {
    line: usize,
    label: String,
    warning_exempt: bool,
}

#[cfg(test)]
std::thread_local! {
    static HEADING_REFERENCE_RENDER_PASSES: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
    static UNRESOLVED_SOURCE_INDEX_ENTRIES: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
    static SOURCE_EXCLUSION_RANGE_SCANS: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

impl<'a> Serializer<'a> {
    pub(super) fn serialize_document<'b>(&mut self, node: &'b AstNode<'b>) {
        let children: Vec<_> = node.children().collect();

        // Source ranges that are copied verbatim rather than rebuilt from the
        // AST.  Whatever they contain — reference definitions, footnote
        // definitions, comments — travels with the copy and must not be
        // emitted a second time from elsewhere.
        let verbatim_ranges = self.collect_verbatim_line_ranges(&children);
        // Everything copied from the source, whether as a whole region or a
        // block at a time.  A block's span reaches back over any definition
        // consumed at its head, so those copies carry definitions of their own.
        // A document is largely copied blocks where it is copied at all, so
        // this is a line-indexed table rather than a list of spans to walk.
        let mut copied_lines = vec![false; self.source_lines.len()];
        Self::mark_lines(&verbatim_ranges, &mut copied_lines);
        Self::mark_copied_block_lines(&children, &mut copied_lines);

        // The analysis below costs a pass over the document and only says
        // something about content that keeps its source text, so a document
        // that copies none skips it.
        if copied_lines.contains(&true) {
            let mut in_leaf_block = vec![false; self.source_lines.len()];
            Self::mark_leaf_block_lines(node, &self.source_lines, &mut in_leaf_block);
            for (label, lines) in
                self.collect_reference_definition_lines(&self.source_lines, &in_leaf_block)
            {
                // Only the first definition of a label counts, the way
                // CommonMark resolves it.  When it is the one the copy carries,
                // the copy also defines it; when a *later* one is, the copy
                // merely repeats a definition that has no effect where it
                // stands but would take effect if the winning one were emitted
                // after it.
                let mut lines = lines.into_iter();
                let winner = lines.next().unwrap_or_default();
                if Self::is_line_marked(winner, &copied_lines) {
                    self.verbatim_reference_labels.insert(label.clone());
                } else if lines.any(|line| Self::is_line_marked(line, &copied_lines)) {
                    self.shadowed_reference_labels.insert(label.clone());
                }
                self.reference_definition_lines.insert(label, winner);
            }
            self.collect_verbatim_reference_claims(node, &copied_lines, 0);

            // Keep alive the definitions those links depend on.  Each falls due
            // where the source puts it, so reserving them all here rather than
            // as each copy is emitted changes nothing about where they land —
            // except that one due before a copy can no longer miss the flush
            // that belongs above it.
            let mut references = Vec::new();
            self.collect_verbatim_references(node, &copied_lines, &mut references, 0);
            self.reserve_verbatim_references(references.iter());
        }

        // Check for undefined reference links using AST
        let disabled_ranges = Self::collect_disabled_line_ranges(node);
        self.check_undefined_references_ast(node, &disabled_ranges);

        // First pass: collect all footnote reference lines
        // This is needed because FootnoteDefinition nodes come at the end of the AST,
        // but we need to know reference lines before flushing at section boundaries
        self.collect_footnote_reference_lines(node);

        // Second pass: process all FootnoteDefinition nodes first
        // This ensures pending_footnotes is populated before we flush at section boundaries
        for child in &children {
            if let NodeValue::FootnoteDefinition(_) = &child.data.borrow().value {
                // A footnote definition inside a verbatim range is copied with
                // the range, so collecting it here would emit it twice.
                let line = child.data.borrow().sourcepos.start.line;
                if Self::is_line_in_ranges(line, &verbatim_ranges) {
                    continue;
                }
                self.serialize_node(child);
            }
        }

        // Identify trailing HTML blocks (non-directive comments at the end of document)
        // These should be output after reference definitions to maintain their position
        let trailing_html_start = self.find_trailing_html_blocks(&children, &verbatim_ranges);

        // Index before which children have already been emitted as part of a
        // verbatim region.
        let mut skip_until = 0usize;

        for (i, child) in children.iter().enumerate() {
            if i < skip_until {
                continue;
            }
            // Skip trailing HTML blocks for now - they'll be output after references
            if i >= trailing_html_start
                && let NodeValue::HtmlBlock(_) = &child.data.borrow().value
            {
                continue;
            }
            // Skip FootnoteDefinition nodes (already processed above)
            if let NodeValue::FootnoteDefinition(_) = &child.data.borrow().value {
                continue;
            }
            // Check for directives in HTML blocks
            if let NodeValue::HtmlBlock(html_block) = &child.data.borrow().value
                && let Some(directive) = Directive::parse(&html_block.literal)
            {
                match directive {
                    Directive::DisableFile => {
                        // Flush pending footnotes and references BEFORE the disable-file directive.
                        // Definitions that appear before the directive should stay before it.
                        let directive_line = child.data.borrow().sourcepos.start.line;
                        self.flush_footnotes_before(Some(directive_line));
                        self.flush_references_before(Some(directive_line));
                        self.flush_footnote_references_before(Some(directive_line));

                        // Output the directive comment, then output remaining content as-is.
                        // A definition flushed just above would otherwise run
                        // straight into the comment, and it has no node of its
                        // own for the child index to account for.
                        self.ensure_blank_line();
                        self.output.push_str(html_block.literal.trim_end());
                        // Get the line after the directive block ends
                        let directive_end_line = child.data.borrow().sourcepos.end.line;
                        // Extract everything from the next line to the end of file
                        if let Some(remaining) =
                            self.extract_source_from_line(directive_end_line + 1)
                        {
                            self.output.push('\n');
                            self.output.push_str(&remaining);
                        }
                        return;
                    }
                    Directive::DisableNextLine => {
                        // Flush pending footnotes and references BEFORE the directive.
                        // Definitions that appear before the directive should stay before it.
                        let directive_line = child.data.borrow().sourcepos.start.line;
                        self.flush_footnotes_before(Some(directive_line));
                        self.flush_references_before(Some(directive_line));
                        self.flush_footnote_references_before(Some(directive_line));

                        self.skip_mode = FormatSkipMode::NextBlock;
                        // Output the directive comment.  A definition flushed
                        // just above has no node of its own for the child index
                        // to account for, so the separator goes by the output.
                        self.ensure_blank_line();
                        self.output.push_str(&html_block.literal);
                        continue;
                    }
                    Directive::DisableNextSection => {
                        // Flush pending footnotes and references BEFORE the directive.
                        // Definitions that appear before the directive should stay before it.
                        let directive_line = child.data.borrow().sourcepos.start.line;
                        self.flush_footnotes_before(Some(directive_line));
                        self.flush_references_before(Some(directive_line));
                        self.flush_footnote_references_before(Some(directive_line));

                        self.skip_mode = FormatSkipMode::UntilSection;
                        // Output the directive comment.  A definition flushed
                        // just above has no node of its own for the child index
                        // to account for, so the separator goes by the output.
                        self.ensure_blank_line();
                        self.output.push_str(&html_block.literal);
                        continue;
                    }
                    Directive::Disable => {
                        // The region is copied from the source instead of being
                        // rebuilt block by block: a reference definition is
                        // consumed by the parser and has no node of its own, so
                        // a node-by-node copy would leave the region's links
                        // pointing at nothing.
                        let bounds = self.disable_region_bounds(&children, i);
                        let region_last = bounds.enable_index.unwrap_or(children.len());
                        let region = self
                            .extract_source_lines(bounds.region_start, bounds.region_end)
                            .map(|source| Self::trim_blank_lines(&source))
                            .unwrap_or_default();

                        // Flush pending footnotes and references BEFORE the disable directive.
                        // Definitions that appear before the directive should stay before it.
                        let directive_line = child.data.borrow().sourcepos.start.line;
                        self.flush_footnotes_before(Some(directive_line));
                        self.flush_references_before(Some(directive_line));
                        self.flush_footnote_references_before(Some(directive_line));

                        // Output the directive comment.  A definition flushed
                        // just above has no node of its own for the child index
                        // to account for, so the separator goes by the output.
                        self.ensure_blank_line();
                        self.output.push_str(html_block.literal.trim_end());
                        self.output.push('\n');

                        if region.is_empty() {
                            // Nothing to copy, either because the region is
                            // empty or because the source is unavailable; fall
                            // back to emitting its blocks one by one.
                            self.skip_mode = FormatSkipMode::Disabled;
                        } else {
                            // A directive naming nouns names them for the
                            // headings after the region as much as for the ones
                            // before it, which is what it did when the region
                            // was emitted a node at a time.  Its comment is
                            // part of the copy; only its effect is left to
                            // carry out.
                            for skipped in &children[i + 1..region_last] {
                                self.apply_noun_directive(skipped);
                            }
                            self.output.push('\n');
                            self.output.push_str(&region);
                            self.output.push('\n');
                            if bounds.disables_file {
                                return;
                            }
                            skip_until = region_last;
                        }
                        continue;
                    }
                    Directive::Enable => {
                        self.skip_mode = FormatSkipMode::None;
                        // Output the directive comment.  A definition flushed
                        // just above has no node of its own for the child index
                        // to account for, so the separator goes by the output.
                        self.ensure_blank_line();
                        self.output.push_str(&html_block.literal);
                        continue;
                    }
                    Directive::ProperNouns(nouns) => {
                        // Add to directive proper nouns list
                        self.directive_proper_nouns.extend(nouns);
                        // Output the directive comment.  A definition flushed
                        // just above has no node of its own for the child index
                        // to account for, so the separator goes by the output.
                        self.ensure_blank_line();
                        self.output.push_str(&html_block.literal);
                        continue;
                    }
                    Directive::CommonNouns(nouns) => {
                        // Add to directive common nouns list
                        self.directive_common_nouns.extend(nouns);
                        // Output the directive comment.  A definition flushed
                        // just above has no node of its own for the child index
                        // to account for, so the separator goes by the output.
                        self.ensure_blank_line();
                        self.output.push_str(&html_block.literal);
                        continue;
                    }
                }
            }

            // Check if we're about to start a new section (h2 or h3 heading)
            // If so, flush any pending references and footnotes first
            let heading_level = match &child.data.borrow().value {
                NodeValue::Heading(h) => Some(h.level),
                _ => None,
            };
            let is_h2 = heading_level == Some(2);
            let is_h2_or_h3 = matches!(heading_level, Some(2) | Some(3));

            if is_h2_or_h3 && i > 0 {
                // Get the source line of the heading to flush only earlier footnotes
                let heading_line = self.section_boundary_line(child);
                // Footnotes come before link reference definitions
                self.flush_footnotes_before(Some(heading_line));
                self.flush_references_before(Some(heading_line));
                self.flush_footnote_references_before(Some(heading_line));
            }

            // Add blank line between block elements (except after front matter)
            if i > 0 {
                let prev_is_front_matter = matches!(
                    &children[i - 1].data.borrow().value,
                    NodeValue::FrontMatter(_)
                );
                if prev_is_front_matter {
                    // No extra blank line needed after front matter
                } else if is_h2 {
                    // Check if previous element was a heading (empty section)
                    let prev_is_heading =
                        matches!(&children[i - 1].data.borrow().value, NodeValue::Heading(_));
                    if prev_is_heading {
                        // Just one blank line between consecutive headings
                        self.output.push('\n');
                    } else {
                        // Two blank lines before h2 sections (one after content + one extra)
                        self.output.push_str("\n\n");
                    }
                } else {
                    self.output.push('\n');
                }
            }

            // Check if this block should be output as-is (skip formatting)
            if self.should_skip_formatting() {
                // For NextBlock mode, reset after this block
                let was_next_block = self.skip_mode == FormatSkipMode::NextBlock;
                if was_next_block {
                    self.skip_mode = FormatSkipMode::None;
                }

                // For UntilSection mode, check if this is a heading to reset
                if self.skip_mode == FormatSkipMode::UntilSection
                    && let NodeValue::Heading(h) = &child.data.borrow().value
                    && h.level <= 2
                {
                    self.skip_mode = FormatSkipMode::None;
                    // Continue with normal formatting for this heading
                    self.serialize_node(child);
                    continue;
                }

                // Output the original source, by whole lines.  A block's span
                // begins at its content, past the indentation and the markers
                // of whatever holds it, and a copy that started there would
                // lose them — including a reference definition the parser took
                // out of a list item, which the span then leaves behind.  It is
                // also what the copy is taken to be everywhere else: the lines
                // it occupies, which is what marks them as copied.
                let sourcepos = child.data.borrow().sourcepos;
                let source = self
                    .extract_source_lines(sourcepos.start.line, sourcepos.end.line)
                    // A block's span may reach over the blank lines below it,
                    // which the separator between blocks supplies again; keeping
                    // both would add one more on every run.
                    .map(|source| Self::trim_blank_lines(&source))
                    .filter(|source| !source.is_empty());
                if let Some(source) = source {
                    self.output.push_str(&source);
                    self.output.push('\n');
                } else {
                    self.serialize_node(child);
                }
                continue;
            }

            self.serialize_node(child);
        }

        // Footnotes come before link reference definitions
        self.flush_footnotes();
        self.flush_references();
        self.flush_footnote_references();

        // Output trailing HTML blocks after references and footnotes
        self.output_trailing_html_blocks(&children, trailing_html_start);
    }

    /// Whether any of these nodes is a directive disabling the file, which
    /// leaves the rest of it as it stands wherever the directive is written,
    /// the end of an enclosing region included.
    fn disables_file<'b>(children: &[&'b AstNode<'b>]) -> bool {
        children.iter().any(|child| {
            matches!(
                &child.data.borrow().value,
                NodeValue::HtmlBlock(html_block)
                    if Directive::parse(&html_block.literal) == Some(Directive::DisableFile)
            )
        })
    }

    /// Take up the nouns a node names, where the node is a directive naming
    /// any.  Whether the node's own text is formatted or copied has no bearing
    /// on the headings it speaks for.
    fn apply_noun_directive<'b>(&mut self, node: &'b AstNode<'b>) {
        let directive = match &node.data.borrow().value {
            NodeValue::HtmlBlock(html_block) => Directive::parse(&html_block.literal),
            _ => None,
        };
        match directive {
            Some(Directive::ProperNouns(nouns)) => self.directive_proper_nouns.extend(nouns),
            Some(Directive::CommonNouns(nouns)) => self.directive_common_nouns.extend(nouns),
            _ => {}
        }
    }

    /// Find the index of the `hongdown-enable` directive that closes the
    /// `hongdown-disable` directive at `from`, if the document has one.
    fn find_enable_directive<'b>(children: &[&'b AstNode<'b>], from: usize) -> Option<usize> {
        children
            .iter()
            .enumerate()
            .skip(from + 1)
            .find(|(_, child)| {
                matches!(
                    &child.data.borrow().value,
                    NodeValue::HtmlBlock(html_block)
                        if Directive::parse(&html_block.literal) == Some(Directive::Enable)
                )
            })
            .map(|(i, _)| i)
    }

    /// Find the source bounds and closing directive of a disabled region.
    fn disable_region_bounds<'b>(
        &self,
        children: &[&'b AstNode<'b>],
        from: usize,
    ) -> DisableRegionBounds {
        let enable_index = Self::find_enable_directive(children, from);
        let region_last = enable_index.unwrap_or(children.len());
        let disables_file = Self::disables_file(&children[from + 1..region_last]);
        let region_end = match enable_index {
            Some(_) if disables_file => self.source_lines.len(),
            Some(index) => children[index]
                .data
                .borrow()
                .sourcepos
                .start
                .line
                .saturating_sub(1),
            None => self.source_lines.len(),
        };
        DisableRegionBounds {
            region_start: children[from].data.borrow().sourcepos.end.line + 1,
            region_end,
            enable_index,
            disables_file,
        }
    }

    /// Collect the source line ranges that are copied verbatim instead of being
    /// rebuilt from the AST: every `hongdown-disable` region and the tail that
    /// follows a `hongdown-disable-file` directive.
    fn collect_verbatim_line_ranges<'b>(
        &self,
        children: &[&'b AstNode<'b>],
    ) -> Vec<(usize, usize)> {
        let mut ranges = Vec::new();
        let last_line = self.source_lines.len();
        let mut i = 0;

        while i < children.len() {
            let directive = match &children[i].data.borrow().value {
                NodeValue::HtmlBlock(html_block) => Directive::parse(&html_block.literal),
                _ => None,
            };
            let start_line = children[i].data.borrow().sourcepos.end.line + 1;
            match directive {
                Some(Directive::DisableFile) => {
                    ranges.push((start_line, last_line));
                    break;
                }
                Some(Directive::Disable) => {
                    let bounds = self.disable_region_bounds(children, i);
                    ranges.push((bounds.region_start, bounds.region_end));
                    if bounds.disables_file {
                        break;
                    }
                    // Nested directives inside the region are part of the copy.
                    i = bounds.enable_index.unwrap_or(children.len());
                }
                _ => {}
            }
            i += 1;
        }

        ranges
    }

    /// Mark, in a line-indexed table, the source lines the document's leaf
    /// blocks occupy.
    ///
    /// A reference definition is consumed by the parser and belongs to no node,
    /// so it can only live in the gaps this leaves: a line inside a leaf block
    /// is that block's content, however much it may resemble one.  Only blocks
    /// that can hold other blocks are descended into, since a definition may
    /// well sit between the children of a blockquote or a list item.
    ///
    /// A table rather than a list of ranges keeps the lookup constant-time; a
    /// document is mostly leaf blocks, so scanning a range list per line would
    /// cost time quadratic in the number of blocks.
    fn mark_leaf_block_lines<'b>(
        node: &'b AstNode<'b>,
        source_lines: &[&str],
        in_leaf_block: &mut [bool],
    ) {
        let data = node.data.borrow();
        let holds_blocks = matches!(
            &data.value,
            NodeValue::Document
                | NodeValue::BlockQuote
                | NodeValue::MultilineBlockQuote(_)
                | NodeValue::Alert(_)
                | NodeValue::List(_)
                | NodeValue::Item(_)
                | NodeValue::TaskItem(_)
                | NodeValue::DescriptionList
                | NodeValue::DescriptionItem(_)
                | NodeValue::DescriptionTerm
                | NodeValue::DescriptionDetails
                | NodeValue::FootnoteDefinition(_)
        );
        if !holds_blocks {
            let sourcepos = data.sourcepos;
            // A setext heading is built from a paragraph and swallows a
            // consumed head the same way, with its underline as one more line
            // of content below the text.
            let underline_lines = match &data.value {
                NodeValue::Paragraph => Some(0),
                NodeValue::Heading(heading) if heading.setext => Some(1),
                _ => None,
            };
            let start = if let Some(underline_lines) = underline_lines {
                drop(data);
                Self::leaf_content_start(
                    node,
                    source_lines,
                    sourcepos.start.line,
                    sourcepos.end.line,
                    underline_lines,
                )
            } else {
                sourcepos.start.line
            };
            Self::mark_lines(&[(start, sourcepos.end.line)], in_leaf_block);
            return;
        }
        drop(data);

        for child in node.children() {
            Self::mark_leaf_block_lines(child, source_lines, in_leaf_block);
        }
    }

    /// The first line of a block's own content, for the blocks that swallow a
    /// consumed head: a paragraph, and the setext heading built from one.
    ///
    /// Definitions are consumed from the head of such a block, and the node
    /// keeps the lines they occupied, so those lines have to stay visible to
    /// the definition scanner while the block's own lines do not.
    ///
    /// Two readings bound where the content starts, and a line is left visible
    /// only where they agree.  The content sits at the end of the span, one
    /// line per break it contains plus any `underline_lines` below it, which
    /// misjudges a block whose inline spans lines without a break node, such as
    /// a code span or display math holding a newline.  A consumed head is also
    /// made of definitions, so the first line that does not begin one ends it,
    /// which instead misjudges a block opening with something that merely
    /// resembles a definition.  Neither mistake survives the other.
    fn leaf_content_start<'b>(
        node: &'b AstNode<'b>,
        source_lines: &[&str],
        start_line: usize,
        end_line: usize,
        underline_lines: usize,
    ) -> usize {
        let by_breaks = end_line
            .saturating_sub(Self::count_line_breaks(node) + underline_lines)
            .max(start_line);

        let mut by_definitions = start_line;
        // A consumed head is made of definitions, each of which may take
        // several lines; the head ends where one no longer begins.
        while let Some((_, next)) =
            Self::read_reference_definition(source_lines, by_definitions, end_line)
        {
            by_definitions = next;
        }
        by_breaks.min(by_definitions)
    }

    /// How far a block's reported start sits above its own content, which is
    /// how far the lines reported for anything inside it sit above their real
    /// ones.
    ///
    /// Only the blocks that swallow a consumed head have such a gap; `None`
    /// says the node is not one of them and whatever gap encloses it still
    /// applies.
    fn consumed_head_offset<'b>(&self, node: &'b AstNode<'b>) -> Option<usize> {
        let (underline_lines, sourcepos) = {
            let data = node.data.borrow();
            let underline_lines = match &data.value {
                NodeValue::Paragraph => 0,
                NodeValue::Heading(heading) if heading.setext => 1,
                _ => return None,
            };
            (underline_lines, data.sourcepos)
        };
        let content_start = Self::leaf_content_start(
            node,
            &self.source_lines,
            sourcepos.start.line,
            sourcepos.end.line,
            underline_lines,
        );
        Some(content_start.saturating_sub(sourcepos.start.line))
    }

    /// Whether a line holds `delimiter` with no backslash escaping it.
    fn contains_unescaped(text: &str, delimiter: char) -> bool {
        super::find_unescaped(text, delimiter).is_some()
    }

    /// The part of a definition's text that follows its destination, which is
    /// where a title may be written.  `None` where the destination is malformed
    /// and the text is therefore no definition's.
    ///
    /// The destination comes first and may hold a quote or a parenthesis of its
    /// own, so only what follows it can open a title.
    fn title_after_destination(text: &str) -> Option<&str> {
        let trimmed = text.trim_start();
        match trimmed.strip_prefix('<') {
            // An unterminated `<…>` is no destination at all.
            Some(pointy) => Some(&pointy[super::find_unescaped(pointy, '>')? + 1..]),
            None => Some(match trimmed.split_once(char::is_whitespace) {
                Some((_, title)) => title,
                None => "",
            }),
        }
    }

    /// Read the reference definition that begins on line `start`, returning its
    /// label and the line after the last one it occupies.  Lines are 1-indexed,
    /// and the definition may not reach `limit`.
    ///
    /// A definition can be written over several lines in every part of it: the
    /// label may break across lines, the destination may sit below the label,
    /// and a title may follow the destination or begin on a line of its own and
    /// run on until its delimiter closes.  Reading all of that in one place is
    /// what keeps the two callers agreeing on where a definition ends: one
    /// walks the definitions a paragraph swallowed at its head, the other the
    /// definitions a document carries between its blocks.
    fn read_reference_definition(
        source_lines: &[&str],
        start: usize,
        limit: usize,
    ) -> Option<(String, usize)> {
        if start >= limit {
            return None;
        }
        let mut rest =
            Self::strip_definition_prefix(source_lines.get(start - 1)?).strip_prefix('[')?;
        let mut line = start;

        // The label, which runs to the first `]` a backslash does not escape,
        // however many lines below that is.
        let mut label = String::new();
        let after_label = loop {
            match super::find_unescaped(rest, ']') {
                Some(end) => {
                    label.push_str(&rest[..end]);
                    break &rest[end + 1..];
                }
                None => {
                    label.push_str(rest);
                    label.push(' ');
                    line += 1;
                    if line >= limit {
                        return None;
                    }
                    rest = Self::strip_continuation_prefix(source_lines.get(line - 1)?)?;
                    if rest.trim().is_empty() {
                        return None;
                    }
                }
            }
        };
        let mut rest = after_label.strip_prefix(':')?;

        // The destination, on the label's last line or the one below it.
        if rest.trim().is_empty() {
            if line + 1 >= limit {
                return Some((label, line + 1));
            }
            line += 1;
            rest = Self::strip_continuation_prefix(source_lines.get(line - 1)?)?;
        }

        // The title, after the destination or on a line of its own below it.
        let title = Self::title_after_destination(rest)?;
        let mut closing = Self::unclosed_delimiter_in(title);
        if title.trim().is_empty()
            && line + 1 < limit
            && let Some(below) = source_lines
                .get(line)
                .and_then(|line| Self::strip_continuation_prefix(line))
            && Self::starts_title(below)
        {
            line += 1;
            closing = Self::unclosed_delimiter_in(below);
        }
        if let Some(closing) = closing {
            // The delimiter that opened the title is what closes it, however
            // many lines later, and one the source escapes closes nothing.
            while line + 1 < limit {
                line += 1;
                if Self::contains_unescaped(source_lines.get(line - 1)?, closing) {
                    break;
                }
            }
        }

        Some((label, line + 1))
    }

    /// The content of a definition's first line, past the container markers
    /// that open it.
    fn strip_definition_prefix(line: &str) -> &str {
        match DEFINITION_PREFIX.find(line) {
            Some(prefix) => &line[prefix.end()..],
            None => line,
        }
    }

    /// The content of a line continuing a definition, past the markers of the
    /// container it sits in.  `None` where the line opens a block of its own
    /// instead, which ends the definition rather than continuing it.
    fn strip_continuation_prefix(line: &str) -> Option<&str> {
        let content = match CONTINUATION_PREFIX.find(line) {
            Some(prefix) => &line[prefix.end()..],
            None => line,
        };
        (!LIST_MARKER.is_match(content)).then_some(content)
    }

    /// Whether a line of its own begins a title, as one below a definition's
    /// destination may.
    fn starts_title(line: &str) -> bool {
        line.trim_start().starts_with(['"', '\'', '('])
    }

    /// What a stretch of title text leaves for a later line to close.
    fn unclosed_delimiter_in(title: &str) -> Option<char> {
        let mut escaped = false;
        let mut quote = None;
        let mut depth = 0usize;
        for character in title.chars() {
            if escaped {
                escaped = false;
                continue;
            }
            match character {
                '\\' => escaped = true,
                '"' | '\'' if quote == Some(character) => quote = None,
                '"' | '\'' if quote.is_none() && depth == 0 => quote = Some(character),
                '(' if quote.is_none() => depth += 1,
                ')' if quote.is_none() => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        quote.or(if depth > 0 { Some(')') } else { None })
    }

    /// Count the line breaks within an inline subtree, which is one fewer than
    /// the number of source lines its content occupies.
    fn count_line_breaks<'b>(node: &'b AstNode<'b>) -> usize {
        let own = match &node.data.borrow().value {
            NodeValue::SoftBreak | NodeValue::LineBreak => 1,
            _ => 0,
        };
        own + node.children().map(Self::count_line_breaks).sum::<usize>()
    }

    /// The line at which a heading opens its section, for deciding what came
    /// before it.
    ///
    /// A setext heading reports the lines of any definition consumed at its
    /// head as its own, so its reported start sits on top of content that in
    /// fact precedes it.  Its content start is where the section really begins.
    fn section_boundary_line<'b>(&self, node: &'b AstNode<'b>) -> usize {
        let (setext, sourcepos) = {
            let data = node.data.borrow();
            let setext = matches!(&data.value, NodeValue::Heading(heading) if heading.setext);
            (setext, data.sourcepos)
        };
        if !setext {
            return sourcepos.start.line;
        }
        Self::leaf_content_start(
            node,
            &self.source_lines,
            sourcepos.start.line,
            sourcepos.end.line,
            1,
        )
    }

    /// Mark every line of the given ranges in a line-indexed table.
    fn mark_lines(ranges: &[(usize, usize)], lines: &mut [bool]) {
        for (start, end) in ranges {
            let start = (*start).max(1);
            let end = (*end).min(lines.len());
            for line in start..=end {
                lines[line - 1] = true;
            }
        }
    }

    /// Whether a line is marked in a line-indexed table.
    fn is_line_marked(line: usize, lines: &[bool]) -> bool {
        line > 0 && lines.get(line - 1).copied().unwrap_or(false)
    }

    /// Mark the source lines of the blocks the skip modes copy one at a time,
    /// as opposed to the regions copied whole by
    /// [`Self::collect_verbatim_line_ranges`], whose lines `copied_lines`
    /// already carries.
    ///
    /// A block is copied by its span, which reaches back over any definition
    /// consumed at its head, so such a definition is carried by the copy even
    /// though nothing else in the block refers to it.
    ///
    /// The modes are followed the way serialization follows them, since only
    /// the blocks it actually copies carry anything: a footnote definition is
    /// emitted by the footnote machinery and leaves the mode untouched, a
    /// directive naming nouns leaves it untouched as well, and `Enable` ends a
    /// section-wide skip just as the next section's heading does.
    fn mark_copied_block_lines<'b>(children: &[&'b AstNode<'b>], copied_lines: &mut [bool]) {
        let mut skip_mode = FormatSkipMode::None;

        for child in children {
            let data = child.data.borrow();
            let sourcepos = data.sourcepos;

            // What a region copies whole is already marked, and its blocks
            // never run through the modes at all.
            if Self::is_line_marked(sourcepos.start.line, copied_lines) {
                continue;
            }
            if let NodeValue::FootnoteDefinition(_) = &data.value {
                continue;
            }
            if let NodeValue::HtmlBlock(html_block) = &data.value
                && let Some(directive) = Directive::parse(&html_block.literal)
            {
                match directive {
                    Directive::DisableNextLine => skip_mode = FormatSkipMode::NextBlock,
                    Directive::DisableNextSection => skip_mode = FormatSkipMode::UntilSection,
                    Directive::DisableFile => break,
                    Directive::Disable | Directive::Enable => skip_mode = FormatSkipMode::None,
                    Directive::ProperNouns(_) | Directive::CommonNouns(_) => {}
                }
                continue;
            }

            let copied = match skip_mode {
                FormatSkipMode::NextBlock => {
                    skip_mode = FormatSkipMode::None;
                    true
                }
                FormatSkipMode::UntilSection => {
                    // The heading that opens the next section is formatted like
                    // any other, and ends the skip.
                    if matches!(&data.value, NodeValue::Heading(heading) if heading.level <= 2) {
                        skip_mode = FormatSkipMode::None;
                        false
                    } else {
                        true
                    }
                }
                FormatSkipMode::None | FormatSkipMode::Disabled => false,
            };
            if copied {
                Self::mark_lines(&[(sourcepos.start.line, sourcepos.end.line)], copied_lines);
            }
        }
    }

    /// Collect the normalized labels of the reference definitions the source
    /// carries, each mapped to the lines defining it, in order.
    ///
    /// The parser resolves references through a map it does not expose, so the
    /// definitions have to be recognized from the source.  What makes that
    /// reliable is `in_leaf_block`: everything the parser did turn into a block
    /// is excluded, and a line the parser kept out of every block while it
    /// looks like a definition is one.  Prose that merely resembles a
    /// definition, in a paragraph, a code block, raw HTML or a heading, is part
    /// of its block and never reaches this test.
    fn collect_reference_definition_lines(
        &self,
        source_lines: &[&str],
        in_leaf_block: &[bool],
    ) -> std::collections::HashMap<String, Vec<usize>> {
        let mut definitions: std::collections::HashMap<String, Vec<usize>> =
            std::collections::HashMap::new();

        // A definition cannot reach into a block, so the first line of the next
        // one is as far as it could possibly go.  Taken in one pass from the
        // end, since a document may hold a long run of lines between blocks.
        let end = source_lines.len() + 1;
        let mut next_block = vec![end; source_lines.len() + 2];
        for line in (1..=source_lines.len()).rev() {
            next_block[line] = if in_leaf_block.get(line - 1).copied().unwrap_or(false) {
                line
            } else {
                next_block[line + 1]
            };
        }

        let mut line_number = 1;
        while line_number <= source_lines.len() {
            if in_leaf_block.get(line_number - 1).copied().unwrap_or(false) {
                line_number += 1;
                continue;
            }
            let limit = next_block[line_number];
            match Self::read_reference_definition(source_lines, line_number, limit) {
                // Footnote definitions are not reference definitions.
                Some((label, next)) => {
                    if !label.starts_with('^') {
                        definitions
                            .entry(self.reference_key(&label))
                            .or_default()
                            .push(line_number);
                    }
                    line_number = next.max(line_number + 1);
                }
                None => line_number += 1,
            }
        }
        definitions
    }

    /// Record the labels that links preserved verbatim depend on, before any
    /// link is serialized.
    ///
    /// A link that is copied from the source keeps the label the source gave
    /// it, whereas a formatted link can be given a derived label such as
    /// `[guide][guide 2]`.  Claiming the labels up front is therefore what lets
    /// the two coexist: the formatted link is the one that yields.
    fn collect_verbatim_reference_claims<'b>(
        &mut self,
        node: &'b AstNode<'b>,
        copied_lines: &[bool],
        line_offset: usize,
    ) {
        let child_offset = self.consumed_head_offset(node).unwrap_or(line_offset);
        for child in node.children() {
            self.collect_verbatim_reference_claims(child, copied_lines, child_offset);
        }

        let (target, line) = {
            let data = node.data.borrow();
            let target = match &data.value {
                NodeValue::Link(link) | NodeValue::Image(link) => {
                    Some((link.url.clone(), link.title.clone()))
                }
                _ => None,
            };
            (target, data.sourcepos.start.line + line_offset)
        };

        if let Some((url, title)) = target
            && Self::is_line_marked(line, copied_lines)
            && let Some(label) = self.verbatim_reference_label(node, line_offset)
        {
            self.verbatim_reference_claims
                .entry(self.reference_key(&label))
                .or_insert(ReferenceLink { label, url, title });
        }
    }

    /// The label a reference-style link or image is written with in the source,
    /// with the collapsed-reference marker stripped.  `None` for links that are
    /// not reference style, and for the empty label, which cannot be defined.
    fn verbatim_reference_label<'b>(
        &self,
        node: &'b AstNode<'b>,
        line_offset: usize,
    ) -> Option<String> {
        let (_, label) = self.get_reference_style_info_shifted(node, line_offset)?;
        let label = label.strip_prefix('\x01').unwrap_or(&label);
        (!label.is_empty()).then(|| label.to_string())
    }

    /// Collect the reference definitions a verbatim block's links depend on.
    ///
    /// Such a link is copied from the source and so never registers the
    /// definition it uses; without [`Self::reserve_verbatim_references`] the
    /// definition would be dropped as unused, leaving the copy pointing at
    /// nothing.
    fn collect_verbatim_references<'b>(
        &self,
        node: &'b AstNode<'b>,
        copied_lines: &[bool],
        collected: &mut Vec<VerbatimReference>,
        line_offset: usize,
    ) {
        let child_offset = self.consumed_head_offset(node).unwrap_or(line_offset);
        // Children first, so that a badge's image comes before the link
        // wrapping it, the order the formatted path would register them in.
        for child in node.children() {
            self.collect_verbatim_references(child, copied_lines, collected, child_offset);
        }

        let (target, line) = {
            let data = node.data.borrow();
            let target = match &data.value {
                NodeValue::Link(link) | NodeValue::Image(link) => {
                    Some((link.url.clone(), link.title.clone()))
                }
                _ => None,
            };
            (target, data.sourcepos.start.line + line_offset)
        };

        if let Some((url, title)) = target
            && Self::is_line_marked(line, copied_lines)
            && let Some(label) = self.verbatim_reference_label(node, line_offset)
        {
            collected.push(VerbatimReference {
                label,
                url,
                title,
                line,
            });
        }
    }

    /// Keep the given definitions from being dropped as unused, reporting the
    /// ones whose label is already spoken for by a different destination.
    fn reserve_verbatim_references<'r>(
        &mut self,
        references: impl Iterator<Item = &'r VerbatimReference>,
    ) {
        for reference in references {
            // A definition inside a verbatim range travels with its own copy.
            let key = self.reference_key(&reference.label);
            if self.verbatim_reference_labels.contains(&key) {
                continue;
            }
            if !self.reserve_reference(&reference.label, &reference.url, &reference.title) {
                self.add_warning(
                    reference.line,
                    format!(
                        "reference definition [{}] cannot be kept: the label is \
                         already used for a different destination",
                        reference.label
                    ),
                );
            }
        }
    }

    /// Drop the leading and trailing blank lines of a verbatim source chunk,
    /// leaving the indentation of the remaining lines untouched.
    fn trim_blank_lines(source: &str) -> String {
        let lines: Vec<&str> = source.lines().collect();
        let Some(start) = lines.iter().position(|line| !line.trim().is_empty()) else {
            return String::new();
        };
        let end = lines
            .iter()
            .rposition(|line| !line.trim().is_empty())
            .unwrap_or(start);
        lines[start..=end].join("\n")
    }

    /// Find the index where trailing HTML blocks start.
    /// Returns `children.len()` if there are no trailing HTML blocks.
    fn find_trailing_html_blocks<'b>(
        &self,
        children: &[&'b AstNode<'b>],
        verbatim_ranges: &[(usize, usize)],
    ) -> usize {
        let mut trailing_start = children.len();

        // Walk backwards from the end, looking for consecutive HTML blocks
        // that are not formatting directives
        for (i, child) in children.iter().enumerate().rev() {
            match &child.data.borrow().value {
                NodeValue::HtmlBlock(html_block) => {
                    // Skip formatting directives - they should stay where they are
                    if Directive::parse(&html_block.literal).is_some() {
                        break;
                    }
                    // A block inside a verbatim range is emitted with that
                    // range; moving it here would emit it twice.
                    let start_line = child.data.borrow().sourcepos.start.line;
                    if Self::is_line_in_ranges(start_line, verbatim_ranges) {
                        break;
                    }
                    // This is a regular HTML block (e.g., comment) - mark as trailing
                    trailing_start = i;
                }
                NodeValue::FootnoteDefinition(_) => {
                    // Skip footnote definitions - they're handled separately
                    continue;
                }
                _ => {
                    // Non-HTML block found - stop looking
                    break;
                }
            }
        }

        trailing_start
    }

    /// Output trailing HTML blocks that were deferred until after references.
    fn output_trailing_html_blocks<'b>(
        &mut self,
        children: &[&'b AstNode<'b>],
        start_index: usize,
    ) {
        let mut is_first = true;
        let mut prev_end_line = 0usize;
        for (i, child) in children.iter().enumerate() {
            if i < start_index {
                continue;
            }

            let data = child.data.borrow();
            if let NodeValue::HtmlBlock(html_block) = &data.value {
                let sourcepos = data.sourcepos;
                if is_first {
                    // Add a blank line before the first trailing HTML block, but
                    // only when something precedes it — a document whose only
                    // content is HTML blocks must not gain leading blank lines.
                    if self.output.is_empty() {
                        // Nothing precedes this block; no separator needed.
                    } else if !self.output.ends_with("\n\n") {
                        if self.output.ends_with('\n') {
                            self.output.push('\n');
                        } else {
                            self.output.push_str("\n\n");
                        }
                    }
                    is_first = false;
                } else if sourcepos.start.line > prev_end_line + 1 && !self.output.ends_with("\n\n")
                {
                    // Preserve a blank line that separated two HTML blocks in the
                    // source (otherwise consecutive comments/placeholders would be
                    // concatenated without their original separation).
                    if self.output.ends_with('\n') {
                        self.output.push('\n');
                    } else {
                        self.output.push_str("\n\n");
                    }
                }
                prev_end_line = sourcepos.end.line;
                self.output.push_str(&html_block.literal);
            }
        }
    }

    pub(super) fn serialize_description_details<'b>(&mut self, node: &'b AstNode<'b>) {
        let children: Vec<_> = node.children().collect();

        // Set flag so nested lists know to add extra indentation
        let was_in_description_details = self.in_description_details;
        self.in_description_details = true;

        // Determine the prefix for blockquote context
        let blockquote_prefix = if self.in_block_quote {
            format!("{}{}", self.blockquote_outer_indent, self.blockquote_prefix)
        } else {
            String::new()
        };

        for (i, child) in children.iter().enumerate() {
            let child_value = &child.data.borrow().value;

            if i == 0 {
                // First child: start with `:   ` marker
                match child_value {
                    NodeValue::Paragraph => {
                        self.output.push_str(&blockquote_prefix);
                        self.output.push_str(":   ");
                        let mut content = String::new();
                        self.collect_inline_content(child, &mut content);
                        let continuation = format!("{}    ", blockquote_prefix);
                        let wrapped = wrap::wrap_text_first_line(
                            content.trim(),
                            "",
                            blockquote_prefix.width() + 4,
                            &continuation,
                            self.options.line_width.map(|lw| lw.get()),
                        );
                        self.output.push_str(&wrapped);
                        self.output.push('\n');
                    }
                    NodeValue::CodeBlock(code) => {
                        // Code block as first child (unusual but possible)
                        self.output.push_str(&blockquote_prefix);
                        self.output.push_str(":   ");
                        self.output.push('\n');
                        self.output.push_str(&blockquote_prefix);
                        self.output.push_str("    ");
                        self.serialize_code_block_with_indent(
                            child,
                            code,
                            &format!("{}    ", blockquote_prefix),
                        );
                    }
                    NodeValue::List(_) => {
                        // List as first child: output marker with 4 spaces, then list on same line
                        // This ensures idempotent formatting - the list stays inside the definition
                        self.output.push_str(&blockquote_prefix);
                        self.output.push_str(":    ");
                        // Set flag so list knows first item shouldn't have base indentation
                        self.description_details_first_list = true;
                        self.serialize_node(child);
                        self.description_details_first_list = false;
                    }
                    NodeValue::BlockQuote | NodeValue::Alert(_) => {
                        // Block quotes and alerts as first child: output marker, newline,
                        // then serialize with proper list_item_indent for continuation lines
                        self.output.push_str(&blockquote_prefix);
                        self.output.push_str(":\n");
                        let old_list_item_indent =
                            std::mem::replace(&mut self.list_item_indent, "    ".to_string());
                        self.serialize_node(child);
                        self.list_item_indent = old_list_item_indent;
                    }
                    _ => {
                        // Other block types: serialize normally with indent
                        self.output.push_str(&blockquote_prefix);
                        self.output.push_str(":   ");
                        self.serialize_node(child);
                    }
                }
            } else {
                // Subsequent children: need blank line and 4-space indent
                self.output.push('\n');
                match child_value {
                    NodeValue::Paragraph => {
                        self.output.push_str(&blockquote_prefix);
                        self.output.push_str("    ");
                        let mut content = String::new();
                        self.collect_inline_content(child, &mut content);
                        let continuation = format!("{}    ", blockquote_prefix);
                        let wrapped = wrap::wrap_text_first_line(
                            content.trim(),
                            "",
                            blockquote_prefix.width() + 4,
                            &continuation,
                            self.options.line_width.map(|lw| lw.get()),
                        );
                        self.output.push_str(&wrapped);
                        self.output.push('\n');
                    }
                    NodeValue::CodeBlock(code) => {
                        self.output.push_str(&blockquote_prefix);
                        self.output.push_str("    ");
                        self.serialize_code_block_with_indent(
                            child,
                            code,
                            &format!("{}    ", blockquote_prefix),
                        );
                    }
                    NodeValue::List(_) => {
                        // Lists handle their own indentation via in_description_details flag
                        self.serialize_node(child);
                    }
                    NodeValue::BlockQuote | NodeValue::Alert(_) => {
                        // Block quotes and alerts need list_item_indent to be set
                        // so that their continuation lines are properly indented
                        let old_list_item_indent =
                            std::mem::replace(&mut self.list_item_indent, "    ".to_string());
                        self.serialize_node(child);
                        self.list_item_indent = old_list_item_indent;
                    }
                    _ => {
                        // Other block types
                        self.output.push_str(&blockquote_prefix);
                        self.output.push_str("    ");
                        self.serialize_node(child);
                    }
                }
            }
        }

        self.in_description_details = was_in_description_details;
    }

    pub(super) fn serialize_heading<'b>(&mut self, node: &'b AstNode<'b>, level: u8) {
        // Collect heading text first
        let heading_text = self.collect_text(node);
        let (heading_body, trailing_anchor) =
            super::heading::split_trailing_explicit_anchor(&heading_text);
        let mut formatted_heading = heading_body.to_string();

        // Apply sentence case if enabled
        if self.options.heading_sentence_case {
            // Merge config proper nouns with directive proper nouns
            let mut proper_nouns = self.options.heading_proper_nouns.clone();
            proper_nouns.extend(self.directive_proper_nouns.clone());

            // Merge config common nouns with directive common nouns
            let mut common_nouns = self.options.heading_common_nouns.clone();
            common_nouns.extend(self.directive_common_nouns.clone());

            formatted_heading =
                super::heading::to_sentence_case(heading_body, &proper_nouns, &common_nouns);
        }
        let anchor_only = trailing_anchor.trim_start();
        if anchor_only.is_empty() {
            // No anchor: output heading body as-is (no spacing computation needed)
        } else {
            let is_atx =
                !(level == 1 && self.options.setext_h1 || level == 2 && self.options.setext_h2);
            let prefix_width = if is_atx { level as usize + 1 } else { 0 };
            let body_width = formatted_heading.width();
            let anchor_width = anchor_only.width();
            let gap = self.options.heading_anchor_align;

            // Hard cap to prevent pathological memory allocation from extreme
            // line_width or heading_anchor_align values.
            const MAX_ANCHOR_PADDING: usize = 10_000;

            let actual_spaces: usize = if gap >= 1 {
                (gap as usize).min(MAX_ANCHOR_PADDING)
            } else if let Some(lw) = self.options.line_width {
                // Saturating arithmetic avoids overflow for any line_width value.
                let target = lw.get().saturating_add_signed(gap as isize);
                let content_width = prefix_width
                    .saturating_add(body_width)
                    .saturating_add(anchor_width);
                let spaces = if target > content_width {
                    target - content_width
                } else {
                    1
                };
                spaces.min(MAX_ANCHOR_PADDING)
            } else {
                1
            };

            formatted_heading.push_str(&" ".repeat(actual_spaces));
            formatted_heading.push_str(anchor_only);
        }

        if level == 1 && self.options.setext_h1 {
            // Setext-style with '='
            self.output.push_str(&formatted_heading);
            self.output.push('\n');
            self.output.push_str(&"=".repeat(formatted_heading.width()));
            self.output.push('\n');
        } else if level == 2 && self.options.setext_h2 {
            // Setext-style with '-'
            self.output.push_str(&formatted_heading);
            self.output.push('\n');
            self.output.push_str(&"-".repeat(formatted_heading.width()));
            self.output.push('\n');
        } else {
            // ATX-style for level 3+ or when setext is disabled
            self.output.push_str(&"#".repeat(level as usize));
            self.output.push(' ');
            self.output.push_str(&formatted_heading);
            self.output.push('\n');
        }
    }

    pub(super) fn serialize_paragraph<'b>(&mut self, node: &'b AstNode<'b>) {
        // Check if this is a PHP Markdown Extra abbreviation definition (*[abbr]: ...)
        // These are not parsed by comrak, so we preserve them as-is
        if let Some(source) = self.extract_source(node) {
            let trimmed = source.trim();
            if trimmed.starts_with("*[") && trimmed.contains("]:") {
                self.output.push_str(trimmed);
                self.output.push('\n');
                return;
            }
        }

        // A paragraph that is a single math span is emitted verbatim, never
        // reflowed.  Display math (`$$…$$`) is parsed by comrak as an inline
        // node that may span several source lines; sending it through the
        // wrapper would turn its newlines into hard line breaks and corrupt the
        // formula.
        let single_math: Option<String> = {
            let children: Vec<_> = node.children().collect();
            if let [only] = children.as_slice() {
                let borrowed = only.data.borrow();
                if let NodeValue::Math(math) = &borrowed.value {
                    if self.list_type.is_none() && !self.in_block_quote {
                        // Top level: preserve the exact source span.
                        Some(self.render_math(only, math))
                    } else {
                        // Nested: reconstruct from the literal so the list or
                        // blockquote prefix added below is not stacked on top of
                        // the container indentation comrak already stripped.
                        Some(escape::format_math(&math.literal, math.display_math))
                    }
                } else {
                    None
                }
            } else {
                None
            }
        };

        // Collect all inline content first (not needed for single-math spans).
        let mut inline_content = String::new();
        if single_math.is_none() {
            self.collect_inline_content(node, &mut inline_content);
        }

        if self.list_type.is_some() {
            // Inside a list item, wrap with the same continuation indent used by other
            // list-item block children so preformatted input remains idempotent.
            let base_indent = self.calculate_list_item_base_indent();
            let continuation = if self.in_block_quote {
                // Inside a blockquote, continuation lines need the outer indent, the current
                // blockquote prefix, and the current list item's own continuation indent.
                format!(
                    "{}{}{}",
                    self.blockquote_outer_indent, self.blockquote_prefix, base_indent
                )
            } else {
                base_indent
            };
            if let Some(rendered) = &single_math {
                self.emit_verbatim_block(rendered, "", &continuation);
            } else {
                let wrapped = wrap::wrap_text_first_line(
                    inline_content.trim(),
                    "",
                    self.paragraph_first_line_prefix_width,
                    &continuation,
                    self.options.line_width.map(|lw| lw.get()),
                );
                self.output.push_str(&wrapped);
            }
        } else {
            // Not in a list - wrap the paragraph at line_width
            let prefix = if self.in_block_quote {
                format!("{}{}", self.blockquote_outer_indent, self.blockquote_prefix)
            } else {
                String::new()
            };
            if let Some(rendered) = &single_math {
                self.emit_verbatim_block(rendered, &prefix, &prefix);
            } else {
                let wrapped = wrap::wrap_text(
                    &inline_content,
                    &prefix,
                    self.options.line_width.map(|lw| lw.get()),
                );
                self.output.push_str(&wrapped);
            }
            self.output.push('\n');
        }
    }

    /// Emit pre-rendered, multi-line block content verbatim (no reflow),
    /// applying `first_prefix` to the first line and `cont_prefix` to each
    /// continuation line.  Used for display-math paragraphs so multi-line
    /// formulas keep their exact line structure inside lists and blockquotes.
    fn emit_verbatim_block(&mut self, rendered: &str, first_prefix: &str, cont_prefix: &str) {
        for (i, line) in rendered.lines().enumerate() {
            if i == 0 {
                self.output.push_str(first_prefix);
            } else {
                self.output.push('\n');
                self.output.push_str(cont_prefix);
            }
            self.output.push_str(line);
        }
    }

    pub(super) fn serialize_front_matter(&mut self, content: &str) {
        // Front matter content from comrak includes the delimiters,
        // so we preserve it verbatim and add a trailing blank line
        self.output.push_str(content.trim());
        self.output.push_str("\n\n");
    }

    /// Recursively collect footnote reference lines from the AST.
    /// This must be called before processing the document to ensure
    /// footnote reference lines are populated for all footnotes.
    fn collect_footnote_reference_lines<'b>(&mut self, node: &'b AstNode<'b>) {
        if let NodeValue::FootnoteReference(footnote_ref) = &node.data.borrow().value {
            let ref_line = node.data.borrow().sourcepos.start.line;
            self.footnotes
                .record_reference_line(footnote_ref.name.clone(), ref_line);
        }
        for child in node.children() {
            self.collect_footnote_reference_lines(child);
        }
    }

    /// Check for undefined reference links using AST traversal.
    ///
    /// This method walks the AST looking for Text nodes that contain `[label]`
    /// patterns. When comrak cannot resolve a reference link, it leaves the
    /// brackets as literal text. We detect these and emit warnings.
    ///
    /// We also check the original source to ensure the bracket wasn't
    /// intentionally escaped (e.g., `\[label]`).
    fn check_undefined_references_ast<'b>(
        &mut self,
        node: &'b AstNode<'b>,
        disabled_ranges: &[(usize, usize)],
    ) {
        if self.source_lines.is_empty() {
            return;
        }

        let opaque_block_ranges = Self::collect_opaque_block_line_ranges(node);
        let mut opaque_block_lines = vec![false; self.source_lines.len()];
        Self::mark_lines(&opaque_block_ranges, &mut opaque_block_lines);

        // Collect PHP Markdown Extra abbreviation definitions from source
        let abbreviations = Self::collect_abbreviations(&self.source_lines, &opaque_block_lines);

        // Collect reference definitions from source that comrak may not have parsed
        // (e.g., when they follow abbreviation definitions without a blank line)
        let source_ref_defs =
            Self::collect_source_reference_definitions(&self.source_lines, &opaque_block_lines);

        // Collect undefined references first to avoid borrow issues.
        let undefined_references = self.find_undefined_references_in_ast(
            node,
            &abbreviations,
            &source_ref_defs,
            disabled_ranges,
        );

        for reference in undefined_references {
            let label = self.restore_reference_label(&reference.label);
            // The label remains occupied even where the warning is suppressed:
            // defining it would still turn copied bracketed text into a link.
            self.unresolved_reference_labels
                .insert(self.reference_key(&label));

            // Filter out warnings that fall within disabled regions.
            if !reference.warning_exempt
                && !Self::is_line_in_ranges(reference.line, disabled_ranges)
            {
                self.add_warning(
                    reference.line,
                    format!("undefined reference link: [{}]", label),
                );
            }
        }
    }

    /// Collect line ranges that should be excluded from warnings due to
    /// formatting directives (hongdown-disable, hongdown-disable-next-line, etc.).
    ///
    /// Returns a vector of (start_line, end_line) tuples representing disabled ranges.
    fn collect_disabled_line_ranges<'b>(node: &'b AstNode<'b>) -> Vec<(usize, usize)> {
        let mut ranges = Vec::new();
        let children: Vec<_> = node.children().collect();

        for (i, child) in children.iter().enumerate() {
            if let NodeValue::HtmlBlock(html_block) = &child.data.borrow().value
                && let Some(directive) = Directive::parse(&html_block.literal)
            {
                match directive {
                    Directive::DisableFile => {
                        // Everything after this directive is disabled
                        let start_line = child.data.borrow().sourcepos.end.line + 1;
                        ranges.push((start_line, usize::MAX));
                    }
                    Directive::DisableNextLine => {
                        // Only the next block is disabled
                        if let Some(next_child) = children.get(i + 1) {
                            // Skip if next child is also a directive
                            if !matches!(
                                &next_child.data.borrow().value,
                                NodeValue::HtmlBlock(hb) if Directive::parse(&hb.literal).is_some()
                            ) {
                                let start_line = next_child.data.borrow().sourcepos.start.line;
                                let end_line = next_child.data.borrow().sourcepos.end.line;
                                ranges.push((start_line, end_line));
                            }
                        }
                    }
                    Directive::DisableNextSection => {
                        // Disabled until next h2 or lower heading
                        let start_line = child.data.borrow().sourcepos.end.line + 1;
                        let mut end_line = usize::MAX;

                        // Find the next section (h2 or lower)
                        for future_child in children.iter().skip(i + 1) {
                            if let NodeValue::Heading(h) = &future_child.data.borrow().value
                                && h.level <= 2
                            {
                                // End just before this heading
                                end_line = future_child.data.borrow().sourcepos.start.line - 1;
                                break;
                            }
                        }
                        ranges.push((start_line, end_line));
                    }
                    Directive::Disable => {
                        // Disabled until corresponding Enable directive
                        let start_line = child.data.borrow().sourcepos.end.line + 1;
                        let mut end_line = usize::MAX;

                        // Find the corresponding Enable directive
                        for future_child in children.iter().skip(i + 1) {
                            if let NodeValue::HtmlBlock(hb) = &future_child.data.borrow().value
                                && let Some(Directive::Enable) = Directive::parse(&hb.literal)
                            {
                                // End just before the Enable directive
                                end_line = future_child.data.borrow().sourcepos.start.line - 1;
                                break;
                            }
                        }
                        ranges.push((start_line, end_line));
                    }
                    Directive::Enable => {
                        // Enable doesn't start a new range, it ends one
                    }
                    Directive::ProperNouns(_) | Directive::CommonNouns(_) => {
                        // These directives don't affect warning ranges
                    }
                }
            }
        }

        ranges
    }

    /// Check if a line number falls within any of the given inclusive ranges.
    fn is_line_in_ranges(line: usize, ranges: &[(usize, usize)]) -> bool {
        ranges.iter().any(|(start, end)| {
            #[cfg(test)]
            SOURCE_EXCLUSION_RANGE_SCANS.with(|scans| scans.set(scans.get() + 1));
            line >= *start && line <= *end
        })
    }

    /// Collect source lines whose contents cannot define Markdown references.
    fn collect_opaque_block_line_ranges<'b>(node: &'b AstNode<'b>) -> Vec<(usize, usize)> {
        fn walk<'b>(node: &'b AstNode<'b>, ranges: &mut Vec<(usize, usize)>) {
            let data = node.data.borrow();
            if matches!(
                &data.value,
                NodeValue::CodeBlock(_) | NodeValue::HtmlBlock(_) | NodeValue::FrontMatter(_)
            ) {
                ranges.push((data.sourcepos.start.line, data.sourcepos.end.line));
                return;
            }
            drop(data);
            for child in node.children() {
                walk(child, ranges);
            }
        }

        let mut ranges = Vec::new();
        walk(node, &mut ranges);
        ranges
    }

    /// Collect PHP Markdown Extra abbreviation definitions from source.
    /// Returns a set of abbreviation names (e.g., "HTML" from "*[HTML]: Hyper Text Markup Language").
    fn collect_abbreviations(
        source_lines: &[&str],
        excluded_lines: &[bool],
    ) -> std::collections::HashSet<String> {
        let mut abbreviations = std::collections::HashSet::new();
        let abbr_pattern = Regex::new(r"^\*\[([^\]]+)\]:").unwrap();

        for (index, line) in source_lines.iter().enumerate() {
            if excluded_lines.get(index).copied().unwrap_or(false) {
                continue;
            }
            if let Some(caps) = abbr_pattern.captures(line)
                && let Some(abbr) = caps.get(1)
            {
                abbreviations.insert(abbr.as_str().to_string());
            }
        }

        abbreviations
    }

    /// Collect reference definitions from source that comrak may not have parsed.
    /// This happens when a reference definition follows an abbreviation definition
    /// without a blank line in between.
    /// Returns a set of (label, line_number) tuples.
    fn collect_source_reference_definitions(
        source_lines: &[&str],
        excluded_lines: &[bool],
    ) -> std::collections::HashSet<String> {
        let mut definitions = std::collections::HashSet::new();
        // Pattern: [label]: URL at start of line (with optional leading whitespace)
        let ref_def_pattern = Regex::new(r"^\s*\[([^\]]+)\]:\s*\S").unwrap();

        for (index, line) in source_lines.iter().enumerate() {
            if excluded_lines.get(index).copied().unwrap_or(false) {
                continue;
            }
            if let Some(caps) = ref_def_pattern.captures(line)
                && let Some(label) = caps.get(1)
            {
                definitions.insert(normalize_reference_key(label.as_str()));
            }
        }

        definitions
    }

    /// Find undefined references by walking the AST.
    /// Returns a vector of (line_number, label) tuples.
    fn find_undefined_references_in_ast<'b>(
        &mut self,
        node: &'b AstNode<'b>,
        abbreviations: &std::collections::HashSet<String>,
        source_ref_defs: &std::collections::HashSet<String>,
        disabled_ranges: &[(usize, usize)],
    ) -> Vec<UndefinedReference> {
        let mut undefined_references = Vec::new();
        let directive_proper_nouns = self.directive_proper_nouns.clone();
        let directive_common_nouns = self.directive_common_nouns.clone();

        self.walk_ast_for_undefined_refs(
            node,
            abbreviations,
            source_ref_defs,
            disabled_ranges,
            &mut undefined_references,
            0,
        );
        self.directive_proper_nouns = directive_proper_nouns;
        self.directive_common_nouns = directive_common_nouns;

        undefined_references
    }

    /// Recursively walk the AST looking for undefined references in Text nodes.
    fn walk_ast_for_undefined_refs<'b>(
        &mut self,
        node: &'b AstNode<'b>,
        abbreviations: &std::collections::HashSet<String>,
        source_ref_defs: &std::collections::HashSet<String>,
        disabled_ranges: &[(usize, usize)],
        undefined_references: &mut Vec<UndefinedReference>,
        line_offset: usize,
    ) {
        let data = node.data.borrow();
        let is_heading = matches!(&data.value, NodeValue::Heading(_));
        let is_inline_container = matches!(
            &data.value,
            NodeValue::Paragraph | NodeValue::Heading(_) | NodeValue::TableCell
        );
        let skip_children = matches!(
            &data.value,
            NodeValue::CodeBlock(_)
                | NodeValue::Link(_)
                | NodeValue::Image(_)
                | NodeValue::HtmlBlock(_)
        );

        drop(data);
        self.apply_noun_directive(node);
        let child_offset = self.consumed_head_offset(node).unwrap_or(line_offset);

        // Scan an inline container exactly once.  This lets a full reference
        // spanning sibling nodes choose its explicit label without a fragment
        // scan also reporting its link text.
        if is_inline_container {
            let Some(mut inline) =
                self.collect_undefined_reference_inline(node, child_offset, is_heading)
            else {
                return;
            };
            self.collect_undefined_reference_ranges(node, &mut inline, child_offset);
            inline.opener_ranges.sort_unstable();
            inline.opaque_ranges.sort_unstable();
            inline.heading_removed_ranges.sort_unstable();
            self.find_undefined_references_in_text(
                &inline,
                (node, child_offset),
                abbreviations,
                source_ref_defs,
                disabled_ranges,
                undefined_references,
            );
            return;
        }

        if skip_children {
            return;
        }

        // Recurse into children
        for child in node.children() {
            self.walk_ast_for_undefined_refs(
                child,
                abbreviations,
                source_ref_defs,
                disabled_ranges,
                undefined_references,
                child_offset,
            );
        }
    }

    /// Extract a container's complete inline source for unresolved-reference
    /// detection.
    ///
    /// Inline node source positions can omit source characters consumed by
    /// escapes.  The block-level container still spans the complete spelling,
    /// so use it as the text to scan and use text-node positions only to decide
    /// which opening brackets are literal inline text.
    fn collect_undefined_reference_inline<'b>(
        &self,
        node: &'b AstNode<'b>,
        line_offset: usize,
        heading: bool,
    ) -> Option<UnresolvedReferenceInline> {
        let data = node.data.borrow();
        let sourcepos = data.sourcepos;
        let description_term = node.parent().is_some_and(|parent| {
            matches!(&parent.data.borrow().value, NodeValue::DescriptionTerm)
        });
        let (text, start_line, start_column) = match &data.value {
            NodeValue::Heading(heading) if heading.setext => {
                let start_line = sourcepos.start.line + line_offset;
                let end_line = sourcepos.end.line.checked_sub(1)?;
                let end_column = self.source_lines.get(end_line - 1)?.len();
                (
                    self.extract_source_range(
                        start_line,
                        sourcepos.start.column,
                        end_line,
                        end_column,
                    )?,
                    start_line,
                    sourcepos.start.column,
                )
            }
            NodeValue::Heading(heading) => {
                let start_line = sourcepos.start.line + line_offset;
                let mut text = self.extract_source_range(
                    start_line,
                    sourcepos.start.column,
                    sourcepos.end.line,
                    sourcepos.end.column,
                )?;
                let prefix = heading.level as usize;
                let whitespace = text[prefix..]
                    .find(|character: char| !character.is_ascii_whitespace())
                    .unwrap_or(text.len() - prefix);
                let content_start = prefix + whitespace;
                text.drain(..content_start);
                Self::strip_atx_closing_sequence(&mut text);
                (text, start_line, sourcepos.start.column + content_start)
            }
            NodeValue::Paragraph if description_term => {
                let start_line = sourcepos.start.line + line_offset;
                let end_line = Self::inline_subtree_end_line(node) + line_offset;
                let end_column = self.source_lines.get(end_line - 1)?.len();
                (
                    self.extract_source_range(
                        start_line,
                        sourcepos.start.column,
                        end_line,
                        end_column,
                    )?,
                    start_line,
                    sourcepos.start.column,
                )
            }
            _ => {
                let start_line = sourcepos.start.line + line_offset;
                (
                    self.extract_source_range(
                        start_line,
                        sourcepos.start.column,
                        sourcepos.end.line,
                        sourcepos.end.column,
                    )?,
                    start_line,
                    sourcepos.start.column,
                )
            }
        };
        drop(data);

        let blockquote_indent = Self::blockquote_outer_indent(node);
        let mut rebuilt = String::with_capacity(text.len());
        let mut line_columns = Vec::new();
        for (line_index, source_line) in text.split('\n').enumerate() {
            if line_index > 0 {
                rebuilt.push('\n');
            }
            let prefix = if line_index > 0
                && let Some(indent) = blockquote_indent
            {
                Self::blockquote_prefix_len(source_line, indent)
            } else {
                0
            };
            line_columns.push(if line_index == 0 {
                start_column
            } else {
                prefix + 1
            });
            rebuilt.push_str(&source_line[prefix..]);
        }
        let text = rebuilt;
        let mut line_offsets = vec![0];
        line_offsets.extend(text.match_indices('\n').map(|(offset, _)| offset + 1));
        #[cfg(test)]
        UNRESOLVED_SOURCE_INDEX_ENTRIES.with(|entries| {
            entries.set(entries.get().max(line_offsets.len()));
        });

        Some(UnresolvedReferenceInline {
            text,
            opener_ranges: Vec::new(),
            opaque_ranges: Vec::new(),
            heading_removed_ranges: Vec::new(),
            heading,
            start_line,
            line_columns,
            line_offsets,
        })
    }

    /// Build minimal definitions that keep resolved reference nodes intact when
    /// an inline container is reparsed to render its unresolved labels.
    fn collect_resolved_reference_definitions<'b>(
        &self,
        node: &'b AstNode<'b>,
        line_offset: usize,
    ) -> String {
        let mut seen = std::collections::HashSet::new();
        let mut definitions = String::new();
        for descendant in node.descendants() {
            if !matches!(
                &descendant.data.borrow().value,
                NodeValue::Link(_) | NodeValue::Image(_)
            ) {
                continue;
            }
            let Some((text, label)) =
                self.get_reference_style_info_shifted(descendant, line_offset)
            else {
                continue;
            };
            let label = label.strip_prefix('\x01').unwrap_or(&label);
            let label = if label.is_empty() { &text } else { label };
            if !seen.insert(label.to_string()) {
                continue;
            }
            definitions.push('[');
            definitions.push_str(label);
            definitions.push_str("]: .\n");
        }
        definitions
    }

    /// Return the indentation before the outermost blockquote marker.
    fn blockquote_outer_indent<'b>(node: &'b AstNode<'b>) -> Option<usize> {
        let mut ancestor = node.parent();
        let mut indent = None;
        while let Some(parent) = ancestor {
            if matches!(
                &parent.data.borrow().value,
                NodeValue::BlockQuote | NodeValue::MultilineBlockQuote(_)
            ) {
                indent = Some(
                    parent
                        .data
                        .borrow()
                        .sourcepos
                        .start
                        .column
                        .saturating_sub(1),
                );
            }
            ancestor = parent.parent();
        }
        indent
    }

    /// Return the byte length of the blockquote container prefix on a source
    /// continuation line.
    fn blockquote_prefix_len(line: &str, outer_indent: usize) -> usize {
        let bytes = line.as_bytes();
        let mut cursor = 0;
        let mut column = 0;
        let mut prefix = 0;
        loop {
            let mut marker = cursor;
            let mut marker_column = column;
            while let Some(byte) = bytes.get(marker) {
                match byte {
                    b' ' => marker_column += 1,
                    b'\t' => marker_column += 4 - marker_column % 4,
                    _ => break,
                }
                marker += 1;
            }
            let allowed_indent = if prefix == 0 { outer_indent + 3 } else { 3 };
            if marker_column - column > allowed_indent || bytes.get(marker) != Some(&b'>') {
                break;
            }
            cursor = marker + 1;
            column = marker_column + 1;
            if bytes
                .get(cursor)
                .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
            {
                column += if bytes[cursor] == b'\t' {
                    4 - column % 4
                } else {
                    1
                };
                cursor += 1;
            }
            prefix = cursor;
        }
        prefix
    }

    fn inline_subtree_end_line<'b>(node: &'b AstNode<'b>) -> usize {
        node.children()
            .map(Self::inline_subtree_end_line)
            .fold(node.data.borrow().sourcepos.end.line, usize::max)
    }

    fn strip_atx_closing_sequence(text: &mut String) {
        let trimmed = text.trim_end_matches([' ', '\t']);
        let hashes = trimmed.trim_end_matches('#');
        if hashes.len() == trimmed.len() {
            return;
        }
        let Some(previous) = hashes.chars().next_back() else {
            text.clear();
            return;
        };
        if !previous.is_ascii_whitespace() {
            return;
        }
        text.truncate(hashes.trim_end_matches([' ', '\t']).len());
    }

    /// Record ordinary-text opener spans and opaque inline-node spans.
    fn collect_undefined_reference_ranges<'b>(
        &self,
        node: &'b AstNode<'b>,
        inline: &mut UnresolvedReferenceInline,
        line_offset: usize,
    ) {
        let data = node.data.borrow();
        let text = matches!(&data.value, NodeValue::Text(_));
        let heading_removed = inline.heading && matches!(&data.value, NodeValue::HtmlInline(_));
        let opaque = matches!(
            &data.value,
            NodeValue::Code(_)
                | NodeValue::HtmlInline(_)
                | NodeValue::Raw(_)
                | NodeValue::Math(_)
                | NodeValue::FootnoteReference(_)
                | NodeValue::WikiLink(_)
                | NodeValue::EscapedTag(_)
        );
        if text || opaque {
            let range = {
                let sourcepos = data.sourcepos;
                let start_line = sourcepos.start.line + line_offset;
                let end_line = sourcepos.end.line + line_offset;
                let Some(start) =
                    Self::inline_source_offset(inline, start_line, sourcepos.start.column)
                else {
                    return;
                };
                let Some(mut end) =
                    Self::inline_source_offset(inline, end_line, sourcepos.end.column)
                else {
                    return;
                };
                end = end.saturating_add(1).min(inline.text.len());
                (start, end)
            };
            if range.0 < range.1 {
                if text {
                    inline.opener_ranges.push(range);
                } else {
                    inline.opaque_ranges.push(range);
                    if heading_removed {
                        inline.heading_removed_ranges.push(range);
                    }
                }
            }
            return;
        }
        drop(data);

        for child in node.children() {
            self.collect_undefined_reference_ranges(child, inline, line_offset);
        }
    }

    fn inline_source_offset(
        inline: &UnresolvedReferenceInline,
        line: usize,
        column: usize,
    ) -> Option<usize> {
        let line_offset = line.checked_sub(inline.start_line)?;
        let offset = *inline.line_offsets.get(line_offset)?;
        let start_column = *inline.line_columns.get(line_offset)?;
        let column_offset = column.checked_sub(start_column)?;
        Some(offset + column_offset)
    }

    fn is_text_opener_at(inline: &UnresolvedReferenceInline, offset: usize) -> bool {
        let range = inline
            .opener_ranges
            .partition_point(|(_, to)| *to <= offset);
        inline
            .opener_ranges
            .get(range)
            .is_some_and(|(from, to)| *from <= offset && offset < *to)
    }

    fn is_opaque_at(inline: &UnresolvedReferenceInline, offset: usize) -> bool {
        let range = inline
            .opaque_ranges
            .partition_point(|(_, to)| *to <= offset);
        inline
            .opaque_ranges
            .get(range)
            .is_some_and(|(from, to)| *from <= offset && offset < *to)
    }

    fn heading_removed_range_end(
        inline: &UnresolvedReferenceInline,
        offset: usize,
    ) -> Option<usize> {
        let range = inline
            .heading_removed_ranges
            .partition_point(|(_, to)| *to <= offset);
        inline
            .heading_removed_ranges
            .get(range)
            .filter(|(from, to)| *from <= offset && offset < *to)
            .map(|(_, to)| *to)
    }

    fn inline_source_line(inline: &UnresolvedReferenceInline, offset: usize) -> usize {
        let line = inline
            .line_offsets
            .partition_point(|line_start| *line_start <= offset)
            .saturating_sub(1);
        inline.start_line + line
    }

    fn find_undefined_references_in_text<'b>(
        &self,
        inline: &UnresolvedReferenceInline,
        container: (&'b AstNode<'b>, usize),
        abbreviations: &std::collections::HashSet<String>,
        source_ref_defs: &std::collections::HashSet<String>,
        disabled_ranges: &[(usize, usize)],
        undefined_references: &mut Vec<UndefinedReference>,
    ) {
        let text = &inline.text;
        let bytes = text.as_bytes();
        let mut start = 0;
        let mut opener_range = 0;
        let mut references = Vec::new();
        while start < bytes.len() {
            while inline
                .opener_ranges
                .get(opener_range)
                .is_some_and(|(_, to)| *to <= start)
            {
                opener_range += 1;
            }
            let is_text_opener = inline
                .opener_ranges
                .get(opener_range)
                .is_some_and(|(from, to)| *from <= start && start < *to);
            if bytes[start] != b'[' || Self::is_escaped_at(text, start) || !is_text_opener {
                start += text[start..]
                    .chars()
                    .next()
                    .map(char::len_utf8)
                    .unwrap_or(1);
                continue;
            }

            let Some(first_end) = Self::find_reference_label_end(inline, start + 1) else {
                start += 1;
                continue;
            };
            let first_label = &text[start + 1..first_end];
            if first_label.is_empty() {
                start = first_end + 1;
                continue;
            }

            let mut raw_label_start = start + 1;
            let mut raw_label_end = first_end;
            let mut match_end = first_end + 1;
            if bytes.get(match_end) == Some(&b'[')
                && Self::is_text_opener_at(inline, match_end)
                && let Some(second_end) = Self::find_reference_label_end(inline, match_end + 1)
            {
                let explicit_label = &text[match_end + 1..second_end];
                if !normalize_reference_key(explicit_label).is_empty() {
                    raw_label_start = match_end + 1;
                    raw_label_end = second_end;
                }
                match_end = second_end + 1;
            }
            let raw_label = &text[raw_label_start..raw_label_end];

            if raw_label.starts_with('^') {
                start = match_end;
                continue;
            }

            let line = Self::inline_source_line(inline, start);
            let raw_key = normalize_reference_key(raw_label);
            let warning_exempt =
                abbreviations.contains(raw_label) || source_ref_defs.contains(&raw_key);
            let verbatim =
                warning_exempt && self.is_verbatim_reference_definition(inline, start, match_end);
            references.push((
                line,
                raw_label_start,
                raw_label_end,
                Self::is_line_in_ranges(line, disabled_ranges),
                warning_exempt,
                verbatim,
            ));
            start = match_end;
        }

        if references.is_empty() {
            return;
        }
        let resolved_reference_definitions =
            self.collect_resolved_reference_definitions(container.0, container.1);
        let heading_ranges: Vec<_> = references
            .iter()
            .filter(|(_, _, _, disabled, _, verbatim)| inline.heading && !disabled && !verbatim)
            .map(|(_, label_start, label_end, _, _, _)| (*label_start, *label_end))
            .collect();
        let mut heading_labels = self
            .render_unresolved_heading_reference_labels(
                text,
                &resolved_reference_definitions,
                &heading_ranges,
            )
            .into_iter();
        let inline_ranges: Vec<_> = references
            .iter()
            .filter(|(_, _, _, disabled, _, verbatim)| !inline.heading && !disabled && !verbatim)
            .map(|(_, label_start, label_end, _, _, _)| (*label_start, *label_end))
            .collect();
        let mut inline_labels = self
            .render_unresolved_inline_reference_labels(
                text,
                &resolved_reference_definitions,
                &inline_ranges,
            )
            .into_iter();
        for (line, label_start, label_end, disabled, warning_exempt, verbatim) in references {
            let raw_label = &text[label_start..label_end];
            let emitted_label = if disabled || verbatim {
                raw_label.to_string()
            } else if inline.heading {
                heading_labels
                    .next()
                    .unwrap_or_else(|| self.render_unresolved_reference_label(raw_label))
            } else {
                inline_labels
                    .next()
                    .unwrap_or_else(|| self.render_unresolved_reference_label(raw_label))
            };
            undefined_references.push(UndefinedReference {
                line,
                label: emitted_label,
                warning_exempt,
            });
        }
    }

    fn is_verbatim_reference_definition(
        &self,
        inline: &UnresolvedReferenceInline,
        label_start: usize,
        match_end: usize,
    ) -> bool {
        if inline.text.as_bytes().get(match_end) != Some(&b':') {
            return false;
        }
        let line_index = inline
            .line_offsets
            .partition_point(|line_start| *line_start <= label_start)
            .saturating_sub(1);
        let source_line = inline.start_line + line_index;
        let Some(source) = self.source_lines.get(source_line - 1) else {
            return false;
        };
        let source_column = inline.line_columns[line_index].saturating_sub(1)
            + label_start.saturating_sub(inline.line_offsets[line_index]);
        let Some(prefix) = source.get(..source_column) else {
            return false;
        };
        let whitespace = |byte: &u8| matches!(byte, b' ' | b'\t');
        prefix.as_bytes().iter().all(whitespace)
            || prefix
                .strip_suffix('*')
                .is_some_and(|prefix| prefix.as_bytes().iter().all(whitespace))
    }

    /// Return the byte offset of the unescaped `]` that closes a label.
    fn find_reference_label_end(
        inline: &UnresolvedReferenceInline,
        mut offset: usize,
    ) -> Option<usize> {
        let text = &inline.text;
        let bytes = text.as_bytes();
        while offset < bytes.len() {
            if let Some(end) = Self::heading_removed_range_end(inline, offset) {
                offset = end;
                continue;
            }
            match bytes[offset] {
                b'\0' | b'[' => return None,
                b']' if Self::is_opaque_at(inline, offset) => return None,
                b']' => return Some(offset),
                b'\\' => {
                    offset += 1;
                    if offset < bytes.len() {
                        offset += text[offset..]
                            .chars()
                            .next()
                            .map(char::len_utf8)
                            .unwrap_or(1);
                    }
                }
                _ => {
                    offset += text[offset..]
                        .chars()
                        .next()
                        .map(char::len_utf8)
                        .unwrap_or(1);
                }
            }
        }
        None
    }

    fn is_escaped_at(text: &str, offset: usize) -> bool {
        let bytes = text.as_bytes();
        let mut previous = offset;
        let mut slashes = 0;
        while previous > 0 && bytes[previous - 1] == b'\\' {
            previous -= 1;
            slashes += 1;
        }
        slashes % 2 == 1
    }

    /// Render label source in link-text context without serializing the
    /// surrounding document.
    fn render_unresolved_reference_label(&self, raw_label: &str) -> String {
        let source = format!("[{raw_label}](.)");
        let arena = Arena::new();
        let options = crate::comrak_options(self.options);
        let root = parse_document(&arena, &source, &options);
        let link = root.first_child().and_then(|block| {
            block
                .children()
                .find(|child| matches!(&child.data.borrow().value, NodeValue::Link(_)))
        });
        let Some(link) = link else {
            return raw_label.replace('\n', " ");
        };

        let source_lines = source.lines().collect();
        let mut serializer = Serializer::new(self.options, source_lines, false);
        let mut rendered = String::new();
        for child in link.children() {
            serializer.collect_inline_node(child, &mut rendered);
        }
        rendered.retain(|character| {
            character != super::MATH_TOKEN_OPEN && character != super::MATH_TOKEN_CLOSE
        });
        rendered.replace(['\0', '\n'], " ")
    }

    /// Render unresolved labels in the context of their complete inline block.
    ///
    /// Punctuation conversion can depend on text before a label, most notably
    /// for deciding whether a straight quote opens or closes.  Mark all labels
    /// and render the block once so each reserved label matches its eventual
    /// document spelling without introducing one parse per reference.
    fn render_unresolved_inline_reference_labels(
        &self,
        inline_source: &str,
        resolved_reference_definitions: &str,
        labels: &[(usize, usize)],
    ) -> Vec<String> {
        self.render_marked_reference_labels(
            inline_source,
            labels,
            |marked_source| {
                let source = if resolved_reference_definitions.is_empty() {
                    marked_source
                } else {
                    format!("{marked_source}\n\n{resolved_reference_definitions}")
                };
                let arena = Arena::new();
                let options = crate::comrak_options(self.options);
                let root = parse_document(&arena, &source, &options);
                let paragraph = root
                    .children()
                    .find(|child| matches!(&child.data.borrow().value, NodeValue::Paragraph))?;

                let source_lines = source.lines().collect();
                let mut serializer = Serializer::new(self.options, source_lines, false);
                let mut rendered = String::new();
                serializer.collect_inline_content(paragraph, &mut rendered);
                rendered.retain(|character| {
                    character != super::MATH_TOKEN_OPEN && character != super::MATH_TOKEN_CLOSE
                });
                Some(rendered)
            },
            |label| label.replace(['\0', '\n'], " "),
        )
    }

    /// Render an unresolved label in the context of its complete heading.
    ///
    /// Sentence casing depends on text before the label, so rendering the label
    /// alone can capitalize a word that stays lowercase in the real heading.
    fn render_unresolved_heading_reference_labels(
        &self,
        heading_source: &str,
        resolved_reference_definitions: &str,
        labels: &[(usize, usize)],
    ) -> Vec<String> {
        if labels.is_empty() {
            return Vec::new();
        }
        #[cfg(test)]
        HEADING_REFERENCE_RENDER_PASSES.with(|passes| passes.set(passes.get() + 1));
        self.render_marked_reference_labels(
            heading_source,
            labels,
            |marked_source| {
                let source = if resolved_reference_definitions.is_empty() {
                    format!("# {marked_source}")
                } else {
                    format!("# {marked_source}\n\n{resolved_reference_definitions}")
                };
                let arena = Arena::new();
                let options = crate::comrak_options(self.options);
                let root = parse_document(&arena, &source, &options);
                let heading = root
                    .children()
                    .find(|child| matches!(&child.data.borrow().value, NodeValue::Heading(_)))?;

                let source_lines = source.lines().collect();
                let mut serializer = Serializer::new(self.options, source_lines, false);
                let heading_text = serializer.collect_text(heading);
                let (heading_body, _) =
                    super::heading::split_trailing_explicit_anchor(&heading_text);
                if self.options.heading_sentence_case {
                    let mut proper_nouns = self.options.heading_proper_nouns.clone();
                    proper_nouns.extend(self.directive_proper_nouns.clone());
                    let mut common_nouns = self.options.heading_common_nouns.clone();
                    common_nouns.extend(self.directive_common_nouns.clone());
                    Some(super::heading::to_sentence_case(
                        heading_body,
                        &proper_nouns,
                        &common_nouns,
                    ))
                } else {
                    Some(heading_body.to_string())
                }
            },
            str::to_string,
        )
    }

    /// Mark reference labels, render their source in context, and extract the
    /// rendered spellings between the markers.
    fn render_marked_reference_labels(
        &self,
        source: &str,
        labels: &[(usize, usize)],
        render_marked_source: impl FnOnce(String) -> Option<String>,
        normalize_label: impl Fn(&str) -> String,
    ) -> Vec<String> {
        if labels.is_empty() {
            return Vec::new();
        }
        let render_fallback = || {
            labels
                .iter()
                .map(|&(start, end)| {
                    source.get(start..end).map_or_else(String::new, |label| {
                        self.render_unresolved_reference_label(label)
                    })
                })
                .collect()
        };
        let (open_marker, close_marker) = Self::unused_reference_label_markers(source);
        let marker_capacity = labels.len() * (open_marker.len() + close_marker.len());
        let mut marked_source = String::with_capacity(source.len() + marker_capacity);
        let mut source_offset = 0;
        for &(label_start, label_end) in labels {
            let Some(marker_start) = label_start.checked_sub(1) else {
                return render_fallback();
            };
            let Some(marker_end) = label_end.checked_add(1) else {
                return render_fallback();
            };
            if marker_start < source_offset || label_end < label_start || marker_end > source.len()
            {
                return render_fallback();
            }
            marked_source.push_str(&source[source_offset..marker_start]);
            marked_source.push_str(&open_marker);
            marked_source.push_str(&source[marker_start..marker_end]);
            marked_source.push_str(&close_marker);
            source_offset = marker_end;
        }
        marked_source.push_str(&source[source_offset..]);
        marked_source.retain(|character| character != '\0');

        let Some(rendered) = render_marked_source(marked_source) else {
            return render_fallback();
        };
        let mut rendered_labels = Vec::with_capacity(labels.len());
        let mut rendered_offset = 0;
        for _ in labels {
            let Some(open) = rendered[rendered_offset..].find(&open_marker) else {
                return render_fallback();
            };
            let marked_start = rendered_offset + open + open_marker.len();
            let Some(close) = rendered[marked_start..].find(&close_marker) else {
                return render_fallback();
            };
            let marked_end = marked_start + close;
            let Some(label) =
                Self::strip_rendered_reference_brackets(&rendered[marked_start..marked_end])
            else {
                return render_fallback();
            };
            rendered_labels.push(normalize_label(label));
            rendered_offset = marked_end + close_marker.len();
        }
        rendered_labels
    }

    fn strip_rendered_reference_brackets(rendered: &str) -> Option<&str> {
        rendered
            .strip_prefix('[')
            .or_else(|| rendered.strip_prefix("\\["))
            .and_then(|label| label.strip_suffix(']'))
    }

    /// Build two markers that cannot occur in the source.
    fn unused_reference_label_markers(source: &str) -> (String, String) {
        const BMP_START: u32 = 0xe000;
        const BMP_END: u32 = 0xf8ff;
        const PLANE_15_START: u32 = 0xf0000;
        const PLANE_15_END: u32 = 0xffffd;
        const PLANE_16_START: u32 = 0x100000;
        const PLANE_16_END: u32 = 0x10fffd;
        const BMP_COUNT: usize = (BMP_END - BMP_START + 1) as usize;
        const PLANE_COUNT: usize = (PLANE_15_END - PLANE_15_START + 1) as usize;
        const PRIVATE_USE_COUNT: usize = BMP_COUNT + PLANE_COUNT * 2;

        let contains_private_use = source.chars().any(|character| {
            matches!(
                character as u32,
                BMP_START..=BMP_END | PLANE_15_START..=PLANE_15_END | PLANE_16_START..=PLANE_16_END
            )
        });
        if !contains_private_use {
            return (
                char::from_u32(BMP_START).unwrap().to_string(),
                char::from_u32(BMP_START + 1).unwrap().to_string(),
            );
        }

        let private_use_index = |character: char| {
            let value = character as u32;
            match value {
                BMP_START..=BMP_END => Some((value - BMP_START) as usize),
                PLANE_15_START..=PLANE_15_END => {
                    Some(BMP_COUNT + (value - PLANE_15_START) as usize)
                }
                PLANE_16_START..=PLANE_16_END => {
                    Some(BMP_COUNT + PLANE_COUNT + (value - PLANE_16_START) as usize)
                }
                _ => None,
            }
        };
        let private_use_character = |index: usize| {
            let value = if index < BMP_COUNT {
                BMP_START + index as u32
            } else if index < BMP_COUNT + PLANE_COUNT {
                PLANE_15_START + (index - BMP_COUNT) as u32
            } else {
                PLANE_16_START + (index - BMP_COUNT - PLANE_COUNT) as u32
            };
            char::from_u32(value).unwrap()
        };

        let mut used = vec![0_u64; PRIVATE_USE_COUNT.div_ceil(64)];
        for character in source.chars() {
            if let Some(index) = private_use_index(character) {
                used[index / 64] |= 1 << (index % 64);
            }
        }
        let mut unused = (0..PRIVATE_USE_COUNT)
            .filter(|index| used[index / 64] & (1 << (index % 64)) == 0)
            .map(private_use_character);
        if let (Some(open), Some(close)) = (unused.next(), unused.next()) {
            return (open.to_string(), close.to_string());
        }

        // A source containing every private-use scalar is already large.
        // Keep even that pathological fallback linear by finding both maximum
        // runs in a single pass instead of repeatedly rescanning the source.
        let mut open_max = 0;
        let mut open_run = 0;
        let mut close_max = 0;
        let mut close_run = 0;
        for character in source.chars() {
            if character == '\u{e000}' {
                open_run += 1;
                open_max = open_max.max(open_run);
            } else {
                open_run = 0;
            }
            if character == '\u{e001}' {
                close_run += 1;
                close_max = close_max.max(close_run);
            } else {
                close_run = 0;
            }
        }
        (
            std::iter::repeat_n('\u{e000}', open_max + 1).collect(),
            std::iter::repeat_n('\u{e001}', close_max + 1).collect(),
        )
    }

    pub(super) fn serialize_thematic_break(&mut self) {
        let style = self.options.thematic_break_style.as_str();
        let leading_spaces = self.options.thematic_break_leading_spaces.get();

        // Determine the prefix based on blockquote context
        if self.in_block_quote {
            let prefix = format!("{}> ", self.blockquote_outer_indent);
            self.output.push_str(&prefix);
        }

        // Add leading spaces
        for _ in 0..leading_spaces {
            self.output.push(' ');
        }

        self.output.push_str(style);
        self.output.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::{
        HEADING_REFERENCE_RENDER_PASSES, SOURCE_EXCLUSION_RANGE_SCANS, Serializer,
        UNRESOLVED_SOURCE_INDEX_ENTRIES,
    };
    use crate::{Options, format_with_warnings};

    #[test]
    fn test_heading_references_render_in_one_pass() {
        HEADING_REFERENCE_RENDER_PASSES.with(|passes| passes.set(0));
        let labels = (0..64)
            .map(|index| format!("[missing {index}]"))
            .collect::<Vec<_>>()
            .join(" ");
        let result = format_with_warnings(&format!("# {labels}\n"), &Options::default()).unwrap();
        assert_eq!(result.warnings.len(), 64);
        assert_eq!(
            result.warnings.first().unwrap().message,
            "undefined reference link: [missing 0]"
        );
        assert_eq!(
            result.warnings.last().unwrap().message,
            "undefined reference link: [missing 63]"
        );
        HEADING_REFERENCE_RENDER_PASSES.with(|passes| assert_eq!(passes.get(), 1));
    }

    #[test]
    fn test_reference_markers_stay_bounded_for_repeated_pua() {
        let source = format!("{}{}", "\u{e000}".repeat(4_096), "\u{e001}".repeat(4_096));
        let (open, close) = Serializer::unused_reference_label_markers(&source);
        assert!(open.chars().count() <= 2);
        assert!(close.chars().count() <= 2);
    }

    #[test]
    fn test_source_index_scales_with_lines() {
        UNRESOLVED_SOURCE_INDEX_ENTRIES.with(|entries| entries.set(0));
        format_with_warnings(&"x".repeat(64 * 1024), &Options::default()).unwrap();
        UNRESOLVED_SOURCE_INDEX_ENTRIES.with(|entries| assert_eq!(entries.get(), 1));
    }

    #[test]
    fn test_source_definition_exclusions_do_not_scan_ranges() {
        let source = (0..64)
            .map(|index| format!("~~~~ text\n[label {index}]: example\n~~~~\n"))
            .collect::<Vec<_>>()
            .join("\n");
        SOURCE_EXCLUSION_RANGE_SCANS.with(|scans| scans.set(0));
        format_with_warnings(&source, &Options::default()).unwrap();
        SOURCE_EXCLUSION_RANGE_SCANS.with(|scans| assert_eq!(scans.get(), 0));
    }

    /// The text a reference definition carries after its label's colon, and
    /// what of a title it leaves open for the lines below to close.
    #[test]
    fn test_unclosed_title_delimiter() {
        let open = |text: &str| {
            Serializer::title_after_destination(text).and_then(Serializer::unclosed_delimiter_in)
        };

        // Nothing but a destination opens nothing.
        assert_eq!(open(" https://example.com/a"), None);
        assert_eq!(open(" <https://example.com/a>"), None);
        // A destination may hold what a title would open with.
        assert_eq!(open(" <https://example.com/foo\"bar>"), None);
        assert_eq!(open(" <https://example.com/foo(bar>"), None);
        assert_eq!(open(" <https://example.com/foo'bar>"), None);
        // A title closed on the same line needs no continuation.
        assert_eq!(open(" https://example.com/a \"A title\""), None);
        assert_eq!(open(" https://example.com/a (A title)"), None);
        // One left open names the delimiter that will close it.
        assert_eq!(open(" https://example.com/a \"A title"), Some('"'));
        assert_eq!(open(" <https://example.com/a> \"A title"), Some('"'));
        assert_eq!(open(" https://example.com/a 'A title"), Some('\''));
        assert_eq!(open(" https://example.com/a (A title"), Some(')'));
        // An escaped delimiter closes nothing.
        assert_eq!(open(" https://example.com/a \"A \\\"title"), Some('"'));
        // An unterminated `<…>` is no destination at all.
        assert_eq!(open(" <https://example.com/a \"A title"), None);
    }
}
