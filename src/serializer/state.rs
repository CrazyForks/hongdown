//! Serializer state and common types.

use indexmap::IndexMap;

use comrak::nodes::{AstNode, ListType, NodeValue};

use crate::Options;

/// The current formatting skip mode.
///
/// Controls whether and how formatting should be skipped for content.
/// Only one mode can be active at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FormatSkipMode {
    /// Normal formatting is active.
    #[default]
    None,
    /// Skip formatting for the next block element only.
    /// Automatically resets to `None` after the block is processed.
    NextBlock,
    /// Skip formatting until the next section heading (h2 or lower).
    /// Automatically resets to `None` when a heading is encountered.
    UntilSection,
    /// Formatting is disabled (by `hongdown-disable` directive).
    /// Remains active until `hongdown-enable` directive is encountered.
    ///
    /// Only used when the original source is unavailable: with a source the
    /// whole region is copied from it in one piece instead.
    Disabled,
}

/// Formatting directives that can be embedded in HTML comments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Directive {
    /// Disable formatting for the next block element only.
    DisableNextLine,
    /// Disable formatting for the entire file.
    DisableFile,
    /// Disable formatting for the next section (until next heading).
    DisableNextSection,
    /// Disable formatting from this point until `Enable`.
    Disable,
    /// Re-enable formatting after `Disable`.
    Enable,
    /// Define proper nouns for sentence case (case-sensitive).
    ProperNouns(Vec<String>),
    /// Define common nouns for sentence case (case-sensitive).
    CommonNouns(Vec<String>),
}

impl Directive {
    /// Parse a directive from an HTML comment.
    /// Returns `Some(Directive)` if the comment contains a valid directive.
    pub fn parse(html: &str) -> Option<Self> {
        let trimmed = html.trim();
        // Check if it's an HTML comment
        if !trimmed.starts_with("<!--") || !trimmed.ends_with("-->") {
            return None;
        }
        // Extract the content between <!-- and -->
        let content = trimmed.strip_prefix("<!--")?.strip_suffix("-->")?.trim();

        // Check for directives without arguments
        match content {
            "hongdown-disable-next-line" => return Some(Directive::DisableNextLine),
            "hongdown-disable-file" => return Some(Directive::DisableFile),
            "hongdown-disable-next-section" => return Some(Directive::DisableNextSection),
            "hongdown-disable" => return Some(Directive::Disable),
            "hongdown-enable" => return Some(Directive::Enable),
            _ => {}
        }

        // Check for directives with arguments
        if let Some(args) = content.strip_prefix("hongdown-proper-nouns:") {
            let nouns = args
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            return Some(Directive::ProperNouns(nouns));
        }

        if let Some(args) = content.strip_prefix("hongdown-common-nouns:") {
            let nouns = args
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            return Some(Directive::CommonNouns(nouns));
        }

        None
    }
}

/// A reference link definition: label -> (url, title)
#[derive(Debug, Clone)]
pub struct ReferenceLink {
    pub label: String,
    pub url: String,
    pub title: String,
}

/// The longest reference label a CommonMark parser accepts, in bytes.
///
/// Matches comrak's own `MAX_LINK_LABEL_LENGTH`; a longer label makes the
/// parser reject the whole link, so a generated label must stay within it.
const MAX_REFERENCE_LABEL_BYTES: usize = 1000;

/// Normalize a reference label into the key used to look it up.
///
/// CommonMark matches reference labels after collapsing internal whitespace
/// and applying Unicode default case folding, so labels that differ only in
/// spacing or case refer to the same definition and must share a single key.
/// This mirrors comrak's `normalize_label(_, Case::Fold)`, down to using the
/// same case folding implementation: plain lowercasing would miss pairs such
/// as `Straße` and `STRASSE`.  The internal SoftBreak marker is treated as
/// the space it will be emitted as.
pub fn normalize_reference_key(label: &str) -> String {
    caseless::default_case_fold_str(&super::escape::normalize_whitespace(
        &label.replace('\x00', " "),
    ))
}

/// The longest suffix [`numbered_reference_label`] appends, in bytes: a space
/// plus a `u32` in decimal.
const MAX_REFERENCE_LABEL_SUFFIX_BYTES: usize = 11;

