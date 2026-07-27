//! Inline node collection and text extraction logic.

use comrak::nodes::{AstNode, NodeMath, NodeValue};

use super::Serializer;
use super::escape;
use super::punctuation;
use super::{MATH_TOKEN_CLOSE, MATH_TOKEN_OPEN};

enum LinkTextSegment {
    Ordinary(String),
    Verbatim(String),
}

impl<'a> Serializer<'a> {
    pub(super) fn collect_text<'b>(&mut self, node: &'b AstNode<'b>) -> String {
        let mut text = String::new();
        self.collect_text_recursive(node, &mut text);
        text
    }

    /// Serialize the inline children of `node` without adding an outer link or
    /// image wrapper.
    pub(super) fn collect_inline_children<'b>(&mut self, node: &'b AstNode<'b>) -> String {
        let mut text = String::new();
        for child in node.children() {
            self.collect_inline_node(child, &mut text);
        }
        text
    }

    pub(super) fn contains_image<'b>(node: &'b AstNode<'b>) -> bool {
        node.children().any(|child| {
            matches!(&child.data.borrow().value, NodeValue::Image(_)) || Self::contains_image(child)
        })
    }

    /// Serialize inline children while collapsing whitespace runs in ordinary
    /// text nodes.  Whitespace-sensitive inline constructs remain untouched.
    pub(super) fn collect_normalized_link_text<'b>(&mut self, node: &'b AstNode<'b>) -> String {
        let mut segments = Vec::new();
        let mut html_depth = 0;
        for child in node.children() {
            self.collect_link_text_segments(child, &mut segments, &mut html_depth);
        }
        Self::normalize_link_text_segments(segments)
    }

    fn collect_link_text_segments<'b>(
        &mut self,
        node: &'b AstNode<'b>,
        segments: &mut Vec<LinkTextSegment>,
        html_depth: &mut usize,
    ) {
        match &node.data.borrow().value {
            NodeValue::Text(_) | NodeValue::SoftBreak => {
                let mut text = String::new();
                self.collect_inline_node(node, &mut text);
                let text = text.replace('\x00', " ");
                if *html_depth == 0 {
                    segments.push(LinkTextSegment::Ordinary(text));
                } else {
                    segments.push(LinkTextSegment::Verbatim(text));
                }
            }
            NodeValue::Emph => {
                let delimiter = self.get_emphasis_delimiter(node).to_string();
                segments.push(LinkTextSegment::Verbatim(delimiter.clone()));
                for child in node.children() {
                    self.collect_link_text_segments(child, segments, html_depth);
                }
                segments.push(LinkTextSegment::Verbatim(delimiter));
            }
            NodeValue::Strong => {
                let delimiter = self.get_strong_delimiter(node).to_string();
                segments.push(LinkTextSegment::Verbatim(delimiter.clone()));
                for child in node.children() {
                    self.collect_link_text_segments(child, segments, html_depth);
                }
                segments.push(LinkTextSegment::Verbatim(delimiter));
            }
            NodeValue::HtmlInline(html) => {
                segments.push(LinkTextSegment::Verbatim(html.clone()));
                Self::update_inline_html_depth(html, html_depth);
            }
            NodeValue::Code(_) | NodeValue::Math(_) | NodeValue::LineBreak => {
                let mut text = String::new();
                self.collect_inline_node(node, &mut text);
                segments.push(LinkTextSegment::Verbatim(text));
            }
            NodeValue::Image(_) => {
                let mut image = String::new();
                self.collect_inline_node(node, &mut image);
                segments.push(LinkTextSegment::Verbatim(image));
            }
            _ if *html_depth == 0 && node.children().next().is_some() => {
                for child in node.children() {
                    self.collect_link_text_segments(child, segments, html_depth);
                }
            }
            _ => {
                let mut text = String::new();
                self.collect_inline_node(node, &mut text);
                segments.push(LinkTextSegment::Verbatim(text));
            }
        }
    }

    fn normalize_link_text_segments(segments: Vec<LinkTextSegment>) -> String {
        let mut output = String::new();
        let mut pending_whitespace = String::new();

        for segment in segments {
            match segment {
                LinkTextSegment::Ordinary(text) => {
                    for ch in text.chars() {
                        if escape::is_commonmark_whitespace(ch) {
                            pending_whitespace.push(ch);
                        } else {
                            Self::flush_link_text_whitespace(&mut output, &mut pending_whitespace);
                            output.push(ch);
                        }
                    }
                }
                LinkTextSegment::Verbatim(text) => {
                    if !text.is_empty() {
                        Self::flush_link_text_whitespace(&mut output, &mut pending_whitespace);
                        output.push_str(&text);
                    }
                }
            }
        }

        Self::flush_link_text_whitespace(&mut output, &mut pending_whitespace);
        output
    }

    fn flush_link_text_whitespace(output: &mut String, pending_whitespace: &mut String) {
        if pending_whitespace.is_empty() {
            return;
        }
        output.push(' ');
        pending_whitespace.clear();
    }

    fn update_inline_html_depth(html: &str, depth: &mut usize) {
        let html = html.trim();
        if html.starts_with("</") {
            *depth = depth.saturating_sub(1);
            return;
        }
        if !html.starts_with('<')
            || html.starts_with("<!")
            || html.starts_with("<?")
            || html.ends_with("/>")
        {
            return;
        }

        let tag = html[1..]
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
            .collect::<String>()
            .to_ascii_lowercase();
        if !tag.is_empty()
            && !matches!(
                tag.as_str(),
                "area"
                    | "base"
                    | "br"
                    | "col"
                    | "embed"
                    | "hr"
                    | "img"
                    | "input"
                    | "link"
                    | "meta"
                    | "param"
                    | "source"
                    | "track"
                    | "wbr"
            )
        {
            *depth += 1;
        }
    }

    /// Collect raw text without escaping (for comparison purposes)
    pub(super) fn collect_raw_text<'b>(&self, node: &'b AstNode<'b>) -> String {
        let mut text = String::new();
        self.collect_raw_text_recursive(node, &mut text);
        text
    }

    fn collect_raw_text_recursive<'b>(&self, node: &'b AstNode<'b>, text: &mut String) {
        match &node.data.borrow().value {
            NodeValue::Text(t) => {
                text.push_str(t);
            }
            NodeValue::SoftBreak => {
                text.push(' ');
            }
            _ => {
                for child in node.children() {
                    self.collect_raw_text_recursive(child, text);
                }
            }
        }
    }

    /// Render a TeX/LaTeX math node to its Markdown form, preferring the exact
    /// original source over reconstruction.
    ///
    /// Inline math must stay on a single line for the wrapper; if the original
    /// source span happens to span multiple lines, fall back to the normalized
    /// literal so the formula remains a single token.
    pub(super) fn render_math<'b>(&self, node: &'b AstNode<'b>, math: &NodeMath) -> String {
        if let Some(source) = self.extract_source(node)
            && (math.display_math || !source.contains('\n'))
        {
            return source;
        }
        escape::format_math(&math.literal, math.display_math)
    }

    fn collect_text_recursive<'b>(&mut self, node: &'b AstNode<'b>, text: &mut String) {
        match &node.data.borrow().value {
            NodeValue::Text(t) => {
                // Apply punctuation transformation first
                let transformed = punctuation::transform_punctuation(t, self.options);

                // Try to preserve escapes from the original source
                if let Some(source) = self.extract_source(node) {
                    text.push_str(&Self::escape_text_preserving_source(&transformed, &source));
                } else {
                    text.push_str(&escape::escape_text(&transformed));
                }
            }
            NodeValue::Code(code) => {
                // Try to use original source to preserve spacing, but validate it first.
                // comrak may provide incorrect sourcepos for code spans in table cells
                // containing escaped pipe characters (e.g., `string \| number`).
                // Also, multiline code spans in source need to be normalized (CommonMark
                // converts newlines in code spans to spaces).
                if let Some(source) = self.extract_source(node) {
                    if escape::is_valid_code_span(&source) && !source.contains('\n') {
                        text.push_str(&source);
                    } else {
                        text.push_str(&escape::format_code_span(&code.literal));
                    }
                } else {
                    text.push_str(&escape::format_code_span(&code.literal));
                }
            }
            NodeValue::Math(math) => {
                // Heading text is not wrapped, so no atomic-token placeholder is
                // needed here; emit the math verbatim.
                text.push_str(&self.render_math(node, math));
            }
            NodeValue::Emph => {
                let delim = self.get_emphasis_delimiter(node);
                text.push(delim);
                for child in node.children() {
                    self.collect_text_recursive(child, text);
                }
                text.push(delim);
            }
            NodeValue::Strong => {
                let delim = self.get_strong_delimiter(node);
                text.push_str(delim);
                for child in node.children() {
                    self.collect_text_recursive(child, text);
                }
                text.push_str(delim);
            }
            NodeValue::SoftBreak => {
                text.push(' ');
            }
            NodeValue::Link(link) => {
                // Handle reference-style links in headings
                if Self::contains_image(node) {
                    let link_text = self.collect_inline_children(node).replace('\x00', " ");
                    if let Some((_, label)) = self.get_reference_style_info(node) {
                        self.format_reference_link(
                            text,
                            &link_text,
                            &label,
                            &link.url,
                            &link.title,
                        );
                    } else {
                        Self::format_inline_link(text, &link_text, &link.url, &link.title);
                    }
                } else if let Some((_, label)) = self.get_reference_style_info(node) {
                    let link_text = self.collect_inline_children(node);
                    self.format_reference_link(text, &link_text, &label, &link.url, &link.title);
                } else {
                    // For inline links, just output plain text (or format as inline?)
                    // In headings, we typically want reference style for external links
                    if Self::is_external_url(&link.url) {
                        let link_text = self.collect_normalized_link_text(node);
                        // Headings don't have footnote references as siblings, so no need for collapsed style
                        self.format_external_link_as_reference(
                            text,
                            &link_text,
                            &link.url,
                            &link.title,
                            false,
                        );
                    } else {
                        let link_text = self.collect_raw_text(node);
                        Self::format_inline_link(text, &link_text, &link.url, &link.title);
                    }
                }
            }
            NodeValue::Image(image) => {
                // Preserve images in headings using inline syntax
                let alt_text = self.collect_raw_text(node);
                Self::format_inline_image(text, &alt_text, &image.url, &image.title);
            }
            _ => {
                for child in node.children() {
                    self.collect_text_recursive(child, text);
                }
            }
        }
    }

    pub(super) fn collect_inline_content<'b>(
        &mut self,
        node: &'b AstNode<'b>,
        content: &mut String,
    ) {
        for child in node.children() {
            self.collect_inline_node(child, content);
        }
    }

    pub(super) fn collect_inline_node<'b>(&mut self, node: &'b AstNode<'b>, content: &mut String) {
        match &node.data.borrow().value {
            NodeValue::Text(text) => {
                // Apply punctuation transformation first
                let transformed = punctuation::transform_punctuation(text, self.options);

                // Try to preserve escapes from the original source
                if let Some(source) = self.extract_source(node) {
                    content.push_str(&Self::escape_text_preserving_source(&transformed, &source));
                } else {
                    content.push_str(&escape::escape_text(&transformed));
                }
            }
            NodeValue::SoftBreak => {
                // Use a special marker to preserve original line breaks
                // This will be processed by wrap_text to decide whether to keep them
                content.push('\x00');
            }
            NodeValue::LineBreak => {
                content.push('\n');
            }
            NodeValue::Emph => {
                let delim = self.get_emphasis_delimiter(node);
                content.push(delim);
                for child in node.children() {
                    self.collect_inline_node(child, content);
                }
                content.push(delim);
            }
            NodeValue::Strong => {
                let delim = self.get_strong_delimiter(node);
                content.push_str(delim);
                for child in node.children() {
                    self.collect_inline_node(child, content);
                }
                content.push_str(delim);
            }
            NodeValue::Code(code) => {
                // Try to use original source to preserve spacing, but validate it first.
                // comrak may provide incorrect sourcepos for code spans in table cells
                // containing escaped pipe characters (e.g., `string \| number`).
                // Also, multiline code spans in source need to be normalized (CommonMark
                // converts newlines in code spans to spaces).
                if let Some(source) = self.extract_source(node) {
                    if escape::is_valid_code_span(&source) && !source.contains('\n') {
                        content.push_str(&source);
                    } else {
                        content.push_str(&escape::format_code_span(&code.literal));
                    }
                } else {
                    content.push_str(&escape::format_code_span(&code.literal));
                }
            }
            NodeValue::Math(math) => {
                let rendered = self.render_math(node, math);
                if math.display_math {
                    content.push_str(&rendered);
                } else {
                    // Bracket inline math with sentinels so the wrapper keeps the
                    // whole formula on one line (its real spaces are preserved);
                    // the sentinels are stripped from the final output.
                    content.push(MATH_TOKEN_OPEN);
                    content.push_str(&rendered);
                    content.push(MATH_TOKEN_CLOSE);
                }
            }
            NodeValue::Link(link) => {
                // Check if link contains an image (badge-style link)
                let contains_image = Self::contains_image(node);

                // Check if this is an autolink (link text equals URL)
                let raw_text = self.collect_raw_text(node);
                let is_autolink = link.title.is_empty() && raw_text == link.url;

                // Check if original was reference style
                if let Some((_, label)) = self.get_reference_style_info(node) {
                    // Preserve reference style
                    if contains_image {
                        // Badge-style with reference: [![alt][img-ref]][link-ref]
                        let actual_label = label.strip_prefix('\x01').unwrap_or(&label);
                        content.push('[');
                        for child in node.children() {
                            self.collect_inline_node(child, content);
                        }
                        let actual_label =
                            self.register_reference(actual_label, &link.url, &link.title);
                        content.push_str("][");
                        content.push_str(&actual_label);
                        content.push(']');
                    } else {
                        // Non-badge reference links: use helper
                        let text = self.collect_inline_children(node);
                        self.format_reference_link(content, &text, &label, &link.url, &link.title);
                    }
                } else if contains_image {
                    // Badge-style inline: [![alt](img-url)](link-url)
                    // Need to iterate children, so can't use helper directly
                    content.push('[');
                    for child in node.children() {
                        self.collect_inline_node(child, content);
                    }
                    content.push_str("](");
                    content.push_str(&link.url);
                    if !link.title.is_empty() {
                        content.push_str(" \"");
                        content.push_str(&link.title);
                        content.push('"');
                    }
                    content.push(')');
                } else if is_autolink {
                    Self::format_autolink(content, &link.url);
                } else if Self::is_external_url(&link.url) {
                    // External URL: collect link text first
                    let link_text = self.collect_normalized_link_text(node);
                    // Check if next sibling starts with '[' to decide if we need collapsed style
                    let use_collapsed = Self::next_sibling_starts_with_bracket(node);
                    self.format_external_link_as_reference(
                        content,
                        &link_text,
                        &link.url,
                        &link.title,
                        use_collapsed,
                    );
                } else {
                    // Relative/local URL: keep as inline link
                    let link_text = self.collect_inline_children(node);
                    Self::format_inline_link(content, &link_text, &link.url, &link.title);
                }
            }
            NodeValue::Image(image) => {
                // Check if original was reference style
                if let Some((text, label)) = self.get_reference_style_info(node) {
                    self.format_reference_image(content, &text, &label, &image.url, &image.title);
                } else {
                    // Inline style: collect alt text and use inline syntax
                    let mut alt_text = String::new();
                    for child in node.children() {
                        self.collect_inline_node(child, &mut alt_text);
                    }
                    Self::format_inline_image(content, &alt_text, &image.url, &image.title);
                }
            }
            NodeValue::HtmlInline(html) => {
                // Preserve inline HTML as-is
                content.push_str(html);
            }
            NodeValue::FootnoteReference(footnote_ref) => {
                content.push_str("[^");
                content.push_str(&footnote_ref.name);
                content.push(']');
            }
            _ => {
                for child in node.children() {
                    self.collect_inline_node(child, content);
                }
            }
        }
    }

    /// Escape text while preserving escapes from the original source.
    ///
    /// When comrak parses text like `node\_modules`, it stores `node_modules` in the AST.
    /// This function compares the parsed text with the original source to detect which
    /// characters were escaped, and preserves those escapes in the output.
    ///
    /// Also preserves HTML entities (e.g., `&lt;`, `&amp;`, `&#60;`) from the source.
    fn escape_text_preserving_source(text: &str, source: &str) -> String {
        let mut result = String::with_capacity(source.len());
        let text_chars: Vec<char> = text.chars().collect();
        let source_chars: Vec<char> = source.chars().collect();

        let mut text_idx = 0;
        let mut source_idx = 0;

        while text_idx < text_chars.len() && source_idx < source_chars.len() {
            let text_char = text_chars[text_idx];
            let source_char = source_chars[source_idx];

            if source_char == '\\' && source_idx + 1 < source_chars.len() {
                if text_char == '\\' {
                    // Both source and text have a backslash
                    // Check if source has an escaped backslash (\\)
                    if source_chars[source_idx + 1] == '\\' {
                        // Source has \\ and text has \ - preserve the escaped backslash
                        result.push_str("\\\\");
                        text_idx += 1;
                        source_idx += 2;
                    } else {
                        // Source has \ followed by non-backslash, text has \
                        // It's a literal backslash that needs escaping
                        Self::push_escaped_text_char(&mut result, &text_chars, text_idx);
                        text_idx += 1;
                        source_idx += 1;
                    }
                } else {
                    // Source has an escape sequence, text doesn't have a backslash
                    let escaped_char = source_chars[source_idx + 1];
                    if escaped_char == text_char {
                        // The escape in source corresponds to this character in text
                        // Preserve explicit escapes for ASCII punctuation. CommonMark
                        // allows escaping any ASCII punctuation with backslash.
                        if escaped_char.is_ascii_punctuation() {
                            // Preserve the escape from source (e.g., \_ → \_)
                            result.push('\\');
                            result.push(escaped_char);
                        } else {
                            // Character doesn't need escaping, so backslash should be literal
                            // Output escaped backslash + character (e.g., \U → \\U)
                            result.push_str("\\\\");
                            result.push(text_char);
                        }
                        text_idx += 1;
                        source_idx += 2;
                    } else {
                        // Escape doesn't match - use normal escaping
                        Self::push_escaped_text_char(&mut result, &text_chars, text_idx);
                        text_idx += 1;
                        // Don't advance source_idx - the escape might be for something else
                    }
                }
            } else if source_char == '&' {
                // Check for HTML entity
                if let Some((entity, decoded_char)) =
                    Self::try_parse_html_entity(&source_chars, source_idx)
                {
                    if decoded_char == text_char {
                        // The entity decodes to this character - preserve the entity
                        result.push_str(&entity);
                        text_idx += 1;
                        source_idx += entity.len();
                    } else {
                        // Entity doesn't match the text character - use normal escaping
                        Self::push_escaped_text_char(&mut result, &text_chars, text_idx);
                        text_idx += 1;
                    }
                } else if source_char == text_char {
                    // Not an entity, just a regular '&'
                    Self::push_escaped_text_char(&mut result, &text_chars, text_idx);
                    text_idx += 1;
                    source_idx += 1;
                } else {
                    // Characters don't match - skip source character
                    source_idx += 1;
                }
            } else if source_char == text_char {
                // Characters match - apply normal escaping rules
                if matches!(text_char, '[' | ']') && text_idx > 0 {
                    // Preserve literal square brackets from source as-is.
                    // Escaping these can break valid reference-style links when
                    // comrak leaves them as text (e.g., after abbreviation defs).
                    result.push(text_char);
                } else {
                    Self::push_escaped_text_char(&mut result, &text_chars, text_idx);
                }
                text_idx += 1;
                source_idx += 1;
            } else {
                // Characters don't match - source might have extra content
                // Skip the source character and try again
                source_idx += 1;
            }
        }

        // Handle any remaining text characters that weren't matched
        for idx in text_idx..text_chars.len() {
            Self::push_escaped_text_char(&mut result, &text_chars, idx);
        }

        result
    }

    fn push_escaped_text_char(result: &mut String, text_chars: &[char], idx: usize) {
        let prev = if idx > 0 {
            Some(text_chars[idx - 1])
        } else {
            None
        };
        let next = text_chars.get(idx + 1).copied();
        escape::push_escaped_char(result, text_chars[idx], prev, next);
    }

    /// Try to parse an HTML entity starting at the given position.
    /// Returns the entity string and the decoded character if successful.
    fn try_parse_html_entity(chars: &[char], start: usize) -> Option<(String, char)> {
        if start >= chars.len() || chars[start] != '&' {
            return None;
        }

        // Find the end of the entity (semicolon)
        let mut end = start + 1;
        while end < chars.len() && end - start < 12 {
            // Max entity length ~10 chars
            if chars[end] == ';' {
                let entity: String = chars[start..=end].iter().collect();
                if let Some(decoded) = Self::decode_html_entity(&entity) {
                    return Some((entity, decoded));
                }
                return None;
            }
            if !chars[end].is_ascii_alphanumeric() && chars[end] != '#' {
                return None;
            }
            end += 1;
        }

        None
    }

    /// Decode a single HTML entity to its character.
    fn decode_html_entity(entity: &str) -> Option<char> {
        // Handle numeric entities
        if entity.starts_with("&#") {
            let inner = entity.trim_start_matches("&#").trim_end_matches(';');
            if let Some(hex) = inner.strip_prefix('x').or_else(|| inner.strip_prefix('X')) {
                // Hexadecimal: &#xNN;
                u32::from_str_radix(hex, 16).ok().and_then(char::from_u32)
            } else {
                // Decimal: &#NN;
                inner.parse::<u32>().ok().and_then(char::from_u32)
            }
        } else {
            // Named entities - use html_escape's complete table
            let name = entity
                .trim_start_matches('&')
                .trim_end_matches(';')
                .as_bytes();
            html_escape::NAMED_ENTITIES
                .binary_search_by_key(&name, |(n, _)| n)
                .ok()
                .and_then(|idx| {
                    let (_, value) = &html_escape::NAMED_ENTITIES[idx];
                    // Most entities decode to a single character
                    let mut chars = value.chars();
                    let first = chars.next()?;
                    // TODO: Some entities decode to multiple characters (e.g., &fj; -> "fj").
                    // Currently we only handle single-character entities. To support
                    // multi-character entities, the return type would need to change from
                    // Option<char> to Option<&str> or similar, and callers would need updating.
                    if chars.next().is_none() {
                        Some(first)
                    } else {
                        None
                    }
                })
        }
    }
}
