//! Document-level serialization logic.

use std::sync::LazyLock;

use comrak::nodes::{AstNode, NodeValue};
use regex::Regex;
use unicode_width::UnicodeWidthStr;

use super::Serializer;
use super::escape;
use super::state::{Directive, FormatSkipMode, ReferenceLink, normalize_reference_key};
use super::wrap;

/// Matches a line that begins a reference definition, capturing its label.
///
/// A blockquote prefix may precede the label, since a definition written inside
/// a blockquote still defines a document-wide label.  A list marker may not:
/// `- [x]: done` is a task list item.
static REFERENCE_DEFINITION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[\s>]*\[([^\]]+)\]:").unwrap());

/// A reference definition that a link preserved verbatim depends on.
struct VerbatimReference {
    label: String,
    url: String,
    title: String,
    /// Line of the link that needs it, for reporting.
    line: usize,
}

impl<'a> Serializer<'a> {
    pub(super) fn serialize_document<'b>(&mut self, node: &'b AstNode<'b>) {
        let children: Vec<_> = node.children().collect();

        // Source ranges that are copied verbatim rather than rebuilt from the
        // AST.  Whatever they contain — reference definitions, footnote
        // definitions, comments — travels with the copy and must not be
        // emitted a second time from elsewhere.
        let verbatim_ranges = self.collect_verbatim_line_ranges(&children);
        // Every range whose content keeps its source text, which is the wider
        // set: it also covers the blocks the skip modes emit one by one.
        let disabled_ranges = Self::collect_disabled_line_ranges(node);

        // Both analyses below cost a pass over the document and only say
        // something about content that keeps its source text, so a document
        // without the directives that produce such content skips them.
        if !verbatim_ranges.is_empty() {
            let mut in_leaf_block = vec![false; self.source_lines.len()];
            Self::mark_leaf_block_lines(node, &self.source_lines, &mut in_leaf_block);
            for (label, lines) in
                Self::collect_reference_definition_lines(&self.source_lines, &in_leaf_block)
            {
                // Only the first definition of a label counts, the way
                // CommonMark resolves it.  When it is the one the copy carries,
                // the copy also defines it; when a *later* one is, the copy
                // merely repeats a definition that has no effect where it
                // stands but would take effect if the winning one were emitted
                // after it.
                let mut lines = lines.into_iter();
                let winner = lines.next().unwrap_or_default();
                if Self::is_line_in_ranges(winner, &verbatim_ranges) {
                    self.verbatim_reference_labels.insert(label);
                } else if lines.any(|line| Self::is_line_in_ranges(line, &verbatim_ranges)) {
                    self.shadowed_reference_labels.insert(label);
                }
            }
        }
        if !disabled_ranges.is_empty() {
            self.collect_verbatim_reference_claims(node, &disabled_ranges);
        }

        // Check for undefined reference links using AST
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
                        // The rest of the file keeps its source text, so its
                        // links cannot register the definitions they use.  A
                        // definition placed before the directive has nowhere
                        // else to go, so reserve it ahead of the flush below.
                        for skipped in &children[i + 1..] {
                            self.reserve_references_of(skipped);
                        }

                        // Flush pending footnotes and references BEFORE the disable-file directive.
                        // Definitions that appear before the directive should stay before it.
                        let directive_line = child.data.borrow().sourcepos.start.line;
                        self.flush_footnotes_before(Some(directive_line));
                        self.flush_references();
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
                        self.flush_references();
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
                        self.flush_references();
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
                        let enable_index = Self::find_enable_directive(&children, i);
                        let region_last = enable_index.unwrap_or(children.len());
                        let region_start = child.data.borrow().sourcepos.end.line + 1;
                        let region_end = match enable_index {
                            Some(j) => children[j]
                                .data
                                .borrow()
                                .sourcepos
                                .start
                                .line
                                .saturating_sub(1),
                            None => self.source_lines.len(),
                        };
                        let region = self
                            .extract_source_lines(region_start, region_end)
                            .map(|source| Self::trim_blank_lines(&source))
                            .unwrap_or_default();