/// Shorten a label, at a character boundary, so that any numbered suffix still
/// fits within [`MAX_REFERENCE_LABEL_BYTES`].  Labels that are already short
/// enough are returned unchanged.
///
/// An over-long label is rejected by the parser on the next pass, turning the
/// link into literal text and losing its destination.  The amount removed does
/// not depend on the number, so every numbered variant of a given label shares
/// one namespace; a number-dependent cut would let labels that shorten to the
/// same prefix allocate against each other unnoticed.
fn truncate_reference_label_base(label: &str) -> &str {
    let budget = MAX_REFERENCE_LABEL_BYTES - MAX_REFERENCE_LABEL_SUFFIX_BYTES;
    if label.len() <= budget {
        return label;
    }
    let mut end = budget;
    while end > 0 && !label.is_char_boundary(end) {
        end -= 1;
    }
    &label[..end]
}

/// Build the `n`th numbered variant of `label`, such as `guide 2`.
fn numbered_reference_label(label: &str, n: u32) -> String {
    format!("{} {n}", truncate_reference_label_base(label))
}

/// A footnote definition: name -> content
#[derive(Debug, Clone)]
pub struct FootnoteDefinition {
    pub name: String,
    pub content: String,
    /// Line number where the footnote was referenced (1-indexed)
    pub reference_line: usize,
    /// Whether the footnote contains block elements (code blocks, etc.)
    /// When true, content contains pre-serialized block content with proper indentation.
    pub has_blocks: bool,
}

/// Manages footnote definitions and their reference tracking.
///
/// This struct encapsulates all state related to footnote processing:
/// - Pending footnote definitions waiting to be emitted
/// - Tracking which footnotes have been emitted
/// - Line numbers where footnotes are referenced
/// - Reference links found within footnote content
#[derive(Debug, Default)]
pub struct FootnoteSet {
    /// Footnote definitions collected for the current section.
    /// Key: name, Value: FootnoteDefinition (insertion order preserved)
    pub pending: IndexMap<String, FootnoteDefinition>,
    /// Footnote names that have already been emitted (to avoid duplicates)
    pub emitted: std::collections::HashSet<String>,
    /// Line numbers where footnotes were referenced (key: footnote name)
    pub reference_lines: std::collections::HashMap<String, usize>,
    /// Whether we're currently collecting footnote content.
    /// When true, reference links are added to `pending_references` instead of
    /// the main reference collection.
    pub collecting_content: bool,
    /// The reference line of the footnote currently being collected.
    /// Used to associate references with their parent footnote's timing.
    pub current_reference_line: usize,
    /// Reference links collected from within footnote definitions.
    /// Key: normalized label (see `normalize_reference_key`).
    /// Value is (ReferenceLink, footnote_reference_line) to track when to flush.
    pub pending_references: IndexMap<String, (ReferenceLink, usize)>,
}

impl FootnoteSet {
    /// Create a new empty FootnoteSet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a footnote definition.
    pub fn add(&mut self, name: String, content: String, reference_line: usize, has_blocks: bool) {
        self.pending.insert(
            name.clone(),
            FootnoteDefinition {
                name,
                content,
                reference_line,
                has_blocks,
            },
        );
    }

    /// Record the line where a footnote is referenced.
    pub fn record_reference_line(&mut self, name: String, line: usize) {
        self.reference_lines.entry(name).or_insert(line);
    }

    /// Get the reference line for a footnote.
    pub fn get_reference_line(&self, name: &str) -> Option<usize> {
        self.reference_lines.get(name).copied()
    }

    /// Start collecting content for a footnote.
    pub fn start_collecting(&mut self, reference_line: usize) {
        self.collecting_content = true;
        self.current_reference_line = reference_line;
    }

    /// Stop collecting content for a footnote.
    pub fn stop_collecting(&mut self) {
        self.collecting_content = false;
        self.current_reference_line = 0;
    }

    /// Add a reference link found within footnote content, under its
    /// normalized lookup key.
    pub fn add_reference(&mut self, key: String, reference: ReferenceLink) {
        self.pending_references
            .insert(key, (reference, self.current_reference_line));
    }
}

/// A warning generated during formatting.
#[derive(Debug, Clone)]
pub struct Warning {
    /// Line number where the issue was detected (1-indexed)
    pub line: usize,
    /// Warning message
    pub message: String,
}

