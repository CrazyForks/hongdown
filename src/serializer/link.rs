//! Link and image serialization logic.

use comrak::nodes::{AstNode, NodeValue};

use super::Serializer;

impl<'a> Serializer<'a> {
    /// Format a reference-style link and write to output buffer.
    pub(super) fn format_reference_link(
        &mut self,
        output: &mut String,
        text: &str,
        label: &str,
        url: &str,
        title: &str,
    ) {
        let collapsed = label.starts_with('\x01');
        let label = label.strip_prefix('\x01').unwrap_or(label);
        // The label may already be taken by a different target, in which case a
        // distinct one is allocated and the full reference form is required.
        let label = self.register_reference(label, url, title);

        output.push('[');
        output.push_str(text);
        if label == text {
            if collapsed {
                // Collapsed reference: [text][]
                output.push_str("][]");
            } else {
                // Shortcut reference: [text]
                output.push(']');
            }
        } else {
            // Full reference: [text][label]
            output.push_str("][");
            output.push_str(&label);
            output.push(']');
        }
    }

    /// Format an inline-style link and write to output buffer.
    pub(super) fn format_inline_link(output: &mut String, text: &str, url: &str, title: &str) {
        output.push('[');
        output.push_str(text);
        output.push_str("](");
        output.push_str(url);
        if !title.is_empty() {
            output.push_str(" \"");
            output.push_str(title);
            output.push('"');
        }
        output.push(')');
    }

    /// Format an autolink and write to output buffer.
    pub(super) fn format_autolink(output: &mut String, url: &str) {
        output.push('<');
        output.push_str(url);
        output.push('>');
    }

    /// Format an external link as reference style and write to output buffer.
    ///
    /// If `use_collapsed` is true, outputs `[text][]` (collapsed reference) instead of
    /// `[text]` (shortcut reference). This is needed to disambiguate when the link is
    /// immediately followed by something that starts with `[`, like a footnote reference.
    pub(super) fn format_external_link_as_reference(
        &mut self,
        output: &mut String,
        text: &str,
        url: &str,
        title: &str,
        use_collapsed: bool,
    ) {
        // Normalize: replace SoftBreak markers with spaces for shortcut refs
        let normalized_text = text.replace('\x00', " ");
        // The link text is only usable as the label when it is not already
        // taken by a different target; otherwise fall back to a full reference.
        let label = self.register_reference(&normalized_text, url, title);

        output.push('[');
        output.push_str(&normalized_text);
        if label == normalized_text {
            output.push(']');
            if use_collapsed {
                output.push_str("[]");
            }
        } else {
            output.push_str("][");
            output.push_str(&label);
            output.push(']');
        }
    }

    /// Check if the next sibling of a node starts with `[`.
    /// This includes footnote references, link references, and images.
    pub(super) fn next_sibling_starts_with_bracket<'b>(node: &'b AstNode<'b>) -> bool {
        if let Some(next) = node.next_sibling() {
            let value = &next.data.borrow().value;
            matches!(
                value,
                NodeValue::FootnoteReference(_) | NodeValue::Link(_) | NodeValue::Image(_)
            )
        } else {
            false
        }
    }

    /// Format a reference-style image and write to output buffer.
    pub(super) fn format_reference_image(
        &mut self,
        output: &mut String,
        text: &str,
        label: &str,
        url: &str,
        title: &str,
    ) {
        let collapsed = label.starts_with('\x01');
        let label = label.strip_prefix('\x01').unwrap_or(label);
        let label = self.register_reference(label, url, title);

        output.push_str("![");
        output.push_str(text);
        if label == text {
            if collapsed {
                // Collapsed reference: ![alt][]
                output.push_str("][]");
            } else {
                // Shortcut reference: ![alt]
                output.push(']');
            }
        } else {
            // Full reference: ![alt][label]
            output.push_str("][");
            output.push_str(&label);
            output.push(']');
        }
    }

    /// Format an inline-style image and write to output buffer.
    pub(super) fn format_inline_image(output: &mut String, alt_text: &str, url: &str, title: &str) {
        output.push_str("![");
        output.push_str(alt_text);
        output.push_str("](");
        output.push_str(url);
        if !title.is_empty() {
            output.push_str(" \"");
            output.push_str(title);
            output.push('"');
        }
        output.push(')');
    }

    pub(super) fn serialize_link<'b>(&mut self, node: &'b AstNode<'b>, url: &str, title: &str) {
        // Check if link contains an image (badge-style link)
        let contains_image = node
            .children()
            .any(|child| matches!(&child.data.borrow().value, NodeValue::Image(_)));

        // Check if this is an autolink (link text equals URL)
        let raw_text = self.collect_raw_text(node);
        let is_autolink = title.is_empty() && raw_text == url;

        // Check if original was reference style
        if let Some((text, label)) = self.get_reference_style_info(node) {
            // For badge-style, serialize children first to get image content
            if contains_image {
                // Badge-style with reference: [![alt][img-ref]][link-ref]
                self.output.push('[');
                for child in node.children() {
                    self.serialize_node(child);
                }
                self.output.push_str("][");
                let actual_label = label.strip_prefix('\x01').unwrap_or(&label);
                let actual_label = self.register_reference(actual_label, url, title);
                self.output.push_str(&actual_label);
                self.output.push(']');
            } else {
                // Use helper for non-badge reference links
                let mut output = String::new();
                self.format_reference_link(&mut output, &text, &label, url, title);
                self.output.push_str(&output);
            }
        } else if contains_image {
            // Badge-style inline: [![alt](img-url)](link-url)
            self.output.push('[');
            for child in node.children() {
                self.serialize_node(child);
            }
            self.output.push_str("](");
            self.output.push_str(url);
            if !title.is_empty() {
                self.output.push_str(" \"");
                self.output.push_str(title);
                self.output.push('"');
            }
            self.output.push(')');
        } else if is_autolink {
            Self::format_autolink(&mut self.output, url);
        } else if Self::is_external_url(url) {
            let link_text = self.collect_text(node);
            let mut output = String::new();
            let use_collapsed = Self::next_sibling_starts_with_bracket(node);
            self.format_external_link_as_reference(
                &mut output,
                &link_text,
                url,
                title,
                use_collapsed,
            );
            self.output.push_str(&output);
        } else {
            // Relative/local URL: keep as inline link
            let link_text = self.collect_text(node);
            Self::format_inline_link(&mut self.output, &link_text, url, title);
        }
    }

    pub(super) fn serialize_image<'b>(&mut self, node: &'b AstNode<'b>, url: &str, title: &str) {
        // Check if original was reference style
        if let Some((text, label)) = self.get_reference_style_info(node) {
            // Use a temporary buffer to avoid double borrow
            let mut output = String::new();
            self.format_reference_image(&mut output, &text, &label, url, title);
            self.output.push_str(&output);
            return;
        }

        // Inline style: ![alt](url).  The alt text is collected only here:
        // collecting it up front would register reference definitions for any
        // links inside it even when the output is discarded, reserving labels
        // that never appear in the document.
        let alt_text = self.collect_text(node);
        Self::format_inline_image(&mut self.output, &alt_text, url, title);
    }
}
