//! Serializer for converting comrak AST to formatted Markdown.

use comrak::nodes::{AstNode, ListType, NodeValue};

use crate::Options;

/// Serializes a comrak AST node to a formatted Markdown string.
pub fn serialize<'a>(node: &'a AstNode<'a>, options: &Options) -> String {
    let mut serializer = Serializer::new(options);
    serializer.serialize_node(node);
    serializer.output
}

struct Serializer<'a> {
    output: String,
    options: &'a Options,
    /// Current list item index (1-based) for ordered lists
    list_item_index: usize,
    /// Current list type
    list_type: Option<ListType>,
    /// Whether we're inside a block quote
    in_block_quote: bool,
}

impl<'a> Serializer<'a> {
    fn new(options: &'a Options) -> Self {
        Self {
            output: String::new(),
            options,
            list_item_index: 0,
            list_type: None,
            in_block_quote: false,
        }
    }

    fn serialize_node<'b>(&mut self, node: &'b AstNode<'b>) {
        match &node.data.borrow().value {
            NodeValue::Document => {
                self.serialize_children(node);
            }
            NodeValue::Heading(heading) => {
                self.serialize_heading(node, heading.level);
            }
            NodeValue::List(list) => {
                self.serialize_list(node, list.list_type);
            }
            NodeValue::CodeBlock(code_block) => {
                self.serialize_code_block(&code_block.info, &code_block.literal);
            }
            NodeValue::BlockQuote => {
                self.serialize_block_quote(node);
            }
            NodeValue::FrontMatter(content) => {
                self.serialize_front_matter(content);
            }
            NodeValue::Item(_) => {
                self.serialize_list_item(node);
            }
            NodeValue::Paragraph => {
                self.serialize_paragraph(node);
            }
            NodeValue::Text(text) => {
                self.output.push_str(text);
            }
            NodeValue::SoftBreak => {
                self.output.push(' ');
            }
            NodeValue::LineBreak => {
                self.output.push('\n');
            }
            NodeValue::Emph => {
                self.output.push('*');
                self.serialize_children(node);
                self.output.push('*');
            }
            NodeValue::Strong => {
                self.output.push_str("**");
                self.serialize_children(node);
                self.output.push_str("**");
            }
            NodeValue::Code(code) => {
                self.output.push('`');
                self.output.push_str(&code.literal);
                self.output.push('`');
            }
            NodeValue::Link(link) => {
                self.output.push('[');
                self.serialize_children(node);
                self.output.push_str("](");
                self.output.push_str(&link.url);
                if !link.title.is_empty() {
                    self.output.push_str(" \"");
                    self.output.push_str(&link.title);
                    self.output.push('"');
                }
                self.output.push(')');
            }
            _ => {
                // For now, just recurse into children for unhandled nodes
                self.serialize_children(node);
            }
        }
    }

    fn serialize_heading<'b>(&mut self, node: &'b AstNode<'b>, level: u8) {
        // Collect heading text first
        let heading_text = self.collect_text(node);

        if level == 1 {
            // Setext-style with '='
            self.output.push_str(&heading_text);
            self.output.push('\n');
            self.output
                .push_str(&"=".repeat(heading_text.chars().count()));
            self.output.push('\n');
        } else if level == 2 {
            // Setext-style with '-'
            self.output.push_str(&heading_text);
            self.output.push('\n');
            self.output
                .push_str(&"-".repeat(heading_text.chars().count()));
            self.output.push('\n');
        } else {
            // ATX-style for level 3+
            self.output.push_str(&"#".repeat(level as usize));
            self.output.push(' ');
            self.output.push_str(&heading_text);
            self.output.push('\n');
        }
    }

    fn collect_text<'b>(&self, node: &'b AstNode<'b>) -> String {
        let mut text = String::new();
        self.collect_text_recursive(node, &mut text);
        text
    }

    fn collect_text_recursive<'b>(&self, node: &'b AstNode<'b>, text: &mut String) {
        match &node.data.borrow().value {
            NodeValue::Text(t) => {
                text.push_str(t);
            }
            NodeValue::SoftBreak => {
                text.push(' ');
            }
            _ => {
                for child in node.children() {
                    self.collect_text_recursive(child, text);
                }
            }
        }
    }

    fn serialize_paragraph<'b>(&mut self, node: &'b AstNode<'b>) {
        // Collect all inline content first
        let mut inline_content = String::new();
        self.collect_inline_content(node, &mut inline_content);

        let prefix = if self.in_block_quote { "> " } else { "" };

        if self.list_type.is_some() {
            // Inside a list item, don't wrap or add trailing newline
            self.output.push_str(&inline_content);
        } else {
            // Wrap the paragraph at line_width
            let wrapped = self.wrap_text(&inline_content, prefix);
            self.output.push_str(&wrapped);
            self.output.push('\n');
        }
    }

    fn collect_inline_content<'b>(&self, node: &'b AstNode<'b>, content: &mut String) {
        for child in node.children() {
            self.collect_inline_node(child, content);
        }
    }

    fn collect_inline_node<'b>(&self, node: &'b AstNode<'b>, content: &mut String) {
        match &node.data.borrow().value {
            NodeValue::Text(text) => {
                content.push_str(text);
            }
            NodeValue::SoftBreak => {
                content.push(' ');
            }
            NodeValue::LineBreak => {
                content.push('\n');
            }
            NodeValue::Emph => {
                content.push('*');
                for child in node.children() {
                    self.collect_inline_node(child, content);
                }
                content.push('*');
            }
            NodeValue::Strong => {
                content.push_str("**");
                for child in node.children() {
                    self.collect_inline_node(child, content);
                }
                content.push_str("**");
            }
            NodeValue::Code(code) => {
                content.push('`');
                content.push_str(&code.literal);
                content.push('`');
            }
            NodeValue::Link(link) => {
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
            }
            _ => {
                for child in node.children() {
                    self.collect_inline_node(child, content);
                }
            }
        }
    }

    fn wrap_text(&self, text: &str, prefix: &str) -> String {
        let line_width = self.options.line_width;
        let mut result = String::new();
        let mut current_line = String::new();
        let prefix_len = prefix.len();

        // Add prefix to first line
        current_line.push_str(prefix);

        for word in text.split_whitespace() {
            let word_len = word.len();

            if current_line.len() == prefix_len {
                // First word on this line
                current_line.push_str(word);
            } else if current_line.len() + 1 + word_len <= line_width {
                // Word fits on current line
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                // Start a new line
                result.push_str(&current_line);
                result.push('\n');
                current_line = String::from(prefix);
                current_line.push_str(word);
            }
        }

        // Add the last line
        if !current_line.is_empty() && current_line != prefix {
            result.push_str(&current_line);
        }

        result
    }

    fn serialize_front_matter(&mut self, content: &str) {
        // Front matter content from comrak includes the delimiters,
        // so we preserve it verbatim and add a trailing blank line
        self.output.push_str(content.trim());
        self.output.push_str("\n\n");
    }

    fn serialize_block_quote<'b>(&mut self, node: &'b AstNode<'b>) {
        let was_in_block_quote = self.in_block_quote;
        self.in_block_quote = true;

        let children: Vec<_> = node.children().collect();
        for (i, child) in children.iter().enumerate() {
            // Add blank quote line between paragraphs
            if i > 0 {
                self.output.push_str(">\n");
            }
            self.serialize_node(child);
        }

        self.in_block_quote = was_in_block_quote;
    }

    fn serialize_code_block(&mut self, info: &str, literal: &str) {
        // Determine the minimum fence length (at least 4)
        let min_fence_length = 4;

        // Find the longest sequence of tildes in the content
        let max_tildes_in_content = literal
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim_start();
                if trimmed.starts_with('~') {
                    Some(trimmed.chars().take_while(|&c| c == '~').count())
                } else {
                    None
                }
            })
            .max()
            .unwrap_or(0);

        // Fence length must be greater than any tilde sequence in content
        let fence_length = std::cmp::max(min_fence_length, max_tildes_in_content + 1);
        let fence = "~".repeat(fence_length);

        // Use "text" as default if no language specified
        let language = if info.is_empty() { "text" } else { info };

        self.output.push_str(&fence);
        self.output.push(' ');
        self.output.push_str(language);
        self.output.push('\n');
        self.output.push_str(literal);
        self.output.push_str(&fence);
        self.output.push('\n');
    }

    fn serialize_list<'b>(&mut self, node: &'b AstNode<'b>, list_type: ListType) {
        let old_list_type = self.list_type;
        let old_index = self.list_item_index;

        self.list_type = Some(list_type);
        self.list_item_index = 0;

        self.serialize_children(node);

        self.list_type = old_list_type;
        self.list_item_index = old_index;
    }

    fn serialize_list_item<'b>(&mut self, node: &'b AstNode<'b>) {
        self.list_item_index += 1;

        match self.list_type {
            Some(ListType::Bullet) => {
                // " -  " format: one leading space, hyphen, two trailing spaces
                self.output.push_str(" -  ");
            }
            Some(ListType::Ordered) => {
                // " N. " format for ordered lists
                self.output.push(' ');
                self.output.push_str(&self.list_item_index.to_string());
                self.output.push_str(". ");
            }
            None => {}
        }

        self.serialize_children(node);
        self.output.push('\n');
    }

    fn serialize_children<'b>(&mut self, node: &'b AstNode<'b>) {
        for child in node.children() {
            self.serialize_node(child);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use comrak::{Arena, Options as ComrakOptions, parse_document};

    fn parse_and_serialize(input: &str) -> String {
        let arena = Arena::new();
        let options = ComrakOptions::default();
        let root = parse_document(&arena, input, &options);
        let format_options = Options::default();
        serialize(root, &format_options)
    }

    #[test]
    fn test_serialize_plain_text() {
        let result = parse_and_serialize("Hello, world!");
        assert_eq!(result, "Hello, world!\n");
    }

    #[test]
    fn test_serialize_multiline_paragraph() {
        let result = parse_and_serialize("Hello\nworld!");
        assert_eq!(result, "Hello world!\n");
    }

    #[test]
    fn test_serialize_h1_setext() {
        let result = parse_and_serialize("# Document Title");
        assert_eq!(result, "Document Title\n==============\n");
    }

    #[test]
    fn test_serialize_h2_setext() {
        let result = parse_and_serialize("## Section Name");
        assert_eq!(result, "Section Name\n------------\n");
    }

    #[test]
    fn test_serialize_h3_atx() {
        let result = parse_and_serialize("### Subsection");
        assert_eq!(result, "### Subsection\n");
    }

    #[test]
    fn test_serialize_h4_atx() {
        let result = parse_and_serialize("#### Deep Subsection");
        assert_eq!(result, "#### Deep Subsection\n");
    }

    #[test]
    fn test_serialize_unordered_list_single_item() {
        let result = parse_and_serialize("- Item one");
        assert_eq!(result, " -  Item one\n");
    }

    #[test]
    fn test_serialize_unordered_list_multiple_items() {
        let result = parse_and_serialize("- Item one\n- Item two\n- Item three");
        assert_eq!(result, " -  Item one\n -  Item two\n -  Item three\n");
    }

    #[test]
    fn test_serialize_ordered_list_single_item() {
        let result = parse_and_serialize("1. First item");
        assert_eq!(result, " 1. First item\n");
    }

    #[test]
    fn test_serialize_ordered_list_multiple_items() {
        let result = parse_and_serialize("1. First\n2. Second\n3. Third");
        assert_eq!(result, " 1. First\n 2. Second\n 3. Third\n");
    }

    #[test]
    fn test_serialize_fenced_code_block() {
        let result = parse_and_serialize("```rust\nfn main() {}\n```");
        assert_eq!(result, "~~~~ rust\nfn main() {}\n~~~~\n");
    }

    #[test]
    fn test_serialize_fenced_code_block_no_language() {
        let result = parse_and_serialize("```\nsome code\n```");
        assert_eq!(result, "~~~~ text\nsome code\n~~~~\n");
    }

    #[test]
    fn test_serialize_fenced_code_block_with_tildes_inside() {
        // When code contains ~~~~, use more tildes for the fence
        let result = parse_and_serialize("```\n~~~~\ninner fence\n~~~~\n```");
        assert_eq!(result, "~~~~~ text\n~~~~\ninner fence\n~~~~\n~~~~~\n");
    }

    #[test]
    fn test_serialize_block_quote_single_line() {
        let result = parse_and_serialize("> This is a quote.");
        assert_eq!(result, "> This is a quote.\n");
    }

    #[test]
    fn test_serialize_block_quote_multiple_lines() {
        let result = parse_and_serialize("> Line one.\n> Line two.");
        assert_eq!(result, "> Line one. Line two.\n");
    }

    #[test]
    fn test_serialize_block_quote_multiple_paragraphs() {
        let result = parse_and_serialize("> First paragraph.\n>\n> Second paragraph.");
        assert_eq!(result, "> First paragraph.\n>\n> Second paragraph.\n");
    }

    #[test]
    fn test_serialize_emphasis() {
        let result = parse_and_serialize("This is *emphasized* text.");
        assert_eq!(result, "This is *emphasized* text.\n");
    }

    #[test]
    fn test_serialize_strong() {
        let result = parse_and_serialize("This is **strong** text.");
        assert_eq!(result, "This is **strong** text.\n");
    }

    #[test]
    fn test_serialize_inline_code() {
        let result = parse_and_serialize("Use the `format()` function.");
        assert_eq!(result, "Use the `format()` function.\n");
    }

    #[test]
    fn test_serialize_inline_link() {
        let result = parse_and_serialize("Visit [Rust](https://www.rust-lang.org/).");
        assert_eq!(result, "Visit [Rust](https://www.rust-lang.org/).\n");
    }

    #[test]
    fn test_serialize_inline_link_with_title() {
        let result =
            parse_and_serialize("Visit [Rust](https://www.rust-lang.org/ \"The Rust Language\").");
        assert_eq!(
            result,
            "Visit [Rust](https://www.rust-lang.org/ \"The Rust Language\").\n"
        );
    }

    fn parse_and_serialize_with_frontmatter(input: &str) -> String {
        let arena = Arena::new();
        let mut options = ComrakOptions::default();
        options.extension.front_matter_delimiter = Some("---".to_string());
        let root = parse_document(&arena, input, &options);
        let format_options = Options::default();
        serialize(root, &format_options)
    }

    #[test]
    fn test_serialize_yaml_front_matter() {
        let input = "---\ntitle: Hello\nauthor: World\n---\n\n# Heading";
        let result = parse_and_serialize_with_frontmatter(input);
        assert_eq!(result, "---\ntitle: Hello\nauthor: World\n---\n\nHeading\n=======\n");
    }

    #[test]
    fn test_serialize_yaml_front_matter_only() {
        let input = "---\ntitle: Test\n---\n\nSome content.";
        let result = parse_and_serialize_with_frontmatter(input);
        assert_eq!(result, "---\ntitle: Test\n---\n\nSome content.\n");
    }

    fn parse_and_serialize_with_width(input: &str, line_width: usize) -> String {
        let arena = Arena::new();
        let options = ComrakOptions::default();
        let root = parse_document(&arena, input, &options);
        let format_options = Options { line_width };
        serialize(root, &format_options)
    }

    #[test]
    fn test_serialize_paragraph_wrap_at_80() {
        // A long line that should wrap at approximately 80 characters
        let input = "This is a very long paragraph that should be wrapped at approximately eighty characters to maintain readability.";
        let result = parse_and_serialize_with_width(input, 80);
        // The line should be wrapped
        assert!(result.contains('\n'));
        // Each line should be at most 80 characters (approximately)
        for line in result.lines() {
            assert!(
                line.len() <= 85,
                "Line too long: {} chars",
                line.len()
            );
        }
    }

    #[test]
    fn test_serialize_paragraph_no_wrap_short() {
        // A short line that should not be wrapped
        let input = "Short paragraph.";
        let result = parse_and_serialize_with_width(input, 80);
        assert_eq!(result, "Short paragraph.\n");
    }

    #[test]
    fn test_serialize_paragraph_wrap_preserves_words() {
        // Words should not be broken
        let input = "Word1 Word2 Word3 Word4 Word5 Word6 Word7 Word8 Word9 Word10 Word11 Word12 Word13 Word14 Word15";
        let result = parse_and_serialize_with_width(input, 40);
        // Check that words are not broken
        for line in result.lines() {
            assert!(
                !line.ends_with('-'),
                "Words should not be hyphenated"
            );
        }
    }
}