/// Safely slice a string, ensuring the indices are valid UTF-8 boundaries.
/// If the indices are not valid boundaries, adjusts to the nearest valid boundary.
fn safe_str_slice(s: &str, start: usize, end: usize) -> &str {
    let safe_start = if start >= s.len() {
        s.len()
    } else if s.is_char_boundary(start) {
        start
    } else {
        // Find the previous valid boundary
        (0..start)
            .rev()
            .find(|&i| s.is_char_boundary(i))
            .unwrap_or(0)
    };

    let safe_end = if end >= s.len() {
        s.len()
    } else if s.is_char_boundary(end) {
        end
    } else {
        // Find the next valid boundary
        (end..=s.len())
            .find(|&i| s.is_char_boundary(i))
            .unwrap_or(s.len())
    };

    &s[safe_start..safe_end]
}

/// Code formatter callback type for WASM builds.
///
/// The callback receives the language identifier and code content,
/// and should return the formatted code (or `None` to keep original).
#[cfg(feature = "wasm")]
pub type CodeFormatterCallback = Option<Box<dyn Fn(&str, &str) -> Option<String>>>;

/// The main serializer state for converting comrak AST to formatted Markdown.
pub struct Serializer<'a> {
    pub output: String,
    pub options: &'a Options,
    /// Original source lines for extracting unformatted content
    pub source_lines: Vec<&'a str>,
    /// Current list item index (1-based) for ordered lists
    pub list_item_index: usize,
    /// Current list type
    pub list_type: Option<ListType>,
    /// Whether the current list is tight (no blank lines between items)
    pub list_tight: bool,
    /// Whether we're inside a block quote
    pub in_block_quote: bool,
    /// Accumulated blockquote prefix for nested blockquotes (e.g., "> " or "> > ")
    pub blockquote_prefix: String,
    /// Reference links collected for the current section
    /// Key: normalized label (see `normalize_reference_key`),
    /// Value: ReferenceLink (insertion order preserved)
    pub pending_references: IndexMap<String, ReferenceLink>,
    /// Reference links that have already been emitted (to avoid duplicates)
    /// Key: normalized label, Value: the target it was emitted with
    pub emitted_references: std::collections::HashMap<String, ReferenceLink>,
    /// The next numbered variant to try for a label whose earlier variants are
    /// all taken.  Key: normalized label of the base.
    pub reference_label_cursors: std::collections::HashMap<String, u32>,
    /// Numbered variants already handed out, so that a target appearing again
    /// reuses its variant instead of being given another one.
    /// Key: (normalized label of the base, url, title)
    pub numbered_reference_labels: std::collections::HashMap<(String, String, String), String>,
    /// Footnote definitions and their reference tracking
    pub footnotes: FootnoteSet,
    /// Normalized keys (see `normalize_reference_key`) of the definitions that
    /// are copied verbatim along with a disabled region, and so must not be
    /// reserved and emitted a second time.
    pub verbatim_reference_labels: std::collections::HashSet<String>,
    /// Labels claimed by links that are preserved verbatim, keyed by their
    /// normalized key.  Such a link cannot be relabelled, so it holds its label
    /// against formatted links, which can be given a derived one instead.
    pub verbatim_reference_claims: std::collections::HashMap<String, ReferenceLink>,
    /// Normalized keys of the labels a verbatim copy redefines: their winning
    /// definition lies outside it, so it has to be emitted ahead of the copy or
    /// the copy would shadow it.
    pub shadowed_reference_labels: std::collections::HashSet<String>,
    /// Definitions that copied links depend on, each with the line its
    /// definition sits on in the source, which is when it falls due.
    pub deferred_references: Vec<(ReferenceLink, usize)>,
    /// Normalized keys of the reference definitions the source carries, mapped
    /// to the line of the one the parser resolves.
    pub reference_definition_lines: std::collections::HashMap<String, usize>,
    /// Current list nesting depth (0 = not in list, 1 = top-level, 2+ = nested)
    pub list_depth: usize,
    /// Current formatting skip mode
    pub skip_mode: FormatSkipMode,
    /// Whether we're inside a description details block (for indentation)
    pub in_description_details: bool,
    /// Whether we're serializing the first list inside description details on the same line as `:`.
    /// When true, the first list item should not have base indentation (only marker).
    pub description_details_first_list: bool,
    /// Warnings generated during formatting
    pub warnings: Vec<Warning>,
    /// Maximum number of items in the current ordered list (for padding calculation)
    pub ordered_list_max_items: usize,
    /// Whether the original source ends with a newline
    pub source_ends_with_newline: bool,
    /// Current indentation prefix for list item content (e.g., "     " for ` 1.  `)
    /// Used by blockquotes and other block elements inside list items.
    pub list_item_indent: String,
    /// Indentation prefix for content inside a blockquote that's nested inside a list.
    /// This is the outer list's indent that should appear before each `>` in the blockquote.
    pub blockquote_outer_indent: String,
    /// The list depth when entering the current blockquote.
    /// Used to determine if a list exists inside vs outside the blockquote.
    pub blockquote_entry_list_depth: usize,
    /// Display width already emitted on the current paragraph's first line.
    /// Used when wrapping paragraph text after prefixes were written separately.
    pub paragraph_first_line_prefix_width: usize,
    /// Proper nouns defined via directives for sentence case (merged with config)
    pub directive_proper_nouns: Vec<String>,
    /// Common nouns defined via directives for sentence case (merged with config)
    pub directive_common_nouns: Vec<String>,
    /// Code formatter callback for WASM builds.
    #[cfg(feature = "wasm")]
    pub code_formatter_callback: CodeFormatterCallback,
}