                        // Definitions the region's links depend on but that sit
                        // outside it have to be kept from being dropped.
                        let mut region_references = Vec::new();
                        if !region.is_empty() {
                            for skipped in &children[i + 1..region_last] {
                                self.collect_verbatim_references(skipped, &mut region_references);
                            }
                        }
                        // A label the copy redefines has to be reserved before
                        // the flush below, so that its winning definition still
                        // comes first in the output; the copy would otherwise
                        // shadow it and retarget the links.
                        let (shadowed, rest): (Vec<_>, Vec<_>) =
                            region_references.iter().partition(|reference| {
                                self.shadowed_reference_labels
                                    .contains(&normalize_reference_key(&reference.label))
                            });
                        self.reserve_verbatim_references(shadowed.into_iter());

                        // Flush pending footnotes and references BEFORE the disable directive.
                        // Definitions that appear before the directive should stay before it.
                        let directive_line = child.data.borrow().sourcepos.start.line;
                        self.flush_footnotes_before(Some(directive_line));
                        self.flush_references();
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
                            // The rest are reserved after the flush, so they are
                            // emitted where definitions normally go rather than
                            // above the directive.
                            self.reserve_verbatim_references(rest.into_iter());
                            self.output.push('\n');
                            self.output.push_str(&region);
                            self.output.push('\n');
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
                let heading_line = child.data.borrow().sourcepos.start.line;
                // Footnotes come before link reference definitions
                self.flush_footnotes_before(Some(heading_line));
                self.flush_references();
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

                // The block keeps its source text, so its links never register
                // the definitions they use; reserve them here instead.
                self.reserve_references_of(child);

                // Output the original source
                if let Some(source) = self.extract_source(child) {
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
                    let enable_index = Self::find_enable_directive(children, i);
                    let end_line = match enable_index {
                        Some(j) => children[j]
                            .data
                            .borrow()
                            .sourcepos
                            .start
                            .line
                            .saturating_sub(1),
                        None => last_line,
                    };
                    ranges.push((start_line, end_line));
                    // Nested directives inside the region are part of the copy.
                    i = enable_index.unwrap_or(children.len());
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
            let start = if let NodeValue::Paragraph = &data.value {
                drop(data);
                Self::paragraph_content_start(
                    node,
                    source_lines,
                    sourcepos.start.line,
                    sourcepos.end.line,
                )
            } else {
                sourcepos.start.line
            };
            for line in start..=sourcepos.end.line {
                if let Some(marked) = in_leaf_block.get_mut(line - 1) {
                    *marked = true;
                }
            }
            return;
        }
        drop(data);

        for child in node.children() {
            Self::mark_leaf_block_lines(child, source_lines, in_leaf_block);
        }
    }

    /// The first line of a paragraph's own content.
    ///
    /// Definitions are consumed from the head of a paragraph, and the node
    /// keeps the lines they occupied, so those lines have to stay visible to
    /// the definition scanner while the paragraph's own lines do not.
    ///
    /// Two readings bound where the content starts, and a line is left visible
    /// only where they agree.  The content sits at the end of the span, one
    /// line per break it contains, which misjudges a paragraph whose inline
    /// spans lines without a break node, such as a code span or display math
    /// holding a newline.  A consumed head is also made of definitions, so the
    /// first line that does not begin one ends it, which instead misjudges a
    /// paragraph opening with something that merely resembles a definition.
    /// Neither mistake survives the other.
    fn paragraph_content_start<'b>(
        node: &'b AstNode<'b>,
        source_lines: &[&str],
        start_line: usize,
        end_line: usize,
    ) -> usize {
        let by_breaks = end_line
            .saturating_sub(Self::count_line_breaks(node))
            .max(start_line);

        let mut by_definitions = start_line;
        while by_definitions < end_line {
            let Some(line) = source_lines.get(by_definitions - 1) else {
                break;
            };
            let Some(matched) = REFERENCE_DEFINITION.find(line) else {
                break;
            };
            by_definitions += 1;
            // A label with nothing after its colon takes its destination from
            // the line below, which the definitions after it sit beneath.  A
            // title carried onto further lines is not followed this way, and
            // leaves them counted as content.
            if line[matched.end()..].trim().is_empty() && by_definitions < end_line {
                by_definitions += 1;
            }
        }

        by_breaks.min(by_definitions)
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
        source_lines: &[&str],
        in_leaf_block: &[bool],
    ) -> std::collections::HashMap<String, Vec<usize>> {
        let mut definitions: std::collections::HashMap<String, Vec<usize>> =
            std::collections::HashMap::new();

        for (i, line) in source_lines.iter().enumerate() {
            let line_number = i + 1;
            if in_leaf_block.get(i).copied().unwrap_or(false) {
                continue;
            }
            if let Some(caps) = REFERENCE_DEFINITION.captures(line)
                && let Some(label) = caps.get(1)
                // Footnote definitions are not reference definitions.
                && !label.as_str().starts_with('^')
            {
                definitions
                    .entry(normalize_reference_key(label.as_str()))
                    .or_default()
                    .push(line_number);
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
        disabled_ranges: &[(usize, usize)],
    ) {
        for child in node.children() {
            self.collect_verbatim_reference_claims(child, disabled_ranges);
        }

        let (target, line) = {
            let data = node.data.borrow();
            let target = match &data.value {
                NodeValue::Link(link) | NodeValue::Image(link) => {
                    Some((link.url.clone(), link.title.clone()))
                }
                _ => None,
            };
            (target, data.sourcepos.start.line)
        };

        if let Some((url, title)) = target
            && Self::is_line_in_ranges(line, disabled_ranges)
            && let Some(label) = self.verbatim_reference_label(node)
        {
            self.verbatim_reference_claims
                .entry(normalize_reference_key(&label))
                .or_insert(ReferenceLink { label, url, title });
        }
    }

    /// The label a reference-style link or image is written with in the source,
    /// with the collapsed-reference marker stripped.  `None` for links that are
    /// not reference style, and for the empty label, which cannot be defined.
    fn verbatim_reference_label<'b>(&self, node: &'b AstNode<'b>) -> Option<String> {
        let (_, label) = self.get_reference_style_info(node)?;
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
        collected: &mut Vec<VerbatimReference>,
    ) {
        // Children first, so that a badge's image comes before the link
        // wrapping it, the order the formatted path would register them in.
        for child in node.children() {
            self.collect_verbatim_references(child, collected);
        }

        let (target, line) = {
            let data = node.data.borrow();
            let target = match &data.value {
                NodeValue::Link(link) | NodeValue::Image(link) => {
                    Some((link.url.clone(), link.title.clone()))
                }
                _ => None,
            };
            (target, data.sourcepos.start.line)
        };

        if let Some((url, title)) = target
            && let Some(label) = self.verbatim_reference_label(node)
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
            let key = normalize_reference_key(&reference.label);
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

    /// Collect and reserve in one step, for a single block emitted verbatim.
    fn reserve_references_of<'b>(&mut self, node: &'b AstNode<'b>) {
        let mut references = Vec::new();
        self.collect_verbatim_references(node, &mut references);
        self.reserve_verbatim_references(references.iter());
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

        // Collect PHP Markdown Extra abbreviation definitions from source
        let abbreviations = Self::collect_abbreviations(&self.source_lines);

        // Collect reference definitions from source that comrak may not have parsed
        // (e.g., when they follow abbreviation definitions without a blank line)
        let source_ref_defs = Self::collect_source_reference_definitions(&self.source_lines);

        // Collect warnings first to avoid borrow issues
        let warnings = Self::find_undefined_references_in_ast(
            node,
            &self.source_lines,
            &abbreviations,
            &source_ref_defs,
        );

        // Filter out warnings that fall within disabled regions
        for (line, msg) in warnings {
            if !Self::is_line_in_ranges(line, disabled_ranges) {
                self.add_warning(line, msg);
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
        ranges
            .iter()
            .any(|(start, end)| line >= *start && line <= *end)
    }

    /// Collect PHP Markdown Extra abbreviation definitions from source.
    /// Returns a set of abbreviation names (e.g., "HTML" from "*[HTML]: Hyper Text Markup Language").
    fn collect_abbreviations(source_lines: &[&str]) -> std::collections::HashSet<String> {
        let mut abbreviations = std::collections::HashSet::new();
        let abbr_pattern = Regex::new(r"^\*\[([^\]]+)\]:").unwrap();

        for line in source_lines {
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
    ) -> std::collections::HashSet<String> {
        let mut definitions = std::collections::HashSet::new();
        // Pattern: [label]: URL at start of line (with optional leading whitespace)
        let ref_def_pattern = Regex::new(r"^\s*\[([^\]]+)\]:\s*\S").unwrap();

        for line in source_lines {
            if let Some(caps) = ref_def_pattern.captures(line)
                && let Some(label) = caps.get(1)
            {
                definitions.insert(label.as_str().to_string());
            }
        }

        definitions
    }

    /// Find undefined references by walking the AST.
    /// Returns a vector of (line_number, warning_message) tuples.
    fn find_undefined_references_in_ast<'b>(
        node: &'b AstNode<'b>,
        source_lines: &[&str],
        abbreviations: &std::collections::HashSet<String>,
        source_ref_defs: &std::collections::HashSet<String>,
    ) -> Vec<(usize, String)> {
        let mut warnings = Vec::new();

        // Pattern to find [label] or [text][label] in text nodes
        // This matches text that looks like a reference link but wasn't parsed as one
        // The pattern [^\[\]] ensures the label doesn't start with [ or ]
        let ref_pattern = Regex::new(r"\[([^\[\]][^\]]*)\](?:\[([^\]]*)\])?").unwrap();

        Self::walk_ast_for_undefined_refs(
            node,
            source_lines,
            &ref_pattern,
            abbreviations,
            source_ref_defs,
            &mut warnings,
        );

        warnings
    }

    /// Recursively walk the AST looking for undefined references in Text nodes.
    fn walk_ast_for_undefined_refs<'b>(
        node: &'b AstNode<'b>,
        source_lines: &[&str],
        ref_pattern: &Regex,
        abbreviations: &std::collections::HashSet<String>,
        source_ref_defs: &std::collections::HashSet<String>,
        warnings: &mut Vec<(usize, String)>,
    ) {
        let data = node.data.borrow();

        match &data.value {
            NodeValue::Text(text) => {
                // Look for [label] patterns in text content
                let line_num = data.sourcepos.start.line;

                for caps in ref_pattern.captures_iter(text) {
                    let full_match = caps.get(0).unwrap();
                    let label = if let Some(explicit_label) = caps.get(2) {
                        // [text][label] form - use the explicit label
                        let l = explicit_label.as_str();
                        if l.is_empty() {
                            // [text][] form - use the text as label
                            caps.get(1).map(|m| m.as_str()).unwrap_or("")
                        } else {
                            l
                        }
                    } else {
                        // [text] form - use the text as label
                        caps.get(1).map(|m| m.as_str()).unwrap_or("")
                    };

                    // Skip empty labels
                    if label.is_empty() {
                        continue;
                    }

                    // Skip footnote references [^name]
                    if label.starts_with('^') {
                        continue;
                    }

                    // Skip GitHub alert markers [!NOTE], [!TIP], etc.
                    if label.starts_with('!') {
                        continue;
                    }

                    // Skip PHP Markdown Extra abbreviations
                    if abbreviations.contains(label) {
                        continue;
                    }

                    // Skip reference definitions that exist in source but comrak didn't parse
                    // (e.g., when they follow abbreviation definitions without a blank line)
                    if source_ref_defs.contains(label) {
                        continue;
                    }

                    // Check original source to see if this was escaped
                    if Self::is_escaped_in_source(source_lines, line_num, full_match.as_str()) {
                        continue;
                    }

                    warnings.push((line_num, format!("undefined reference link: [{}]", label)));
                }
            }
            // Skip code blocks and inline code - they don't contain reference links
            NodeValue::CodeBlock(_) | NodeValue::Code(_) => {
                return;
            }
            // Skip other leaf nodes that don't contain text we care about
            NodeValue::HtmlBlock(_) | NodeValue::HtmlInline(_) => {
                return;
            }
            _ => {}
        }

        drop(data);

        // Recurse into children
        for child in node.children() {
            Self::walk_ast_for_undefined_refs(
                child,
                source_lines,
                ref_pattern,
                abbreviations,
                source_ref_defs,
                warnings,
            );
        }
    }

    /// Check if a bracket pattern was escaped in the original source.
    /// Returns true if the pattern appears as `\[...]` in the source.
    fn is_escaped_in_source(source_lines: &[&str], line_num: usize, pattern: &str) -> bool {
        if line_num == 0 || line_num > source_lines.len() {
            return false;
        }

        let line = source_lines[line_num - 1];

        // Look for the pattern in the line and check if it's preceded by backslash
        if let Some(pos) = line.find(pattern)
            && pos > 0
        {
            let bytes = line.as_bytes();
            // Check if preceded by backslash (and not double backslash)
            if bytes[pos - 1] == b'\\' && (pos < 2 || bytes[pos - 2] != b'\\') {
                return true;
            }
        }

        false
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