impl<'a> Serializer<'a> {
    pub fn new(
        options: &'a Options,
        source_lines: Vec<&'a str>,
        source_ends_with_newline: bool,
    ) -> Self {
        Self {
            output: String::new(),
            options,
            source_lines,
            list_item_index: 0,
            list_type: None,
            list_tight: true,
            in_block_quote: false,
            blockquote_prefix: String::new(),
            pending_references: IndexMap::new(),
            emitted_references: std::collections::HashMap::new(),
            reference_label_cursors: std::collections::HashMap::new(),
            numbered_reference_labels: std::collections::HashMap::new(),
            footnotes: FootnoteSet::new(),
            verbatim_reference_labels: std::collections::HashSet::new(),
            verbatim_reference_claims: std::collections::HashMap::new(),
            shadowed_reference_labels: std::collections::HashSet::new(),
            deferred_references: Vec::new(),
            reference_definition_lines: std::collections::HashMap::new(),
            list_depth: 0,
            skip_mode: FormatSkipMode::None,
            in_description_details: false,
            description_details_first_list: false,
            warnings: Vec::new(),
            ordered_list_max_items: 0,
            source_ends_with_newline,
            list_item_indent: String::new(),
            blockquote_outer_indent: String::new(),
            blockquote_entry_list_depth: 0,
            paragraph_first_line_prefix_width: 0,
            directive_proper_nouns: Vec::new(),
            directive_common_nouns: Vec::new(),
            #[cfg(feature = "wasm")]
            code_formatter_callback: None,
        }
    }

    /// Create a new serializer with a code formatter callback (WASM only).
    #[cfg(feature = "wasm")]
    pub fn with_code_formatter_callback(
        options: &'a Options,
        source_lines: Vec<&'a str>,
        source_ends_with_newline: bool,
        callback: CodeFormatterCallback,
    ) -> Self {
        Self {
            output: String::new(),
            options,
            source_lines,
            list_item_index: 0,
            list_type: None,
            list_tight: true,
            in_block_quote: false,
            blockquote_prefix: String::new(),
            pending_references: IndexMap::new(),
            emitted_references: std::collections::HashMap::new(),
            reference_label_cursors: std::collections::HashMap::new(),
            numbered_reference_labels: std::collections::HashMap::new(),
            footnotes: FootnoteSet::new(),
            verbatim_reference_labels: std::collections::HashSet::new(),
            verbatim_reference_claims: std::collections::HashMap::new(),
            shadowed_reference_labels: std::collections::HashSet::new(),
            deferred_references: Vec::new(),
            reference_definition_lines: std::collections::HashMap::new(),
            list_depth: 0,
            skip_mode: FormatSkipMode::None,
            in_description_details: false,
            description_details_first_list: false,
            warnings: Vec::new(),
            ordered_list_max_items: 0,
            source_ends_with_newline,
            list_item_indent: String::new(),
            blockquote_outer_indent: String::new(),
            blockquote_entry_list_depth: 0,
            paragraph_first_line_prefix_width: 0,
            directive_proper_nouns: Vec::new(),
            directive_common_nouns: Vec::new(),
            code_formatter_callback: callback,
        }
    }

    /// Add a warning.
    pub fn add_warning(&mut self, line: usize, message: String) {
        self.warnings.push(Warning { line, message });
    }

    /// Extract original source text for a node using its sourcepos.
    pub fn extract_source<'b>(&self, node: &'b AstNode<'b>) -> Option<String> {
        self.extract_source_shifted(node, 0)
    }

    /// Extract original source text for a node whose reported lines sit
    /// `line_offset` above its real ones.
    ///
    /// An inline inside a paragraph that swallowed a definition at its head is
    /// reported against the paragraph's own start, so its lines are short by
    /// however many lines that definition took.  Its columns are the ones its
    /// real line has.
    pub fn extract_source_shifted<'b>(
        &self,
        node: &'b AstNode<'b>,
        line_offset: usize,
    ) -> Option<String> {
        if self.source_lines.is_empty() {
            return None;
        }
        let sourcepos = node.data.borrow().sourcepos;
        let start_line = sourcepos.start.line + line_offset;
        let end_line = sourcepos.end.line + line_offset;
        let start_col = sourcepos.start.column;
        let end_col = sourcepos.end.column;

        if start_line == 0 || end_line == 0 {
            return None;
        }

        // Lines and columns are 1-indexed in sourcepos
        let start_idx = start_line - 1;
        let end_idx = end_line - 1;

        if end_idx >= self.source_lines.len() {
            return None;
        }

        let mut result = String::new();
        for i in start_idx..=end_idx {
            if i > start_idx {
                result.push('\n');
            }
            let line = self.source_lines[i];
            if start_idx == end_idx {
                // Single line: extract from start_col to end_col
                let start_byte = start_col.saturating_sub(1);
                let end_byte = end_col;
                result.push_str(safe_str_slice(line, start_byte, end_byte));
            } else if i == start_idx {
                // First line: from start_col to end
                let start_byte = start_col.saturating_sub(1);
                result.push_str(safe_str_slice(line, start_byte, line.len()));
            } else if i == end_idx {
                // Last line: from start to end_col
                let end_byte = end_col.min(line.len());
                result.push_str(safe_str_slice(line, 0, end_byte));
            } else {
                // Middle lines: full line
                result.push_str(line);
            }
        }
        Some(result)
    }

    /// Extract original source text for an inclusive range of lines.
    /// Line numbers are 1-indexed; `end_line` is clamped to the last line.
    /// Returns `None` when there is no source or the range is empty.
    pub fn extract_source_lines(&self, start_line: usize, end_line: usize) -> Option<String> {
        if self.source_lines.is_empty() || start_line == 0 || end_line < start_line {
            return None;
        }
        let start_idx = start_line - 1;
        if start_idx >= self.source_lines.len() {
            return None;
        }
        let end_idx = (end_line - 1).min(self.source_lines.len() - 1);
        Some(self.source_lines[start_idx..=end_idx].join("\n"))
    }

    /// Extract original source text from a given line to the end of the file.
    /// Line numbers are 1-indexed.
    pub fn extract_source_from_line(&self, start_line: usize) -> Option<String> {
        if self.source_lines.is_empty() || start_line == 0 {
            return None;
        }
        let start_idx = start_line - 1;
        if start_idx >= self.source_lines.len() {
            return None;
        }
        let mut result = String::new();
        for (i, line) in self.source_lines.iter().enumerate().skip(start_idx) {
            if i > start_idx {
                result.push('\n');
            }
            result.push_str(line);
        }
        // Preserve trailing newline if the original source had one
        if self.source_ends_with_newline {
            result.push('\n');
        }
        Some(result)
    }

    /// Check if formatting should be skipped for this node.
    pub fn should_skip_formatting(&self) -> bool {
        self.skip_mode != FormatSkipMode::None
    }

    /// Look up a reference that has already been registered under `key`,
    /// whether it is still pending or has already been emitted.
    fn find_reference(&self, key: &str) -> Option<&ReferenceLink> {
        self.emitted_references
            .get(key)
            .or_else(|| self.pending_references.get(key))
            .or_else(|| self.footnotes.pending_references.get(key).map(|(r, _)| r))
    }

    /// Look up whatever holds `key`, including a label merely claimed by a link
    /// that is preserved verbatim.  A claim counts as occupying the label even
    /// before the definition is reserved, because the claiming link cannot be
    /// relabelled and so must not have its label taken from under it.
    fn find_occupant(&self, key: &str) -> Option<&ReferenceLink> {
        self.find_reference(key)
            .or_else(|| self.verbatim_reference_claims.get(key))
    }

    /// Whether a verbatim copy carries a definition for `key`.
    ///
    /// Such a definition holds the label in the output whether or not anything
    /// resolves through it, and whether it is the definition the parser
    /// resolved or a duplicate the copy repeats.  So the label is spoken for
    /// even when nothing reveals what it points at, and a link that took it
    /// would find its own definition placed after the copy, where CommonMark's
    /// first-wins rule would hand it the copy's destination instead.
    fn is_label_carried_verbatim(&self, key: &str) -> bool {
        self.verbatim_reference_labels.contains(key) || self.shadowed_reference_labels.contains(key)
    }

    /// Register a reference link and return the label that must be used to
    /// refer to it.
    ///
    /// A label may only be shared by links whose complete target (both URL and
    /// title) is identical; sharing it otherwise would silently change one of
    /// the links' destinations.  When the desired label is already taken by a
    /// different target, a distinct label is derived from it by appending a
    /// number, and the caller must fall back to full reference syntax.
    ///
    /// If collecting_footnote_content is true, the reference is added to
    /// pending_footnote_references instead, along with the current footnote's
    /// reference line for proper flush timing.
    pub fn register_reference(&mut self, label: &str, url: &str, title: &str) -> String {
        let key = normalize_reference_key(label);
        // Copy the occupant out of the borrow so the maps below can be updated.
        let occupant = self
            .find_occupant(&key)
            .map(|existing| (existing.url.clone(), existing.title.clone()));
        match occupant {
            // Already registered with the same target: share it.  The existing
            // entry is left untouched so that its spelling (and, once emitted,
            // its definition) is not duplicated or altered.
            Some((occupied_url, occupied_title))
                if occupied_url == url && occupied_title == title =>
            {
                self.satisfy_claimed_label(key, label, url, title);
                label.to_string()
            }
            // Free label: take it.
            None if !self.is_label_carried_verbatim(&key) => {
                self.insert_reference(key, label.to_string(), url, title);
                label.to_string()
            }
            // Taken: by a different target, or by a definition that a verbatim
            // copy carries.  A carried definition holds its label document-wide
            // whether or not a link resolves through it, and its target is
            // unknown when none does, so its label cannot be shared either.
            _ => {
                // The variants live in the namespace of the shortened base, so
                // that is what both the cursor and the reuse map are keyed by:
                // labels long enough to be shortened to the same prefix share
                // one namespace and must not allocate against each other.
                let cursor_key = normalize_reference_key(truncate_reference_label_base(label));
                // Reuse the variant this target was already given, if any.
                // Looking it up directly keeps a document full of same-text
                // links from rescanning every variant it has handed out.
                let target = (cursor_key.clone(), url.to_string(), title.to_string());
                if let Some(label) = self.numbered_reference_labels.get(&target) {
                    return label.clone();
                }
                // Every variant below the cursor is already taken, and taken
                // labels are never released, so the search can resume there.
                let start = self
                    .reference_label_cursors
                    .get(&cursor_key)
                    .copied()
                    .unwrap_or(2);
                for n in start.. {
                    let candidate = numbered_reference_label(label, n);
                    let candidate_key = normalize_reference_key(&candidate);
                    // Copy the occupant out of the borrow so the maps below can
                    // be updated.
                    let occupant = self
                        .find_occupant(&candidate_key)
                        .map(|existing| (existing.url.clone(), existing.title.clone()));
                    match occupant {
                        // Occupied by something else.  Remember what, because
                        // the cursor will not visit this variant again and a
                        // later link to that same target must find it here
                        // rather than allocating a duplicate for it.
                        Some((occupied_url, occupied_title))
                            if occupied_url != url || occupied_title != title =>
                        {
                            self.numbered_reference_labels
                                .entry((cursor_key.clone(), occupied_url, occupied_title))
                                .or_insert(candidate);
                            continue;
                        }
                        // Held by a definition a verbatim copy carries, whose
                        // target is unknown; nothing to remember it by, so the
                        // search simply moves on.
                        None if self.is_label_carried_verbatim(&candidate_key) => continue,
                        // Already holds this very target: share it.
                        Some(_) => {
                            self.satisfy_claimed_label(candidate_key, &candidate, url, title)
                        }
                        None => self.insert_reference(candidate_key, candidate.clone(), url, title),
                    }
                    self.reference_label_cursors.insert(cursor_key, n + 1);
                    self.numbered_reference_labels
                        .insert(target, candidate.clone());
                    return candidate;
                }
                unreachable!("the numbered label space is unbounded")
            }
        }
    }

    /// Register a definition for a label that so far is only *claimed*.
    ///
    /// A claim marks the label as spoken for by a link preserved verbatim, but
    /// emits no definition of its own, so a formatted link sharing that label
    /// still has to supply one.  The exception is a label whose definition is
    /// preserved verbatim too: that copy already defines it, and adding another
    /// would merely repeat it.
    fn satisfy_claimed_label(&mut self, key: String, label: &str, url: &str, title: &str) {
        if self.find_reference(&key).is_none() && !self.verbatim_reference_labels.contains(&key) {
            self.insert_reference(key, label.to_string(), url, title);
        }
    }

    /// Keep a reference definition alive for a link that is emitted verbatim.
    ///
    /// Such a link keeps whatever label the source gave it, so unlike
    /// [`Self::register_reference`] this never falls back to a derived label:
    /// a renamed definition would leave the verbatim link pointing at nothing.
    /// Returns `false` when the label is already taken by a different target,
    /// which the caller reports as a warning since the link cannot be saved.
    ///
    /// The definition is queued rather than registered at once, and falls due
    /// where the source put it, the way a footnote's references follow their
    /// footnote.  A copied link is not the reason its definition sits where it
    /// does, so letting it claim a place among the pending definitions would
    /// move that definition around — and where it would land depends on whether
    /// the parser could resolve the copied link, which is to say on whether an
    /// earlier run had already written the very definition being reserved.
    /// Following the source keeps every run agreeing on the placement.
    pub fn reserve_reference(&mut self, label: &str, url: &str, title: &str) -> bool {
        let key = normalize_reference_key(label);
        if let Some(existing) = self.find_reference(&key)
            && (existing.url != url || existing.title != title)
        {
            return false;
        }
        // A definition the scanner did not find falls due at the end, which is
        // the one placement that cannot come out ahead of something.
        let due = self
            .reference_definition_lines
            .get(&key)
            .copied()
            .unwrap_or(usize::MAX);
        self.deferred_references.push((
            ReferenceLink {
                label: label.to_string(),
                url: url.to_string(),
                title: title.to_string(),
            },
            due,
        ));
        true
    }

    /// Move the queued definitions that are due before `before_line` into the
    /// pending ones, behind everything the document's own links registered.
    /// `None` takes them all, for the flush that ends the document.
    ///
    /// A definition already emitted or already pending needs nothing: the first
    /// is written, and the second is about to be.  One held only by a footnote's
    /// collection does need this, though, since that collection is flushed on
    /// the footnote's schedule rather than here.
    pub fn take_deferred_references(&mut self, before_line: Option<usize>) {
        let mut deferred = std::mem::take(&mut self.deferred_references);
        deferred.retain(|(reference, due)| {
            if before_line.is_some_and(|line| *due >= line) {
                return true;
            }
            let key = normalize_reference_key(&reference.label);
            if !self.emitted_references.contains_key(&key)
                && !self.pending_references.contains_key(&key)
            {
                self.pending_references.insert(key, reference.clone());
            }
            false
        });
        self.deferred_references = deferred;
    }

    /// Store a new reference definition under an unused key.
    fn insert_reference(&mut self, key: String, label: String, url: &str, title: &str) {
        let reference = ReferenceLink {
            label,
            url: url.to_string(),
            title: title.to_string(),
        };
        if self.footnotes.collecting_content {
            self.footnotes.add_reference(key, reference);
        } else {
            self.pending_references.insert(key, reference);
        }
    }

    /// Check if a URL is external (starts with http:// or https://).
    pub fn is_external_url(url: &str) -> bool {
        url.starts_with("http://") || url.starts_with("https://")
    }

    /// Get the emphasis delimiter character.
    /// Uses '_' if the content contains '*' (to avoid escaping).
    /// Otherwise, preserves the original delimiter from source, defaulting to '*'.
    pub fn get_emphasis_delimiter<'b>(&self, node: &'b AstNode<'b>) -> char {
        // If content contains '*', use '_' to avoid escaping
        if self.node_text_contains_char(node, '*') {
            return '_';
        }
        // Otherwise, preserve original delimiter or default to '*'
        if let Some(source) = self.extract_source(node)
            && source.starts_with('_')
        {
            return '_';
        }
        '*'
    }

    /// Get the strong emphasis delimiter string.
    /// Uses "__" if the content contains '*' (to avoid escaping).
    /// Otherwise, preserves the original delimiter from source, defaulting to "**".
    pub fn get_strong_delimiter<'b>(&self, node: &'b AstNode<'b>) -> &'static str {
        // If content contains '*', use '__' to avoid escaping
        if self.node_text_contains_char(node, '*') {
            return "__";
        }
        // Otherwise, preserve original delimiter or default to '**'
        if let Some(source) = self.extract_source(node)
            && source.starts_with("__")
        {
            return "__";
        }
        "**"
    }

    /// Check if any text node within the given node contains the specified character.
    fn node_text_contains_char<'b>(&self, node: &'b AstNode<'b>, ch: char) -> bool {
        self.node_text_contains_char_recursive(node, ch)
    }

    fn node_text_contains_char_recursive<'b>(&self, node: &'b AstNode<'b>, ch: char) -> bool {
        match &node.data.borrow().value {
            NodeValue::Text(t) => t.contains(ch),
            _ => node
                .children()
                .any(|child| self.node_text_contains_char_recursive(child, ch)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::safe_str_slice;

    #[test]
    fn test_safe_str_slice_ascii() {
        let s = "hello world";
        assert_eq!(safe_str_slice(s, 0, 5), "hello");
        assert_eq!(safe_str_slice(s, 6, 11), "world");
        assert_eq!(safe_str_slice(s, 0, 11), "hello world");
    }

    #[test]
    fn test_safe_str_slice_valid_utf8_boundaries() {
        // ✅ is 3 bytes: [226, 156, 133]
        let s = "✅ test";
        assert_eq!(safe_str_slice(s, 0, 3), "✅");
        assert_eq!(safe_str_slice(s, 4, 8), "test");
        assert_eq!(safe_str_slice(s, 0, 8), "✅ test");
    }

    #[test]
    fn test_safe_str_slice_invalid_start_boundary() {
        // ✅ is 3 bytes: [226, 156, 133]
        let s = "✅ test";
        // Start at byte 1 (middle of ✅) should adjust to byte 0
        assert_eq!(safe_str_slice(s, 1, 8), "✅ test");
        // Start at byte 2 (middle of ✅) should adjust to byte 0
        assert_eq!(safe_str_slice(s, 2, 8), "✅ test");
    }

    #[test]
    fn test_safe_str_slice_invalid_end_boundary() {
        // ✅ is 3 bytes: [226, 156, 133]
        let s = "✅ test";
        // End at byte 1 (middle of ✅) should adjust to byte 3
        assert_eq!(safe_str_slice(s, 0, 1), "✅");
        // End at byte 2 (middle of ✅) should adjust to byte 3
        assert_eq!(safe_str_slice(s, 0, 2), "✅");
    }

    #[test]
    fn test_safe_str_slice_out_of_bounds() {
        let s = "hello";
        // End beyond string length
        assert_eq!(safe_str_slice(s, 0, 100), "hello");
        // Start beyond string length
        assert_eq!(safe_str_slice(s, 100, 200), "");
    }

    #[test]
    fn test_safe_str_slice_multiple_emoji() {
        // 🚨 is 4 bytes, ✅ is 3 bytes
        let s = "🚨 ✅";
        assert_eq!(safe_str_slice(s, 0, 4), "🚨");
        assert_eq!(safe_str_slice(s, 5, 8), "✅");
        // Invalid boundary in middle of 🚨
        assert_eq!(safe_str_slice(s, 1, 8), "🚨 ✅");
        assert_eq!(safe_str_slice(s, 2, 8), "🚨 ✅");
        assert_eq!(safe_str_slice(s, 3, 8), "🚨 ✅");
    }

    #[test]
    fn test_safe_str_slice_emoji_only() {
        // When the string is just an emoji and we try to slice at byte 1
        let s = "✅";
        // This was the exact case causing the panic: byte index 1 in a 3-byte emoji
        assert_eq!(safe_str_slice(s, 1, 3), "✅");
        assert_eq!(safe_str_slice(s, 0, 1), "✅");
    }
}
