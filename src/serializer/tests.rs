use super::*;
use crate::{LineWidth, ThematicBreakStyle};
use comrak::{Arena, Options as ComrakOptions, parse_document};
use unicode_width::UnicodeWidthStr;

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

fn parse_and_serialize(input: &str) -> String {
    let arena = Arena::new();
    let options = comrak_options();
    let root = parse_document(&arena, input, &options);
    let format_options = Options::default();
    serialize_with_source(root, &format_options, None)
}

fn parse_and_serialize_with_options(input: &str, format_options: &Options) -> String {
    let arena = Arena::new();
    let options = comrak_options();
    let root = parse_document(&arena, input, &options);
    serialize_with_source(root, format_options, None)
}

fn parse_and_serialize_with_source(input: &str) -> String {
    let arena = Arena::new();
    let options = comrak_options();
    let root = parse_document(&arena, input, &options);
    let format_options = Options::default();
    serialize_with_source(root, &format_options, Some(input))
}

fn parse_and_serialize_with_source_and_width(input: &str, line_width: usize) -> String {
    let arena = Arena::new();
    let options = comrak_options();
    let root = parse_document(&arena, input, &options);
    let format_options = Options {
        line_width: Some(LineWidth::new(line_width).unwrap()),
        ..Options::default()
    };
    serialize_with_source(root, &format_options, Some(input))
}

fn parse_and_serialize_no_wrap(input: &str) -> String {
    let arena = Arena::new();
    let options = comrak_options();
    let root = parse_document(&arena, input, &options);
    let format_options = Options {
        line_width: None,
        ..Options::default()
    };
    serialize_with_source(root, &format_options, None)
}

fn parse_and_serialize_with_warnings(input: &str) -> SerializeResult {
    parse_and_serialize_with_warnings_and_options(input, &Options::default())
}

fn parse_and_serialize_with_warnings_and_options(
    input: &str,
    format_options: &Options,
) -> SerializeResult {
    let arena = Arena::new();
    let options = comrak_options();
    let root = parse_document(&arena, input, &options);
    serialize_with_source_and_warnings(root, format_options, Some(input))
}

#[test]
fn test_serialize_plain_text() {
    let result = parse_and_serialize("Hello, world!");
    assert_eq!(result, "Hello, world!\n");
}

#[test]
fn test_serialize_multiline_paragraph() {
    // Original line breaks are preserved when lines are under 80 chars
    let result = parse_and_serialize("Hello\nworld!");
    assert_eq!(result, "Hello\nworld!\n");
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
    // trailing_spaces=2, so "1.  " (number, marker, trailing=2)
    let result = parse_and_serialize("1. First item");
    assert_eq!(result, "1.  First item\n");
}

#[test]
fn test_serialize_ordered_list_multiple_items() {
    // trailing_spaces=2, so "N.  " format
    let result = parse_and_serialize("1. First\n2. Second\n3. Third");
    assert_eq!(result, "1.  First\n2.  Second\n3.  Third\n");
}

#[test]
fn test_serialize_tight_list() {
    // Tight list: no blank lines between items
    let input = " -  Item one\n -  Item two\n -  Item three";
    let result = parse_and_serialize(input);
    assert_eq!(result, " -  Item one\n -  Item two\n -  Item three\n");
}

#[test]
fn test_serialize_loose_list() {
    // Loose list: blank lines between items should be preserved
    let input = " -  Item one\n\n -  Item two\n\n -  Item three";
    let result = parse_and_serialize(input);
    assert_eq!(
        result, " -  Item one\n\n -  Item two\n\n -  Item three\n",
        "Loose list should have blank lines between items"
    );
}

#[test]
fn test_serialize_loose_list_with_content() {
    // Loose list with multi-line content
    let input = " -  *Zero dependencies*: LogTape has zero dependencies.\n\n -  *Library support*: Designed for libraries.";
    let result = parse_and_serialize(input);
    assert!(
        result.contains(" -  *Zero dependencies*"),
        "Should contain first item"
    );
    assert!(
        result.contains("\n\n -  *Library support*"),
        "Should have blank line before second item, got:\n{}",
        result
    );
}

#[test]
fn test_serialize_fenced_code_block() {
    let result = parse_and_serialize("```rust\nfn main() {}\n```");
    assert_eq!(result, "~~~~ rust\nfn main() {}\n~~~~\n");
}

#[test]
fn test_serialize_fenced_code_block_preserves_html_entities_in_info_string() {
    let input = "```c++ title=&quot;main.cpp&quot;\nint main() {}\n```";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(
        result,
        "~~~~ c++ title=&quot;main.cpp&quot;\nint main() {}\n~~~~\n"
    );
}

#[test]
fn test_serialize_fenced_code_block_no_language() {
    // Code block without language should remain without language identifier
    let result = parse_and_serialize("```\nsome code\n```");
    assert_eq!(result, "~~~~\nsome code\n~~~~\n");
}

#[test]
fn test_serialize_fenced_code_block_with_tildes_inside() {
    // When code contains ~~~~, use more tildes for the fence
    let result = parse_and_serialize("```\n~~~~\ninner fence\n~~~~\n```");
    assert_eq!(result, "~~~~~\n~~~~\ninner fence\n~~~~\n~~~~~\n");
}

#[test]
fn test_serialize_block_quote_single_line() {
    let result = parse_and_serialize("> This is a quote.");
    assert_eq!(result, "> This is a quote.\n");
}

#[test]
fn test_serialize_block_quote_multiple_lines() {
    // Original line breaks are preserved when lines are under 80 chars
    let result = parse_and_serialize("> Line one.\n> Line two.");
    assert_eq!(result, "> Line one.\n> Line two.\n");
}

#[test]
fn test_serialize_block_quote_multiple_paragraphs() {
    let result = parse_and_serialize("> First paragraph.\n>\n> Second paragraph.");
    assert_eq!(result, "> First paragraph.\n>\n> Second paragraph.\n");
}

#[test]
fn test_loose_list_in_block_quote_stays_in_one_quote() {
    let input = r#"> This is a block quote.
>
> - And this is a list item in the block quote.
>
> - And this is another list item in the block quote."#;
    let result = parse_and_serialize(input);
    assert_eq!(
        result,
        "> This is a block quote.\n>\n>  -  And this is a list item in the block quote.\n>\n>  -  And this is another list item in the block quote.\n"
    );
}

#[test]
fn test_serialize_emphasis() {
    let result = parse_and_serialize("This is *emphasized* text.");
    assert_eq!(result, "This is *emphasized* text.\n");
}

#[test]
fn test_serialize_underscore_emphasis_preserved() {
    // Underscore emphasis should be preserved as underscore, not converted to asterisk
    let result = parse_and_serialize_with_source("This is _emphasized_ text.");
    assert_eq!(result, "This is _emphasized_ text.\n");
}

#[test]
fn test_serialize_mixed_emphasis_preserved() {
    // Mixed emphasis styles should each be preserved
    let result = parse_and_serialize_with_source("This is _underscore_ and *asterisk* emphasis.");
    assert_eq!(result, "This is _underscore_ and *asterisk* emphasis.\n");
}

#[test]
fn test_serialize_emphasis_with_asterisk_uses_underscore() {
    // When emphasis content contains an asterisk, use underscore delimiter
    // to avoid escaping the asterisk inside
    let result = parse_and_serialize(r"This is *foo\*bar* text.");
    assert_eq!(result, "This is _foo\\*bar_ text.\n");
}

#[test]
fn test_serialize_strong_with_asterisk_uses_underscore() {
    // When strong content contains an asterisk, use underscore delimiter
    let result = parse_and_serialize(r"This is **foo\*bar** text.");
    assert_eq!(result, "This is __foo\\*bar__ text.\n");
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
fn test_serialize_external_link_becomes_reference() {
    // External links (https://) are converted to reference style
    let result = parse_and_serialize("Visit [Rust](https://www.rust-lang.org/).");
    assert!(result.contains("Visit [Rust]."));
    assert!(result.contains("[Rust]: https://www.rust-lang.org/"));
}

#[test]
fn test_serialize_external_link_with_title_becomes_reference() {
    // External links with titles are also converted to reference style
    let result =
        parse_and_serialize("Visit [Rust](https://www.rust-lang.org/ \"The Rust Language\").");
    assert!(result.contains("Visit [Rust]."));
    assert!(result.contains("[Rust]: https://www.rust-lang.org/ \"The Rust Language\""));
}

#[test]
fn test_reference_destinations_requiring_angle_brackets() {
    // https://github.com/dahlia/hongdown/issues/25
    let input = concat!(
        " -  Read [space].\n",
        " -  Read [empty].\n\n",
        "[space]: <https://example.com/a b> \"A title\"\n",
        "[empty]: <>\n",
    );
    let expected = concat!(
        " -  Read [space].\n",
        " -  Read [empty].\n\n",
        "[space]: <https://example.com/a b> \"A title\"\n",
        "[empty]: <>\n",
    );

    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(result.output, expected);
    assert!(result.warnings.is_empty());

    let second = parse_and_serialize_with_warnings(&result.output);
    assert_eq!(second.output, result.output);
    assert!(second.warnings.is_empty());
}

#[test]
fn test_angle_bracketed_reference_destination_escaping_is_idempotent() {
    let input = concat!(
        " -  Read [escaped].\n",
        " -  Read [backslash].\n",
        " -  Read [entity].\n",
        " -  Read [line break].\n",
        " -  Read [carriage return].\n",
        " -  Read [surrogate].\n",
        " -  Read [out of range].\n",
        " -  Read [overflow].\n\n",
        "[escaped]: <https://example.com/a\\> b>\n",
        "[backslash]: <https://example.com/a b\\\\*c>\n",
        "[entity]: <https://example.com/a b&amp;#10;>\n",
        "[line break]: <https://example.com/a&#10;b>\n",
        "[carriage return]: <https://example.com/a&#13;b>\n",
        "[surrogate]: <a&amp;#xD800;b>\n",
        "[out of range]: <a&amp;#x110000;b>\n",
        "[overflow]: <a&amp;#99999999;b>\n",
    );

    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(result.output, input);
    assert!(result.warnings.is_empty());

    let second = parse_and_serialize_with_warnings(&result.output);
    assert_eq!(second.output, result.output);
    assert!(second.warnings.is_empty());
}

#[test]
fn test_reference_destinations_not_safe_bare_use_angle_brackets() {
    let input = concat!(
        " -  Read [leading angle].\n",
        " -  Read [vertical tab].\n",
        " -  Read [open paren].\n",
        " -  Read [backslash].\n",
        " -  Read [entity].\n",
        " -  Read [balanced].\n\n",
        "[leading angle]: <\\<foo\\>>\n",
        "[vertical tab]: <a&#11;b>\n",
        "[open paren]: <a(b>\n",
        "[backslash]: <a\\\\*b>\n",
        "[entity]: <a&amp;#10;>\n",
        "[balanced]: <a(b)>\n",
    );
    let expected = concat!(
        " -  Read [leading angle].\n",
        " -  Read [vertical tab].\n",
        " -  Read [open paren].\n",
        " -  Read [backslash].\n",
        " -  Read [entity].\n",
        " -  Read [balanced].\n\n",
        "[leading angle]: <\\<foo\\>>\n",
        "[vertical tab]: <a\u{b}b>\n",
        "[open paren]: <a(b>\n",
        "[backslash]: <a\\\\*b>\n",
        "[entity]: <a&amp;#10;>\n",
        "[balanced]: a(b)\n",
    );

    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(result.output, expected);
    assert!(result.warnings.is_empty());

    let second = parse_and_serialize_with_warnings(&result.output);
    assert_eq!(second.output, result.output);
    assert!(second.warnings.is_empty());
}

#[test]
fn test_bare_reference_destination_parenthesis_limit() {
    let at_limit = format!("a{}{}", "(".repeat(32), ")".repeat(32));
    assert!(!reference_destination_requires_angle_brackets(&at_limit));

    let beyond_limit = format!("a{}{}", "(".repeat(33), ")".repeat(33));
    assert!(reference_destination_requires_angle_brackets(&beyond_limit));
}

#[test]
fn test_reference_order_preserved() {
    // Regular references should maintain insertion order
    let input = "See [foo](https://foo.com), [bar](https://bar.com), and [baz](https://baz.com).";
    let result = parse_and_serialize(input);
    // Find positions of references
    let foo_pos = result.find("[foo]:").unwrap();
    let bar_pos = result.find("[bar]:").unwrap();
    let baz_pos = result.find("[baz]:").unwrap();
    assert!(
        foo_pos < bar_pos && bar_pos < baz_pos,
        "References should be in insertion order, got:\n{}",
        result
    );
}

#[test]
fn test_numeric_references_sorted_at_end() {
    // Numeric references should be sorted by number and placed at the end
    let input = "See [foo](https://foo.com), [2](https://2.com), [bar](https://bar.com), [1](https://1.com).";
    let result = parse_and_serialize(input);
    // foo and bar should come before numeric refs
    let foo_pos = result.find("[foo]:").unwrap();
    let bar_pos = result.find("[bar]:").unwrap();
    let one_pos = result.find("[1]:").unwrap();
    let two_pos = result.find("[2]:").unwrap();
    // Regular refs first, in order
    assert!(foo_pos < bar_pos, "foo should come before bar");
    // Numeric refs at end, sorted by number
    assert!(
        bar_pos < one_pos,
        "Regular refs should come before numeric refs"
    );
    assert!(
        one_pos < two_pos,
        "Numeric refs should be sorted: 1 before 2, got:\n{}",
        result
    );
}

#[test]
fn test_single_numeric_reference_not_sorted() {
    // A single numeric reference should stay in insertion order
    let input = "See [foo](https://foo.com), [1](https://1.com), [bar](https://bar.com).";
    let result = parse_and_serialize(input);
    let foo_pos = result.find("[foo]:").unwrap();
    let one_pos = result.find("[1]:").unwrap();
    let bar_pos = result.find("[bar]:").unwrap();
    // With only one numeric ref, it stays in insertion order
    assert!(
        foo_pos < one_pos && one_pos < bar_pos,
        "Single numeric ref should stay in insertion order, got:\n{}",
        result
    );
}

#[test]
fn test_hash_numeric_references_sorted() {
    // References like #123 should also be sorted numerically
    let input = "See [#456](https://issue/456) and [#123](https://issue/123).";
    let result = parse_and_serialize(input);
    let pos_123 = result.find("[#123]:").unwrap();
    let pos_456 = result.find("[#456]:").unwrap();
    assert!(
        pos_123 < pos_456,
        "#123 should come before #456, got:\n{}",
        result
    );
}

fn parse_and_serialize_with_frontmatter(input: &str) -> String {
    let arena = Arena::new();
    let mut options = ComrakOptions::default();
    options.extension.front_matter_delimiter = Some("---".to_string());
    let root = parse_document(&arena, input, &options);
    let format_options = Options::default();
    serialize_with_source(root, &format_options, None)
}

#[test]
fn test_serialize_yaml_front_matter() {
    let input = "---\ntitle: Hello\nauthor: World\n---\n\n# Heading";
    let result = parse_and_serialize_with_frontmatter(input);
    assert_eq!(
        result,
        "---\ntitle: Hello\nauthor: World\n---\n\nHeading\n=======\n"
    );
}

#[test]
fn test_serialize_yaml_front_matter_only() {
    let input = "---\ntitle: Test\n---\n\nSome content.";
    let result = parse_and_serialize_with_frontmatter(input);
    assert_eq!(result, "---\ntitle: Test\n---\n\nSome content.\n");
}

#[test]
fn test_serialize_two_blank_lines_before_h2() {
    let input = "# Title\n\nParagraph.\n\n## Section";
    let result = parse_and_serialize(input);
    // Should have two blank lines before h2 (one after paragraph + one extra)
    assert!(result.contains("Paragraph.\n\n\nSection"));
}

#[test]
fn test_serialize_one_blank_line_for_empty_section() {
    // When h1 is immediately followed by h2 (empty section), only one blank line
    let input = "# Title\n\n## Section\n\nContent.";
    let result = parse_and_serialize(input);
    // Should have only one blank line between headings
    assert_eq!(result, "Title\n=====\n\nSection\n-------\n\nContent.\n");
}

#[test]
fn test_serialize_consecutive_h2_sections() {
    // When h2 is immediately followed by another h2 (empty section)
    let input = "## Section 1\n\n## Section 2\n\nContent.";
    let result = parse_and_serialize(input);
    // Should have only one blank line between headings
    assert_eq!(
        result,
        "Section 1\n---------\n\nSection 2\n---------\n\nContent.\n"
    );
}

fn parse_and_serialize_with_width(input: &str, line_width: usize) -> String {
    let arena = Arena::new();
    let options = ComrakOptions::default();
    let root = parse_document(&arena, input, &options);
    let format_options = Options {
        line_width: Some(LineWidth::new(line_width).unwrap()),
        ..Options::default()
    };
    serialize_with_source(root, &format_options, None)
}

fn assert_all_non_empty_lines_fit_display_width(output: &str, line_width: usize) {
    for line in output.lines().filter(|line| !line.is_empty()) {
        assert!(
            line.width() <= line_width,
            "Line exceeds width {} (actual {}): {:?}\nFull output:\n{}",
            line_width,
            line.width(),
            line,
            output
        );
    }
}

#[test]
fn test_heading_with_inline_code() {
    // Inline code in headings should be preserved
    let input = "# Heading with `code`";
    let result = parse_and_serialize(input);
    assert_eq!(result, "Heading with `code`\n===================\n");
}

#[test]
fn test_heading_with_multiple_inline_codes() {
    // Multiple inline codes in headings
    let input = "### Looking at the `to`, `cc`, and `bcc` fields";
    let result = parse_and_serialize(input);
    assert_eq!(result, "### Looking at the `to`, `cc`, and `bcc` fields\n");
}

#[test]
fn test_korean_in_link() {
    // Korean text in links should not cause panic
    let input = "[한국어](https://example.com)";
    let result = parse_and_serialize(input);
    assert!(result.contains("[한국어]"));
    assert!(result.contains("https://example.com"));
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
        assert!(line.len() <= 85, "Line too long: {} chars", line.len());
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
        assert!(!line.ends_with('-'), "Words should not be hyphenated");
    }
}

#[test]
fn test_selective_rewrap_short_lines_preserved() {
    // Short lines (under 80 chars) should be preserved as-is
    let input = "Line one.\nLine two.\nLine three.";
    let result = parse_and_serialize(input);
    // Each line should stay on its own line
    assert_eq!(
        result, "Line one.\nLine two.\nLine three.\n",
        "Short lines should be preserved"
    );
}

#[test]
fn test_selective_rewrap_long_line_wrapped() {
    // A line over 80 chars should be rewrapped
    let input = "This is a very long line that definitely exceeds the eighty character limit and should be wrapped to the next line properly.";
    let result = parse_and_serialize_with_width(input, 80);
    // Should be wrapped
    let lines: Vec<&str> = result.lines().collect();
    assert!(
        lines.len() > 1,
        "Long line should be wrapped, got:\n{}",
        result
    );
    // Each line should be under 80 chars
    for line in &lines {
        assert!(line.len() <= 80, "Line should be under 80 chars: {}", line);
    }
}

#[test]
fn test_selective_rewrap_mixed_lines() {
    // Mix of short and long lines - short should be preserved, long rewrapped
    let input = "Short line one.\nShort line two.\nThis is a very long line that definitely exceeds the eighty character limit and needs to be wrapped.";
    let result = parse_and_serialize_with_width(input, 80);
    // Short lines should be preserved
    assert!(
        result.starts_with("Short line one.\nShort line two.\n"),
        "Short lines should be preserved at start, got:\n{}",
        result
    );
}

fn parse_and_serialize_with_table(input: &str) -> String {
    let arena = Arena::new();
    let mut options = ComrakOptions::default();
    options.extension.table = true;
    let root = parse_document(&arena, input, &options);
    let format_options = Options::default();
    serialize_with_source(root, &format_options, None)
}

#[test]
fn test_serialize_simple_table() {
    let input = "| A | B |\n|---|---|\n| 1 | 2 |";
    let result = parse_and_serialize_with_table(input);
    assert!(result.contains("| A"));
    assert!(result.contains("| B"));
    assert!(result.contains("| 1"));
    assert!(result.contains("| 2"));
}

#[test]
fn test_serialize_table_with_alignment() {
    let input = "| Left | Center | Right |\n|:-----|:------:|------:|\n| L | C | R |";
    let result = parse_and_serialize_with_table(input);
    // Should contain alignment markers
    assert!(result.contains(":--"));
    assert!(result.contains("--:"));
}

#[test]
fn test_serialize_table_right_aligned_cell_data() {
    // Right-aligned columns should have data cells right-aligned (padded on the left)
    let input = "| Name | Value |\n| ---- | ----: |\n| A | 1 |\n| BB | 22 |";
    let result = parse_and_serialize_with_table(input);
    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(lines.len(), 4, "Table should have 4 lines");

    // The Value column is right-aligned with width 5 ("Value" length)
    // "1" should be right-aligned: "|     1 |" (4 spaces + 1)
    // "22" should be right-aligned: "|    22 |" (3 spaces + 22)
    // The data rows are lines[2] and lines[3]
    assert!(
        lines[2].contains("|     1 |"),
        "Right-aligned column data should be right-aligned (padded on left), got:\n{}",
        result
    );
    assert!(
        lines[3].contains("|    22 |"),
        "Right-aligned column data should be right-aligned (padded on left), got:\n{}",
        result
    );
}

#[test]
fn test_serialize_table_center_aligned_cell_data() {
    // Center-aligned columns should have data cells center-aligned
    let input = "| Name | Value |\n| ---- | :---: |\n| A | 1 |\n| BB | 22 |";
    let result = parse_and_serialize_with_table(input);
    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(lines.len(), 4, "Table should have 4 lines");

    // The Value column is center-aligned with width 5 ("Value" length)
    // Cell format is: "| " + content + " |", where content is centered
    // "1" centered in width 5: "  1  " -> "|   1   |"
    // "22" centered in width 5: " 22  " (1 left, 2 right) -> "|  22   |"
    // Note: Rust's {:^} adds extra padding on right when asymmetric
    assert!(
        lines[2].contains("|   1   |"),
        "Center-aligned column data should be centered, got:\n{}",
        result
    );
    assert!(
        lines[3].contains("|  22   |"),
        "Center-aligned column data should be centered, got:\n{}",
        result
    );
}

#[test]
fn test_serialize_table_aligned_columns() {
    let input = "| Name | Age |\n|------|-----|\n| Alice | 30 |\n| Bob | 25 |";
    let result = parse_and_serialize_with_table(input);
    // Columns should be aligned with padding
    let lines: Vec<&str> = result.lines().collect();
    // All rows should have the same pipe positions (aligned)
    if lines.len() >= 3 {
        // Find pipe positions in first data row
        let first_pipes: Vec<_> = lines[0].match_indices('|').map(|(i, _)| i).collect();
        // Verify other rows have pipes in similar positions (allowing for padding)
        for line in &lines[1..] {
            let pipes: Vec<_> = line.match_indices('|').map(|(i, _)| i).collect();
            assert_eq!(
                first_pipes.len(),
                pipes.len(),
                "All rows should have same number of pipes"
            );
        }
    }
}

#[test]
fn test_serialize_table_with_links() {
    // Table cells containing links should preserve the links
    let input = "| Package | Link |\n|---------|------|\n| [foo](/foo) | [bar](https://bar.com) |";
    let result = parse_and_serialize_with_table(input);
    // Links should be preserved in table cells
    assert!(
        result.contains("[foo](/foo)"),
        "Relative link should be preserved in table, got:\n{}",
        result
    );
    assert!(
        result.contains("[bar]"),
        "External link text should be preserved in table, got:\n{}",
        result
    );
}

#[test]
fn test_serialize_table_with_reference_links() {
    // Table cells containing reference-style links should preserve them
    let input = "| Package | JSR |\n|---------|-----|\n| [*@pkg/core*](/packages/core/) | [JSR][jsr:@pkg/core] |\n\n[jsr:@pkg/core]: https://jsr.io/@pkg/core";
    let result = parse_and_serialize_with_source(input);
    // Reference links should be preserved in table cells
    assert!(
        result.contains("[*@pkg/core*](/packages/core/)"),
        "Link with emphasis should be preserved in table, got:\n{}",
        result
    );
    assert!(
        result.contains("[JSR][jsr:@pkg/core]"),
        "Reference-style link should be preserved in table, got:\n{}",
        result
    );
}

#[test]
fn test_serialize_table_with_pipe_in_code_span() {
    // Table cells containing code spans with pipe characters should preserve the closing backtick.
    // This tests the fix for comrak sourcepos bug with escaped pipes in code spans.
    let input = "| Option | Type |\n|--------|------|\n| `foo` | `string \\| number` |";
    let result = parse_and_serialize_with_table(input);
    assert!(
        result.contains("`string \\| number`"),
        "Code span with pipe should have closing backtick preserved, got:\n{}",
        result
    );
    assert!(
        result.contains("`foo`"),
        "Simple code span should be preserved, got:\n{}",
        result
    );
}

#[test]
fn test_serialize_table_with_multiple_pipes_in_code_span() {
    // Test multiple pipe characters in a single code span
    let input = "| Field | Type |\n|-------|------|\n| `val` | `\"a\" \\| \"b\" \\| \"c\"` |";
    let result = parse_and_serialize_with_table(input);
    assert!(
        result.contains("`\"a\" \\| \"b\" \\| \"c\"`"),
        "Code span with multiple pipes should have closing backtick preserved, got:\n{}",
        result
    );
}

#[test]
fn test_table_inside_list_item_has_blank_line_and_indentation() {
    let input = r#"4.  **Properties of embedded types** — when a property's value is an object type
    that is **serialized inline** (not just referenced by URL), the context must
    also cover all of that embedded type's properties. This is the most commonly
    missed case.

    Common embedded types and the context URLs that cover them:

    | Embedded type                       | Context URL to include                        |
    | ----------------------------------- | --------------------------------------------- |
    | `DataIntegrityProof` (from `proof`) | `https://w3id.org/security/data-integrity/v1` |
    | `Key` (from `publicKey`)            | `https://w3id.org/security/v1`                |
"#;

    let first_pass = parse_and_serialize(input);

    assert!(
        first_pass.contains("cover them:\n\n    | Embedded type                       | Context URL to include                        |"),
        "Table should be separated from the paragraph by a blank line and indented as list item content, got:\n{}",
        first_pass
    );
    assert!(
        first_pass.contains("\n    | ----------------------------------- | --------------------------------------------- |"),
        "Table separator row should be indented inside list item, got:\n{}",
        first_pass
    );
    assert!(
        first_pass.contains("\n    | `DataIntegrityProof` (from `proof`) | `https://w3id.org/security/data-integrity/v1` |"),
        "Table data row should be indented inside list item, got:\n{}",
        first_pass
    );

    let second_pass = parse_and_serialize(&first_pass);
    assert_eq!(
        first_pass, second_pass,
        "Formatting should be idempotent.\nFirst pass:\n{}\nSecond pass:\n{}",
        first_pass, second_pass
    );
}

#[test]
fn test_table_with_inline_math_aligns_columns() {
    // Inline math is bracketed with wrap sentinels during cell collection; those
    // sentinels must not inflate the table column widths or the pipes misalign.
    let input = "\
| Col | Math |
| --- | --- |
| a | $x^2$ |
| bb | cc |
";
    let result = parse_and_serialize(input);
    let expected = "\
| Col | Math  |
| --- | ----- |
| a   | $x^2$ |
| bb  | cc    |
";
    assert_eq!(result, expected);
}

fn parse_and_serialize_with_description_list(input: &str) -> String {
    let arena = Arena::new();
    let mut options = ComrakOptions::default();
    options.extension.description_lists = true;
    let root = parse_document(&arena, input, &options);
    let format_options = Options::default();
    serialize_with_source(root, &format_options, None)
}

#[test]
fn test_serialize_definition_list_single() {
    let input = "Term\n:   Definition";
    let result = parse_and_serialize_with_description_list(input);
    assert!(result.contains("Term\n"));
    assert!(result.contains(":   Definition"));
}

#[test]
fn test_serialize_definition_list_multiple_definitions() {
    let input = "Term\n:   First definition\n:   Second definition";
    let result = parse_and_serialize_with_description_list(input);
    assert!(result.contains("Term\n"));
    assert!(result.contains(":   First definition"));
    assert!(result.contains(":   Second definition"));
}

fn parse_and_serialize_with_alerts(input: &str) -> String {
    let arena = Arena::new();
    let mut options = ComrakOptions::default();
    options.extension.alerts = true;
    let root = parse_document(&arena, input, &options);
    let format_options = Options::default();
    serialize_with_source(root, &format_options, None)
}

#[test]
fn test_serialize_github_note_alert() {
    let input = "> [!NOTE]\n> This is a note.";
    let result = parse_and_serialize_with_alerts(input);
    assert!(result.contains("> [!NOTE]"));
    assert!(result.contains("> This is a note."));
}

#[test]
fn test_serialize_github_warning_alert() {
    let input = "> [!WARNING]\n> This is a warning.";
    let result = parse_and_serialize_with_alerts(input);
    assert!(result.contains("> [!WARNING]"));
    assert!(result.contains("> This is a warning."));
}

#[test]
fn test_serialize_github_caution_alert() {
    let input = "> [!CAUTION]\n> Be careful!";
    let result = parse_and_serialize_with_alerts(input);
    assert!(result.contains("> [!CAUTION]"));
    assert!(result.contains("> Be careful!"));
}

fn parse_and_serialize_with_footnotes(input: &str) -> String {
    let arena = Arena::new();
    let mut options = ComrakOptions::default();
    options.extension.footnotes = true;
    let root = parse_document(&arena, input, &options);
    let format_options = Options::default();
    serialize_with_source(root, &format_options, None)
}

fn parse_and_serialize_with_footnotes_and_options(input: &str, format_options: &Options) -> String {
    let arena = Arena::new();
    let mut options = ComrakOptions::default();
    options.extension.footnotes = true;
    let root = parse_document(&arena, input, &options);
    serialize_with_source(root, format_options, None)
}

#[test]
fn test_serialize_footnote_reference() {
    let input = "This has a footnote[^1].\n\n[^1]: The footnote text.";
    let result = parse_and_serialize_with_footnotes(input);
    assert!(result.contains("[^1]"));
}

#[test]
fn test_serialize_footnote_definition() {
    let input = "Text[^note].\n\n[^note]: A named footnote.";
    let result = parse_and_serialize_with_footnotes(input);
    assert!(result.contains("[^note]"));
}

#[test]
fn test_serialize_double_space_after_period() {
    // Hong's style uses two spaces after periods
    let input = "First sentence.  Second sentence.";
    let result = parse_and_serialize(input);
    // Should preserve double spaces
    assert_eq!(result, "First sentence.  Second sentence.\n");
}

#[test]
fn test_definition_list_with_nested_list_continuation() {
    let input = "Term\n:   Definition:\n\n     -  Item with long text\n        that continues";
    let result = parse_and_serialize(input);
    // Continuation should also have proper indent
    // List inside description details: `     -  ` = 5 spaces + `-` + 2 spaces = 8 chars
    // So continuation lines should be indented with 8 spaces
    assert!(result.contains("     -  Item with long text"));
    assert!(
        result.contains("        that continues"),
        "Continuation line should be indented with 8 spaces, got:\n{}",
        result
    );
}

#[test]
fn test_alert_preserves_blank_line_after_header() {
    let input = "> [!TIP]\n>\n> This is a tip.";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(result, "> [!TIP]\n>\n> This is a tip.\n");
}

#[test]
fn test_alert_without_blank_line_after_header() {
    let input = "> [!NOTE]\n> This is a note.";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(result, "> [!NOTE]\n> This is a note.\n");
}

fn parse_and_serialize_with_alerts_and_width(input: &str, line_width: usize) -> String {
    let arena = Arena::new();
    let mut options = ComrakOptions::default();
    options.extension.alerts = true;
    let root = parse_document(&arena, input, &options);
    let format_options = Options {
        line_width: Some(LineWidth::new(line_width).unwrap()),
        ..Options::default()
    };
    serialize_with_source(root, &format_options, None)
}

#[test]
fn test_serialize_list_in_alert() {
    // Lists inside alerts should have proper prefixing
    let input = "> [!NOTE]\n>  -  First item\n>  -  Second item";
    let result = parse_and_serialize_with_alerts(input);
    assert!(result.contains("> [!NOTE]"));
    assert!(result.contains(">  -  First item"));
    assert!(result.contains(">  -  Second item"));
}

#[test]
fn test_serialize_long_list_item_in_alert() {
    // Long list items in alerts should wrap with proper continuation prefix
    let input = "> [!NOTE]\n>  -  This is a very long list item that should wrap properly inside the alert block.";
    let result = parse_and_serialize_with_alerts_and_width(input, 60);
    // Should wrap with ">     " continuation (> + 4 spaces)
    assert!(result.contains(">  -  This is a very long"));
    assert!(result.contains("\n>     ")); // Continuation line with > and 4 spaces
}

#[test]
fn test_blockquote_inside_list_item() {
    // Blockquotes inside list items should have proper indentation
    let input = "1.  Item with blockquote:\n\n    > This is quoted text\n    > inside a list item.\n\n2.  Next item.";
    let result = parse_and_serialize_with_alerts(input);
    assert!(result.contains("1.  Item with blockquote:"));
    assert!(result.contains("    > This is quoted text"));
    assert!(result.contains("    > inside a list item."));
    assert!(result.contains("2.  Next item."));
}

#[test]
fn test_alert_inside_list_item() {
    // Alerts inside list items should have proper indentation
    let input =
        "1.  Item with alert:\n\n    > [!IMPORTANT]\n    > Important message.\n\n2.  Next item.";
    let result = parse_and_serialize_with_alerts(input);
    assert!(result.contains("1.  Item with alert:"));
    assert!(result.contains("    > [!IMPORTANT]"));
    assert!(result.contains("    > Important message."));
    assert!(result.contains("2.  Next item."));
}

#[test]
fn test_alert_inside_unordered_list_item() {
    // Alerts inside unordered list items
    let input = " -  Item with alert:\n\n     > [!NOTE]\n     > A note inside a list.";
    let result = parse_and_serialize_with_alerts(input);
    assert!(result.contains(" -  Item with alert:"));
    assert!(result.contains("    > [!NOTE]"));
    assert!(result.contains("    > A note inside a list."));
}

#[test]
fn test_serialize_external_link_as_reference() {
    // External URLs should be converted to reference links
    let input = "Visit [Rust](https://www.rust-lang.org/) for more info.";
    let result = parse_and_serialize(input);
    // Should use reference style, not inline
    assert!(result.contains("[Rust]"));
    assert!(!result.contains("](https://"));
    assert!(result.contains("[Rust]: https://www.rust-lang.org/"));
}

#[test]
fn test_serialize_relative_link_stays_inline() {
    // Relative paths should stay as inline links
    let input = "See the [README](./README.md) for details.";
    let result = parse_and_serialize(input);
    // Should keep inline style for relative paths
    assert!(result.contains("[README](./README.md)"));
}

#[test]
fn test_serialize_reference_links_at_section_end() {
    // Reference definitions should appear at the end of each section
    let input = r#"# Title

See [Example](https://example.com/) here.

## Section One

Visit [Rust](https://www.rust-lang.org/) and [Cargo](https://doc.rust-lang.org/cargo/).

## Section Two

Check [Python](https://python.org/) too.
"#;
    let result = parse_and_serialize(input);
    // Each section should have its references at the end
    assert!(result.contains("[Rust]: https://www.rust-lang.org/"));
    assert!(result.contains("[Cargo]: https://doc.rust-lang.org/cargo/"));
    assert!(result.contains("[Python]: https://python.org/"));
    // References should come before the next section
    let rust_def_pos = result.find("[Rust]: ").unwrap();
    let section_two_pos = result.find("Section Two").unwrap();
    assert!(rust_def_pos < section_two_pos);
}

#[test]
fn test_serialize_shortcut_reference_when_text_matches_label() {
    // When link text matches a sensible label, use shortcut reference [text]
    let input = "Use [comrak](https://docs.rs/comrak) for parsing.";
    let result = parse_and_serialize(input);
    // Should use shortcut reference style
    assert!(result.contains("[comrak]"));
    assert!(result.contains("[comrak]: https://docs.rs/comrak"));
}

#[test]
fn test_serialize_escaped_asterisk_in_emphasis() {
    // When emphasis contains an asterisk, use underscore delimiter
    // This avoids needing to escape the asterisk
    let input = r"*\*.ts*";
    let result = parse_and_serialize(input);
    assert_eq!(result, "_\\*.ts_\n");
}

#[test]
fn test_serialize_escaped_underscore() {
    // Escaped underscores should be preserved
    let input = r"\_\_init\_\_";
    let result = parse_and_serialize(input);
    assert_eq!(result, "\\_\\_init\\_\\_\n");
}

#[test]
fn test_serialize_escaped_underscore_in_emphasis() {
    // Escaped underscores inside emphasis should be preserved
    // This is common for filenames like *node\_modules* where the underscore
    // needs escaping to prevent it from ending the emphasis
    let input = r"*node\_modules*";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(result, "*node\\_modules*\n");
}

#[test]
fn test_ordered_list_with_code_block() {
    // Code blocks inside ordered list items should be indented to align with content
    // The marker "1.  " is 4 characters, so content indent should be 4 spaces
    let input =
        "1.  First item:\n\n    ~~~~ bash\n    echo \"hello\"\n    ~~~~\n\n2.  Second item.\n";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(
        result, input,
        "Code block in ordered list should preserve indentation"
    );
}

#[test]
fn test_unordered_list_with_code_block() {
    // Code blocks inside unordered list items should be indented to align with content
    // The marker " -  " is 4 characters, so content indent should be 4 spaces
    let input =
        " -  First item:\n\n    ~~~~ bash\n    echo \"hello\"\n    ~~~~\n\n -  Second item.\n";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(
        result, input,
        "Code block in unordered list should preserve indentation"
    );
}

#[test]
fn test_double_brackets_preserved() {
    // Double brackets around references (common in changelogs) should not be escaped
    let input = "See [[#123]] for details.\n\n[#123]: https://example.com/123\n";
    let result = parse_and_serialize(input);
    assert_eq!(
        result,
        "See [[#123]] for details.\n\n[#123]: https://example.com/123\n"
    );
}

#[test]
fn test_double_brackets_with_multiple_refs() {
    // Double brackets with multiple references and text
    let input = "[[#120], [#121] by Author]\n\n[#120]: https://example.com/120\n[#121]: https://example.com/121\n";
    let result = parse_and_serialize(input);
    assert_eq!(
        result,
        "[[#120], [#121] by Author]\n\n[#120]: https://example.com/120\n[#121]: https://example.com/121\n"
    );
}

#[test]
fn test_serialize_escaped_backslash() {
    // Escaped backslash should be preserved
    let input = r"path\\to\\file";
    let result = parse_and_serialize(input);
    assert_eq!(result, "path\\\\to\\\\file\n");
}

#[test]
fn test_multi_paragraph_list_item() {
    // Multiple paragraphs within a single list item should be separated by blank lines
    let input = " -  First paragraph.\n\n    Second paragraph.\n\n    Third paragraph.\n";
    let result = parse_and_serialize(input);
    assert_eq!(
        result,
        " -  First paragraph.\n\n    Second paragraph.\n\n    Third paragraph.\n"
    );
}

#[test]
fn test_tight_nested_list() {
    // Nested list directly following text (tight) - no blank line
    // Nested list indent = parent content start position (leading + 1 + trailing = 4)
    // plus its own leading space, so 5 spaces before marker
    let input = " -  Item:\n     -  Nested 1\n     -  Nested 2\n";
    let result = parse_and_serialize(input);
    assert_eq!(result, " -  Item:\n     -  Nested 1\n     -  Nested 2\n");
}

#[test]
fn test_loose_nested_list() {
    // Nested list after blank line (loose) - preserve blank line
    // Nested list indent = parent content start position (leading + 1 + trailing = 4)
    // plus its own leading space, so 5 spaces before marker
    let input = " -  Item.\n\n     -  Nested 1\n     -  Nested 2\n";
    let result = parse_and_serialize(input);
    assert_eq!(result, " -  Item.\n\n     -  Nested 1\n     -  Nested 2\n");
}

#[test]
fn test_serialize_asterisk_in_text_not_emphasis() {
    // Asterisks in plain text that aren't emphasis should be escaped
    let input = "5 * 3 = 15";
    let result = parse_and_serialize(input);
    // The asterisk in "5 * 3" should be escaped to prevent misinterpretation
    assert_eq!(result, "5 \\* 3 = 15\n");
}

#[test]
fn test_serialize_image_inside_link_badge_style() {
    // Badge-style: image inside a link, both using reference style
    // Input: [![alt][img-ref]][link-ref] with definitions
    // Should output fully inline: [![alt](img-url)](link-url)
    let input = r#"[![JSR][JSR badge]][JSR]

[JSR]: https://jsr.io/
[JSR badge]: https://jsr.io/badge.svg
"#;
    let result = parse_and_serialize(input);
    // The output should have a clickable image linking to JSR
    assert!(
        result.contains("[![JSR](https://jsr.io/badge.svg)](https://jsr.io/)"),
        "Should output fully inline badge-style link"
    );
    assert!(
        !result.contains("[![JSR](https://jsr.io/badge.svg)]:"),
        "Should not create malformed reference definition"
    );
}

#[test]
fn test_serialize_underscore_always_escaped() {
    // Underscores are always escaped for safety and consistency across parsers
    let input = "Use ALL_CAPS for constants.";
    let result = parse_and_serialize(input);
    assert_eq!(result, "Use ALL\\_CAPS for constants.\n");
}

#[test]
fn test_serialize_underscore_at_boundary_escaped() {
    // Underscores at word boundaries should be escaped
    let input = r"\_start and end\_";
    let result = parse_and_serialize(input);
    assert_eq!(result, "\\_start and end\\_\n");
}

#[test]
fn test_serialize_autolink_preserved() {
    // Autolinks <url> should be preserved as autolink format, not reference style
    let input = "Visit <https://example.com/> for more info.";
    let result = parse_and_serialize(input);
    assert_eq!(result, "Visit <https://example.com/> for more info.\n");
}

#[test]
fn test_serialize_nested_list_wrap_continuation() {
    // Nested list items should wrap with proper continuation indent
    // accounting for the parent list's indentation
    let input = " 1. First\n     -  This is a very long nested item that should wrap with proper eight-space continuation.";
    let result = parse_and_serialize_with_width(input, 80);
    // Continuation should have 8 spaces (4 for parent + 4 for list item content)
    assert!(
        result.contains("\n        "),
        "Nested list continuation should have 8-space indent, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_next_line() {
    // hongdown-disable-next-line should preserve the next block element as-is
    let input = "<!-- hongdown-disable-next-line -->\n[![Badge][badge-img]][badge-url]\n\n[badge-img]: https://example.com/badge.svg\n[badge-url]: https://example.com";
    let result = parse_and_serialize_with_source(input);
    // The badge line should be preserved exactly as-is (not converted to inline)
    assert!(
        result.contains("[![Badge][badge-img]][badge-url]"),
        "disable-next-line should preserve the next line as-is, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_file() {
    // hongdown-disable-file should preserve the entire file as-is
    let input = "<!-- hongdown-disable-file -->\n\nTitle\n===\n\nSome paragraph with *emphasis* that would normally be reformatted.";
    let result = parse_and_serialize_with_source(input);
    // The entire content after the directive should be preserved
    assert!(
        result.contains("Title\n==="),
        "disable-file should preserve file content as-is, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_file_after_front_matter() {
    // hongdown-disable-file after front matter should preserve everything after it
    let input = "---\ntitle: Test\n---\n\n<!-- hongdown-disable-file -->\n\n# Title\n\nSome   badly   formatted   text.";
    let result = parse_and_serialize_with_source(input);
    // The file content should be preserved exactly as-is
    assert_eq!(
        result, input,
        "disable-file after front matter should preserve file content exactly"
    );
}

#[test]
fn test_directive_disable_file_preserves_trailing_newline() {
    // hongdown-disable-file should preserve trailing newline
    let input = "<!-- hongdown-disable-file -->\n\n# Title\n\nSome text.\n";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(
        result, input,
        "disable-file should preserve trailing newline"
    );
}

#[test]
fn test_directive_disable_next_section() {
    // hongdown-disable-next-section should preserve content until the next heading
    let input = "First section\n-------------\n\nNormal content.\n\n<!-- hongdown-disable-next-section -->\n\nSecond section\n--------------\n\n[![Badge][img]][url]\n\n[img]: https://example.com/img.svg\n[url]: https://example.com\n\nThird section\n-------------\n\nThis should be formatted normally.";
    let result = parse_and_serialize_with_source(input);
    // Second section should be preserved as-is
    assert!(
        result.contains("[![Badge][img]][url]"),
        "disable-next-section should preserve section content as-is, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_enable() {
    // hongdown-disable and hongdown-enable should bracket unformatted regions
    let input = "Normal paragraph.\n\n<!-- hongdown-disable -->\n\n[![Badge][img]][url]\n\nAnother unformatted line.\n\n<!-- hongdown-enable -->\n\nBack to normal formatting.\n\n[img]: https://example.com/img.svg\n[url]: https://example.com";
    let result = parse_and_serialize_with_source(input);
    // Content between disable/enable should be preserved
    assert!(
        result.contains("[![Badge][img]][url]"),
        "disable/enable should preserve bracketed content as-is, got:\n{}",
        result
    );
    assert!(
        result.contains("Another unformatted line."),
        "disable/enable should preserve all bracketed content, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_preserves_reference_definition() {
    // A reference definition inside a disabled region has no AST node of its
    // own, but it must still survive: dropping it leaves the region's links
    // pointing at nothing.
    let input = "Intro.\n\n<!-- hongdown-disable -->\n\nPreserved [guide] here.\n\n[guide]: https://example.com/preserved\n";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(
        result, input,
        "a disabled region must keep its own reference definition"
    );
}

#[test]
fn test_directive_disable_preserves_region_verbatim() {
    // Everything between the directives is copied from the source, so
    // indentation and blank lines inside the region are kept as they were.
    let input = "Intro.\n\n<!-- hongdown-disable -->\n\n  Indented   [a] line.\n\n\n[a]: https://example.com/a\n[b]: https://example.com/b \"Title\"\n\nUses [b] too.\n\n<!-- hongdown-enable -->\n\nBack.\n";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(
        result, input,
        "a disabled region must be preserved verbatim"
    );
}

#[test]
fn test_directive_disable_keeps_definitions_defined_outside() {
    // The definitions live after the region, so they cannot be copied with it;
    // the region's links must still keep them from being dropped.
    let input = "Normal paragraph.\n\n<!-- hongdown-disable -->\n\n[![Badge][img]][url]\n\n<!-- hongdown-enable -->\n\nBack to normal formatting.\n\n[img]: https://example.com/img.svg\n[url]: https://example.com\n";
    let result = parse_and_serialize_with_source(input);
    // Definitions keep the order the formatted path would give them: the
    // badge's image is registered before the link wrapping it.
    assert_eq!(
        result, input,
        "definitions used only inside a disabled region must be kept"
    );
}

#[test]
fn test_directive_disable_without_source_still_skips_formatting() {
    // Without the original source there is nothing to copy, so the region
    // falls back to emitting its blocks one by one.
    let input = "Normal.\n\n<!-- hongdown-disable -->\n\nUnformatted.\n\n<!-- hongdown-enable -->\n\nBack.\n";
    let result = parse_and_serialize(input);
    assert!(
        result.contains("Unformatted."),
        "the region's content must survive without a source, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_copied_definition_holds_its_label() {
    // No link resolves through the copied definition, so nothing reveals its
    // target, but it still defines the label document-wide.  Letting a later
    // link take that label would put a second definition after it, and since
    // CommonMark keeps the first, the later link would point at the copy.
    let input = "<!-- hongdown-disable -->\n\nRaw text.\n\n[guide]: https://example.com/copied\n\n<!-- hongdown-enable -->\n\nLater [guide](https://example.com/other) link.\n";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(
        result,
        "<!-- hongdown-disable -->\n\nRaw text.\n\n[guide]: https://example.com/copied\n\n<!-- hongdown-enable -->\n\nLater [guide][guide 2] link.\n\n[guide 2]: https://example.com/other\n",
        "the later link must keep its own destination"
    );
    assert_eq!(
        parse_and_serialize_with_source(&result),
        result,
        "formatting must be idempotent"
    );
}

#[test]
fn test_directive_disable_copied_duplicate_holds_its_label() {
    // The copy repeats a label defined above it, and nothing resolves through
    // the definition above, so that one is dropped as unused.  The copy is
    // still a definition, and a later link taking its label would resolve
    // through the copy rather than through its own destination.
    let input = "[g]: https://example.com/first\n\n<!-- hongdown-disable -->\n\nRaw text.\n\n[g]: https://example.com/second\n\n<!-- hongdown-enable -->\n\nLater [g](https://example.com/other) link.\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("Later [g][g 2] link.")
            && result.contains("[g 2]: https://example.com/other"),
        "the later link must keep its own destination, got:\n{}",
        result
    );
    assert_eq!(
        parse_and_serialize_with_source(&result),
        result,
        "formatting must be idempotent"
    );
}

#[test]
fn test_directive_disable_link_keeps_its_label_against_a_formatted_one() {
    // Both links want the label `guide` for different destinations.  The one
    // inside the region is copied verbatim and cannot be relabelled, so the
    // formatted link is the one that takes a derived label.
    let input = "[guide](https://example.com/first)\n\n<!-- hongdown-disable -->\n\nSee [guide] here.\n\n<!-- hongdown-enable -->\n\n[guide]: https://example.com/second\n";
    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(
        result.output,
        "[guide][guide 2]\n\n[guide 2]: https://example.com/first\n\n<!-- hongdown-disable -->\n\nSee [guide] here.\n\n<!-- hongdown-enable -->\n\n[guide]: https://example.com/second\n",
        "the verbatim link must keep its label and both destinations must survive"
    );
    assert!(
        result.warnings.is_empty(),
        "the collision is resolvable, so nothing should be reported: {:?}",
        result.warnings
    );
    assert_eq!(
        parse_and_serialize_with_source(&result.output),
        result.output,
        "formatting must be idempotent"
    );
}

#[test]
fn test_directive_disable_file_keeps_definitions_defined_before() {
    // The tail keeps its source text, so a definition placed before the
    // directive is only kept if the tail's links reserve it.
    let input = "Intro.\n\n<!-- hongdown-disable-file -->\n\nRaw   [guide] text.\n";
    let source = format!("[guide]: https://example.com/guide\n\n{}", input);
    let result = parse_and_serialize_with_source(&source);
    assert!(
        result.contains("[guide]: https://example.com/guide"),
        "a definition the disabled tail depends on must be kept, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_keeps_a_duplicated_definition_inert() {
    // The region's copy repeats a label that is already defined above it.
    // CommonMark resolves both links through the first definition, so that one
    // has to stay first in the output or the links would be retargeted.
    let input = "Intro [g].\n\n[g]: https://example.com/first\n\n<!-- hongdown-disable -->\n\nRaw [g].\n\n[g]: https://example.com/second\n";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(
        result, input,
        "the winning definition must stay ahead of the copied duplicate"
    );
}

#[test]
fn test_directive_disable_keeps_a_multiline_definition_ahead_of_its_duplicate() {
    // The winning definition spells its destination on the following line, as
    // CommonMark allows.  Overlooking it would make the region's duplicate look
    // like the winner and let the copy retarget the link.
    let input = "Intro.\n\n[g]:\n    https://example.com/first\n\n<!-- hongdown-disable -->\n\nRaw [g].\n\n[g]: https://example.com/second\n";
    let result = parse_and_serialize_with_source(input);
    let winner = result
        .find("[g]: https://example.com/first")
        .expect("the winning definition must be kept");
    let duplicate = result
        .find("[g]: https://example.com/second")
        .expect("the region's copy must be preserved");
    assert!(
        winner < duplicate,
        "the winning definition must come first, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_definition_inside_region_is_not_duplicated() {
    // The definition is copied with the region and also serves a link outside
    // it; the outside link must reuse that copy rather than emit its own.
    let input =
        "Intro [g].\n\n<!-- hongdown-disable -->\n\nRaw [g].\n\n[g]: https://example.com/url\n";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(
        result.matches("[g]: ").count(),
        1,
        "the definition must appear exactly once, got:\n{}",
        result
    );
    assert_eq!(result, input, "the region must be preserved verbatim");
}

#[test]
fn test_directive_disable_ignores_definitions_inside_code_blocks() {
    // A definition-looking line inside a fenced code block in the region is
    // not a definition, so the real one outside must still be kept.
    let input = "Intro.\n\n<!-- hongdown-disable -->\n\nSee [guide] here.\n\n~~~~ markdown\n[guide]: https://example.com/not-a-definition\n~~~~\n\n<!-- hongdown-enable -->\n\n[guide]: https://example.com/real\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("[guide]: https://example.com/real"),
        "a definition must not be considered preserved by a code block, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_ignores_definitions_inside_html_blocks() {
    // Raw HTML is not Markdown either, so a definition-looking line in it does
    // not stand in for the real definition outside the region.
    let input = "Intro.\n\n<!-- hongdown-disable -->\n\nSee [guide] here.\n\n<pre>\n[guide]: https://example.com/not-a-definition\n</pre>\n\n<!-- hongdown-enable -->\n\n[guide]: https://example.com/real\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("[guide]: https://example.com/real"),
        "a definition must not be considered preserved by an HTML block, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_ignores_a_bare_label_that_defines_nothing() {
    // `[foo]:` here is a continuation line of a paragraph, not a definition, so
    // the real definition must still be kept.
    let input = "Intro.\n\n<!-- hongdown-disable -->\n\nSee [foo] here.\n\nA line\n[foo]:\nnot a destination at all\n\n<!-- hongdown-enable -->\n\n[foo]: https://example.com/real\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("[foo]: https://example.com/real"),
        "a label that defines nothing must not stand in for the definition, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_ignores_a_definition_lookalike_in_a_paragraph() {
    // The line inside the region continues a paragraph, so it defines nothing.
    // Taking it for the winning definition would suppress the real one and
    // leave both links pointing at nothing.
    let input = "Formatted [g] link.\n\n<!-- hongdown-disable -->\n\nThis is prose\n[g]: https://example.com/not-a-definition\n\n<!-- hongdown-enable -->\n\nEnd.\n\n[g]: https://example.com/real\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("[g]: https://example.com/real"),
        "the real definition must survive, got:\n{}",
        result
    );
    assert_eq!(
        parse_and_serialize_with_source(&result),
        result,
        "formatting must be idempotent"
    );
}

#[test]
fn test_directive_disable_sees_a_blockquoted_definition_in_the_region() {
    // A definition under a blockquote prefix is still a definition, and it is
    // copied with the region, so it must not also be emitted outside it.
    let input = "Intro.\n\n<!-- hongdown-disable -->\n\n> Quoted [g] link.\n>\n> [g]: https://example.com/quoted\n\n<!-- hongdown-enable -->\n\nEnd.\n";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(
        result.matches("[g]: ").count(),
        1,
        "the definition must appear exactly once, got:\n{}",
        result
    );
    assert_eq!(result, input, "the region must be preserved verbatim");
}

#[test]
fn test_directive_disable_ignores_a_definition_lookalike_in_a_code_span() {
    // A code span holding a newline spans lines without a break node, so the
    // line count alone would expose the line above it.  It is paragraph content
    // all the same, and the real definition must survive.
    let input = "Formatted [g] link.\n\n<!-- hongdown-disable -->\n\nSee [g] and `code\n[g]: not-a-definition\nspan`\n\n<!-- hongdown-enable -->\n\nEnd.\n\n[g]: https://example.com/real\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("[g]: https://example.com/real"),
        "the real definition must survive, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_sees_a_definition_at_a_paragraph_head() {
    // The parser consumes a definition written at the head of a paragraph but
    // leaves its line inside the paragraph's span.  Overlooking it would make
    // the region's duplicate look like the winner and retarget the links.
    let input = "Intro.\n\n[g]: https://example.com/first\nSome text right after.\n\n<!-- hongdown-disable -->\n\nRaw [g].\n\n[g]: https://example.com/second\n";
    let result = parse_and_serialize_with_source(input);
    let winner = result
        .find("[g]: https://example.com/first")
        .expect("the winning definition must be kept");
    let duplicate = result
        .find("[g]: https://example.com/second")
        .expect("the region's copy must be preserved");
    assert!(
        winner < duplicate,
        "the winning definition must come first, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_sees_definitions_after_a_multiline_one() {
    // The first definition takes its destination from the line below it, and
    // the second sits beneath that.  Overlooking the second would make the
    // region's duplicate of it the apparent winner and retarget the links.
    let input = "Intro.\n\n[a]:\n    https://example.com/a\n[b]: https://example.com/b\nSome text right after.\n\n<!-- hongdown-disable -->\n\nRaw [a] and [b].\n\n[b]: https://example.com/wrong\n";
    let result = parse_and_serialize_with_source(input);
    let winner = result
        .find("[b]: https://example.com/b")
        .expect("the winning definition must be kept");
    let duplicate = result
        .find("[b]: https://example.com/wrong")
        .expect("the region's copy must be preserved");
    assert!(
        winner < duplicate,
        "the winning definition must come first, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_shadowed_definition_beats_a_footnote_schedule() {
    // The winning definition is only spoken for by a footnote referenced after
    // the region, and that collection is flushed on the footnote's schedule.
    // It still has to come out ahead of the copy that redefines the label.
    let input = "[a]: https://example.com/first\n\nIntro [a].\n\n<!-- hongdown-disable -->\n\nRaw [a].\n\n[a]: https://example.com/second\n\n<!-- hongdown-enable -->\n\nText[^1].\n\n[^1]: See [a].\n";
    let result = parse_and_serialize_with_source(input);
    let winner = result
        .find("[a]: https://example.com/first")
        .expect("the winning definition must be kept");
    let duplicate = result
        .find("[a]: https://example.com/second")
        .expect("the region's copy must be preserved");
    assert!(
        winner < duplicate,
        "the winning definition must come first, got:\n{}",
        result
    );
    assert_eq!(
        result.matches("[a]: https://example.com/first").count(),
        1,
        "and only once, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_sees_a_definition_a_setext_heading_swallowed() {
    // A setext heading keeps the lines of a definition consumed at its head,
    // just as a paragraph does.  Overlooking it would make the region's copy
    // the apparent winner and retarget the links.
    let input = "[a]: https://example.com/first\nHeading\n=======\n\nIntro [a].\n\n<!-- hongdown-disable -->\n\nRaw [a].\n\n[a]: https://example.com/second\n";
    let result = parse_and_serialize_with_source(input);
    let winner = result
        .find("[a]: https://example.com/first")
        .expect("the winning definition must be kept");
    let duplicate = result
        .find("[a]: https://example.com/second")
        .expect("the region's copy must be preserved");
    assert!(
        winner < duplicate,
        "the winning definition must come first, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_next_line_survives_a_noun_directive() {
    // A directive that only names nouns leaves the skip in place, so the block
    // after it is still the one copied, along with the definition at its head.
    let input = "<!-- hongdown-disable-next-line -->\n<!-- hongdown-proper-nouns: Foo -->\n[a]: https://example.com/copied\nSome   prose.\n\nLater [a](https://example.com/other).\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("Later [a][a 2].") && result.contains("[a 2]: https://example.com/other"),
        "the copied definition must hold its label, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_next_line_survives_a_footnote_definition() {
    // A footnote definition is emitted by the footnote machinery rather than
    // copied, so it does not consume the skip either.
    let input = "Text[^1].\n\n<!-- hongdown-disable-next-line -->\n\n[^1]: Note.\n\n[a]: https://example.com/copied\nSome   prose.\n\nLater [a](https://example.com/other).\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("Later [a][a 2].") && result.contains("[a 2]: https://example.com/other"),
        "the copied definition must hold its label, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_next_section_ends_at_an_enable() {
    // `hongdown-enable` ends a section-wide skip, so what follows is formatted
    // and its definitions are not carried by any copy.
    let input = "<!-- hongdown-disable-next-section -->\n\nRaw   text.\n\n<!-- hongdown-enable -->\n\n[a]: https://example.com/plain\nSome prose.\n\nLater [a](https://example.com/other).\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("Raw   text."),
        "the skipped block must keep its spacing, got:\n{}",
        result
    );
    assert!(
        result.contains("Later [a].") && result.contains("[a]: https://example.com/other"),
        "the label is free after the enable, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_next_line_copy_carries_its_head_definition() {
    // The block is copied by its source span, which reaches back over the
    // definition consumed at its head, so the copy defines that label too and
    // a later link must not take it.
    let input = "<!-- hongdown-disable-next-line -->\n[a]: https://example.com/copied\nSome   prose.\n\nLater [a](https://example.com/other).\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("Later [a][a 2].") && result.contains("[a 2]: https://example.com/other"),
        "the later link must keep its own destination, got:\n{}",
        result
    );
    assert_eq!(
        parse_and_serialize_with_source(&result),
        result,
        "formatting must be idempotent"
    );
}

#[test]
fn test_mdx_copied_block_carries_its_head_definition() {
    let input = "<!-- hongdown-disable -->\n\n\
                 [foo {bar}]: https://example.com/copied\n\
                 Prose directly below keeps the definition consumed.\n\n\
                 <!-- hongdown-enable -->\n\n\
                 Read [foo {bar}](https://example.com/other).\n";
    let options = Options {
        mdx: true,
        ..Options::default()
    };
    let result = crate::format(input, &options).unwrap();
    assert!(
        result.contains("Read [foo {bar}][foo {bar} 2].")
            && result.contains("[foo {bar} 2]: https://example.com/other"),
        "the formatted link must keep its own destination, got:\n{result}"
    );
    assert_eq!(
        crate::format(&result, &options).unwrap(),
        result,
        "formatting must be idempotent"
    );
}

#[test]
fn test_directive_disable_reservation_precedes_a_setext_heading() {
    // The definition sits on the line a setext heading reports as its own
    // start, but it precedes the heading, so it belongs above the section.
    let input = "<!-- hongdown-disable -->\n\nRaw [a].\n\n<!-- hongdown-enable -->\n\n[a]: https://example.com/first\nSection\n-------\n\nBody.\n";
    let result = parse_and_serialize_with_source(input);
    let definition = result
        .find("[a]: https://example.com/first")
        .expect("the definition must be kept");
    let section = result
        .find("Section\n-------")
        .expect("the section must be emitted");
    assert!(
        definition < section,
        "the definition must come before the section it precedes, got:\n{}",
        result
    );
    assert_eq!(
        parse_and_serialize_with_source(&result),
        result,
        "formatting must be idempotent"
    );
}

#[test]
fn test_directive_disable_reservation_waits_for_its_section() {
    // The definition the copied link needs sits after a section boundary, so
    // reserving it must not pull it up to that boundary.  It is a link only on
    // the second run, when the first has already written the definition, so
    // pulling it up would mean one run never settles the file.
    let input = "<!-- hongdown-disable -->\n\nRaw [a].\n\n<!-- hongdown-enable -->\n\nSection\n-------\n\nLater [a](https://example.com/a).\n";
    let result = parse_and_serialize_with_source(input);
    let definition = result
        .find("[a]: https://example.com/a")
        .expect("the definition must be kept");
    let section = result.find("Section").expect("the section must be emitted");
    assert!(
        definition > section,
        "the definition must stay in the section the source put it in, got:\n{}",
        result
    );
    assert_eq!(
        parse_and_serialize_with_source(&result),
        result,
        "one pass must settle the file"
    );
}

#[test]
fn test_directive_disable_reservation_keeps_its_section() {
    // Nothing outside the region uses these labels, so the reservation decides
    // where they go.  It follows the source, which puts them at the end of the
    // region's own section rather than at the end of the document.
    let input = "Section one\n-----------\n\n<!-- hongdown-disable -->\n\n[![B][img]][url]\n\n<!-- hongdown-enable -->\n\n[img]: https://example.com/i.svg\n[url]: https://example.com/u\n\n\nSection two\n-----------\n\nText.\n";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(
        result, input,
        "the definitions must stay in their own section"
    );
}

#[test]
fn test_directive_disable_reservation_keeps_the_definition_order() {
    // `[a]` in the region resolves only once the formatter has defined it, so
    // it is a link on the second run and not on the first.  Reserving it must
    // not therefore move its definition ahead of `[b]`, or one run would not
    // settle the file.
    let input = "<!-- hongdown-disable -->\n\nRaw [a].\n\n<!-- hongdown-enable -->\n\nLater [b](https://example.com/b) and [a](https://example.com/a).\n";
    let result = parse_and_serialize_with_source(input);
    let first = result
        .find("[b]: https://example.com/b")
        .expect("the first definition must be kept");
    let second = result
        .find("[a]: https://example.com/a")
        .expect("the second definition must be kept");
    assert!(
        first < second,
        "the definitions must follow the order the document's own links set, got:\n{}",
        result
    );
    assert_eq!(
        parse_and_serialize_with_source(&result),
        result,
        "one pass must settle the file"
    );
}

#[test]
fn test_directive_disable_rejects_a_malformed_definition_lookalike() {
    // Neither line is a definition — the parser leaves both inside a paragraph
    // — so the real definition must survive.
    let input = "Intro.\n\n<!-- hongdown-disable -->\n\nSee [g].\n\nprose\n[g]: foo(bar\n[g]: <unterminated\n\n<!-- hongdown-enable -->\n\n[g]: https://example.com/real\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("[g]: https://example.com/real"),
        "the real definition must survive, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_accepts_a_destination_with_balanced_parentheses() {
    // Parentheses are ordinary in URLs, so the definition is real and is copied
    // with the region rather than emitted a second time outside it.
    let input = "Intro.\n\n<!-- hongdown-disable -->\n\nSee [w].\n\n[w]: https://en.wikipedia.org/wiki/Foo_(bar)\n\n<!-- hongdown-enable -->\n\nEnd.\n";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(
        result.matches("[w]: ").count(),
        1,
        "the definition must appear exactly once, got:\n{}",
        result
    );
    assert_eq!(result, input, "the region must be preserved verbatim");
}

#[test]
fn test_directive_disable_separates_a_definition_from_the_directive() {
    // The definition is flushed above the region because the copy redefines the
    // label, and it has no node of its own for the child index to count.
    let input = "[g]: https://example.com/first\n\n<!-- hongdown-disable -->\n\nRaw [g].\n\n[g]: https://example.com/second\n";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(
        result, input,
        "the directive must not run into the definition above it"
    );
}

#[test]
fn test_directive_after_front_matter_gets_one_blank_line() {
    // Front matter already ends with a blank line, so the directive must not
    // add another one on top of it.
    let input = "---\ntitle: Test\n---\n\n<!-- hongdown-disable-next-line -->\n\nSome   text.\n";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(
        result, input,
        "a directive after front matter must be separated by one blank line"
    );
}

#[test]
fn test_directive_disable_file_definition_does_not_add_leading_blank_lines() {
    // The definition is the only thing before the directive, and it has no node
    // of its own, so the document would start with the flush that emits it.
    let input = "[g]: https://example.com/g\n\n<!-- hongdown-disable-file -->\n\nRaw   [g] text.\n";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(
        result, input,
        "a document must not gain leading blank lines"
    );
}

#[test]
fn test_directive_disable_next_line_keeps_definitions() {
    // Same loss through a different directive: the badge is emitted verbatim,
    // so nothing registers the definitions it depends on.
    let input = "<!-- hongdown-disable-next-line -->\n[![Badge][badge-img]][badge-url]\n\n[badge-img]: https://example.com/badge.svg\n[badge-url]: https://example.com\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("[badge-img]: https://example.com/badge.svg"),
        "disable-next-line must keep the definitions its block uses, got:\n{}",
        result
    );
    assert!(
        result.contains("[badge-url]: https://example.com"),
        "disable-next-line must keep the definitions its block uses, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_next_section_keeps_definitions() {
    // Without a following heading the whole rest of the document is skipped,
    // so its links are the only users of the definitions.
    let input = "Intro.\n\n<!-- hongdown-disable-next-section -->\n\nPreserved [guide] here.\n\n[guide]: https://example.com/preserved\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("[guide]: https://example.com/preserved"),
        "disable-next-section must keep the definitions its content uses, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_preserves_footnote_definition_in_place() {
    // A footnote definition inside the region is copied with the region, so it
    // must not also be re-emitted by the footnote flush.
    let input = "Intro.\n\n<!-- hongdown-disable -->\n\nPara[^1].\n\n[^1]: The   note.\n\n<!-- hongdown-enable -->\n\nAfter.\n";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(
        result.matches("[^1]:").count(),
        1,
        "the footnote definition must not be duplicated, got:\n{}",
        result
    );
    assert_eq!(
        result, input,
        "a footnote definition inside a disabled region stays where it was"
    );
}

#[test]
fn test_directive_disable_file_inside_a_region_reaches_past_it() {
    // A directive disabling the file does so wherever it is written, so the
    // region it sits in is not where its effect ends.  What follows the enable
    // keeps its own marker and spacing.
    let input = "<!-- hongdown-disable -->\n\n<!-- hongdown-disable-file -->\n\nRaw   text.\n\n<!-- hongdown-enable -->\n\n*   badly    spaced list item\n";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(result, input, "the rest of the file must be left as it is");
}

#[test]
fn test_directive_disable_keeps_trailing_comment_once() {
    // A trailing HTML comment inside the region is part of the verbatim copy
    // and must not be emitted a second time after the references.
    let input = "Intro.\n\n<!-- hongdown-disable -->\n\nPara.\n\n<!-- a comment -->\n";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(
        result.matches("<!-- a comment -->").count(),
        1,
        "the trailing comment must not be duplicated, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_definition_used_on_both_sides() {
    // The definition sits outside the region and is used from both sides; it
    // must stay where the formatter normally puts it, exactly once.
    let input = "Intro.\n\n<!-- hongdown-disable -->\n\nPreserved [guide] here.\n\n<!-- hongdown-enable -->\n\nAfter [guide] too.\n\n[guide]: https://example.com/preserved\n";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(
        result, input,
        "the shared definition must be kept exactly once"
    );
}

#[test]
fn test_reference_label_keeps_an_escaped_bracket() {
    // A label may hold a bracket the backslash escapes.  Cutting the label
    // there would name a different definition, and the output would no longer
    // parse as one.
    let input = "See [x][a\\]b] here.\n\n[a\\]b]: https://example.com/x\n";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(result, input, "the label must be read whole");
}

#[test]
fn test_directive_disable_reads_a_link_below_a_consumed_definition() {
    // The copied paragraph opens with a definition, so the parser reports the
    // link inside it against the paragraph's own start, a line above where it
    // really sits.  Reading the link there would find no label, the winning
    // definition would go unreserved, and the copy's own would take over.
    let input = "[b]: https://example.com/first\n\n<!-- hongdown-disable -->\n\n[b]: https://example.com/second\nRaw [b].\n\n<!-- hongdown-enable -->\n\nEnd.\n";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(
        result, input,
        "the winning definition must survive, ahead of the copy"
    );
}

#[test]
fn test_directive_disable_next_line_reads_a_link_below_a_consumed_definition() {
    // Same block, copied by a different directive: the winning definition still
    // has to come out above the copy that repeats its label.
    let input = "[b]: https://example.com/first\n\n<!-- hongdown-disable-next-line -->\n[b]: https://example.com/second\nRaw [b].\n";
    let result = parse_and_serialize_with_source(input);
    let winner = result
        .find("[b]: https://example.com/first")
        .expect("the winning definition must be kept");
    let duplicate = result
        .find("[b]: https://example.com/second")
        .expect("the region's copy must be preserved");
    assert!(
        winner < duplicate,
        "the winning definition must come first, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_sees_a_definition_below_a_multiline_title() {
    // The first definition's title runs onto a second line, and the next
    // definition sits beneath it.  Missing that one would make the region's
    // copy of its label look like the winner.
    let input = "[a]: https://example.com/a \"A title\nspanning lines\"\n[b]: https://example.com/first\nSome prose.\n\n<!-- hongdown-disable -->\n\nRaw [b].\n\n[b]: https://example.com/second\n";
    let result = parse_and_serialize_with_source(input);
    let winner = result
        .find("[b]: https://example.com/first")
        .expect("the winning definition must be kept");
    let duplicate = result
        .find("[b]: https://example.com/second")
        .expect("the region's copy must be preserved");
    assert!(
        winner < duplicate,
        "the winning definition must come first, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_sees_a_definition_below_a_three_line_title() {
    // The title runs over two continuation lines, and only the delimiter that
    // opened it closes it.  Stopping at the first continuation would hide the
    // definition beneath from the scanner.
    let input = "[a]: https://example.com/a \"A title\nspanning three\nlines\"\n[b]: https://example.com/first\nSome prose.\n\n<!-- hongdown-disable -->\n\nRaw [b].\n\n[b]: https://example.com/second\n";
    let result = parse_and_serialize_with_source(input);
    let winner = result
        .find("[b]: https://example.com/first")
        .expect("the winning definition must be kept");
    let duplicate = result
        .find("[b]: https://example.com/second")
        .expect("the region's copy must be preserved");
    assert!(
        winner < duplicate,
        "the winning definition must come first, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_next_line_copies_whole_lines() {
    // A block's span begins at its content, past the indentation and the
    // markers of whatever holds it.  Copying from there would lose them, and
    // with them a definition the parser took out of the list item, which the
    // span leaves behind — while the lines it sits on count as copied.
    let input = "<!-- hongdown-disable-next-line -->\n\n  - [a]: https://example.com\n\nSee [x][a 2].\n\n[a 2]: https://example.com\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("  - [a]: https://example.com"),
        "the copy must keep the whole line, got:\n{}",
        result
    );
    assert_eq!(result, input, "and formatting must be a fixed point");
}

#[test]
fn test_directive_disable_next_section_copies_indented_code() {
    // The indentation is what makes this a code block, and the span reaching
    // over the blank line below it must not add one to the separator that
    // follows.
    let input = "<!-- hongdown-disable-next-section -->\n\n    indented code\n\nAfter.\n";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(result, input, "the copy must keep the block as it stands");
}

#[test]
fn test_directive_disable_next_section_copies_whole_lines() {
    // Same, for the section-wide skip, where the definition the list item held
    // is what the copied link resolves through.
    let input =
        "<!-- hongdown-disable-next-section -->\n\n[x][a]\n\n  - [a]: https://example.com\n";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(
        result, input,
        "the copy must keep the definition the link needs"
    );
}

#[test]
fn test_directive_disable_sees_a_definition_under_a_list_marker() {
    // A definition keeps defining a document-wide label wherever it is written,
    // and a list marker does not make one a task item — the colon rules that
    // out.  The copy therefore holds the label against a later link.
    let input = "<!-- hongdown-disable -->\n\n- [foo]: https://example.com/first\n\n<!-- hongdown-enable -->\n\nLater [foo](https://example.com/second).\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("Later [foo][foo 2].")
            && result.contains("[foo 2]: https://example.com/second"),
        "the later link must keep its own destination, got:\n{}",
        result
    );
    assert_eq!(
        parse_and_serialize_with_source(&result),
        result,
        "formatting must be idempotent"
    );
}

#[test]
fn test_directive_disable_sees_a_definition_with_a_broken_label() {
    // The label breaks across two lines, which names `foo bar` all the same, so
    // the copy holds that label too.
    let input = "<!-- hongdown-disable -->\n\n[foo\nbar]: https://example.com/first\n\n<!-- hongdown-enable -->\n\nLater [foo bar](https://example.com/second).\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("Later [foo bar][foo bar 2].")
            && result.contains("[foo bar 2]: https://example.com/second"),
        "the later link must keep its own destination, got:\n{}",
        result
    );
    assert_eq!(
        parse_and_serialize_with_source(&result),
        result,
        "formatting must be idempotent"
    );
}

#[test]
fn test_directive_disable_sees_a_broken_label_inside_a_blockquote() {
    // The label breaks across two lines of a blockquote, so both carry its
    // marker.  A marker is no more part of the label on the second line than on
    // the first, and the label is `foo bar` either way.
    let input = "<!-- hongdown-disable -->\n\n> [foo\n> bar]: https://example.com/first\n\n<!-- hongdown-enable -->\n\nLater [foo bar](https://example.com/second).\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("Later [foo bar][foo bar 2].")
            && result.contains("[foo bar 2]: https://example.com/second"),
        "the later link must keep its own destination, got:\n{}",
        result
    );
    assert_eq!(
        parse_and_serialize_with_source(&result),
        result,
        "formatting must be idempotent"
    );
}

#[test]
fn test_directive_disable_sees_a_broken_label_inside_a_list_item() {
    // A list marks only the line it opens and indents the rest, so the label's
    // second line carries no marker and reads as part of `foo bar`.
    let input = "<!-- hongdown-disable -->\n\n- [foo\n  bar]: https://example.com/first\n\n<!-- hongdown-enable -->\n\nLater [foo bar](https://example.com/second).\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("Later [foo bar][foo bar 2].")
            && result.contains("[foo bar 2]: https://example.com/second"),
        "the later link must keep its own destination, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_reads_an_ordered_marker_below_a_label_as_label() {
    // An ordered marker only opens a block in the middle of one where it
    // numbers from one, so `2.` goes on reading as part of the label above it.
    let input = "<!-- hongdown-disable -->\n\n[foo\n2. bar]: https://example.com/first\n\n<!-- hongdown-enable -->\n\nLater [foo 2. bar](https://example.com/second).\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("Later [foo 2. bar][foo 2. bar 2].")
            && result.contains("[foo 2. bar 2]: https://example.com/second"),
        "the copied label must be held, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_list_marker_below_a_label_ends_the_definition() {
    // A list marker opens a block wherever it stands, so it interrupts rather
    // than continuing the label above it.  The parser defines nothing here, and
    // neither does the copy, leaving the label free for the later link.
    let input = "<!-- hongdown-disable -->\n\n[foo\n- bar]: https://example.com/phantom\n\n<!-- hongdown-enable -->\n\nLater [foo bar](https://example.com/second).\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("Later [foo bar].")
            && result.contains("[foo bar]: https://example.com/second"),
        "the label is free, so the later link keeps it, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_sees_a_definition_below_a_title_of_its_own_line() {
    // The title begins on the line below a complete destination, as CommonMark
    // allows.  The definition beneath it still has to be found.
    let input = "[a]: https://example.com/a\n  \"A title\"\n[b]: https://example.com/first\nSome prose.\n\n<!-- hongdown-disable -->\n\nRaw [b].\n\n[b]: https://example.com/second\n";
    let result = parse_and_serialize_with_source(input);
    let winner = result
        .find("[b]: https://example.com/first")
        .expect("the winning definition must be kept");
    let duplicate = result
        .find("[b]: https://example.com/second")
        .expect("the region's copy must be preserved");
    assert!(
        winner < duplicate,
        "the winning definition must come first, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_sees_a_definition_below_a_title_running_on() {
    // Same, with the title beginning below the destination and running on to a
    // further line before it closes.
    let input = "[a]: https://example.com/a\n  \"A title\nspanning lines\"\n[b]: https://example.com/first\nSome prose.\n\n<!-- hongdown-disable -->\n\nRaw [b].\n\n[b]: https://example.com/second\n";
    let result = parse_and_serialize_with_source(input);
    let winner = result
        .find("[b]: https://example.com/first")
        .expect("the winning definition must be kept");
    let duplicate = result
        .find("[b]: https://example.com/second")
        .expect("the region's copy must be preserved");
    assert!(
        winner < duplicate,
        "the winning definition must come first, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_title_is_not_closed_by_an_escaped_quote() {
    // The quotes inside the title are escaped, so they close nothing and the
    // title still runs to the line below.
    let input = "[a]: https://example.com/a \"A \\\"quoted\\\" title\nspanning lines\"\n[b]: https://example.com/first\nSome prose.\n\n<!-- hongdown-disable -->\n\nRaw [b].\n\n[b]: https://example.com/second\n";
    let result = parse_and_serialize_with_source(input);
    let winner = result
        .find("[b]: https://example.com/first")
        .expect("the winning definition must be kept");
    let duplicate = result
        .find("[b]: https://example.com/second")
        .expect("the region's copy must be preserved");
    assert!(
        winner < duplicate,
        "the winning definition must come first, got:\n{}",
        result
    );
}

#[test]
fn test_directive_disable_copy_carries_an_escaped_bracket_label() {
    // The region carries both the link and its definition, so nothing is left
    // for the reservation to emit — least of all a definition under a label cut
    // short at the escaped bracket.
    let input = "<!-- hongdown-disable -->\n\nSee [x][a\\]b] here.\n\n[a\\]b]: https://example.com/x\n\n<!-- hongdown-enable -->\n\nEnd.\n";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(
        result, input,
        "the region must be preserved with nothing added after it"
    );
}

#[test]
fn test_preserve_reference_style_badge() {
    // Reference-style badge links should be preserved as reference style
    let input = "[![JSR][JSR badge]][JSR]\n\n[JSR]: https://jsr.io/@optique\n[JSR badge]: https://jsr.io/badges/@optique/core";
    let result = parse_and_serialize_with_source(input);
    // Should preserve reference style, not convert to inline
    assert!(
        result.contains("[![JSR][JSR badge]][JSR]"),
        "Reference-style badge should be preserved, got:\n{}",
        result
    );
}

#[test]
fn test_preserve_reference_style_image() {
    // Reference-style images should be preserved as reference style
    let input = "![Logo][logo]\n\n[logo]: https://example.com/logo.png";
    let result = parse_and_serialize_with_source(input);
    // Should preserve reference style
    assert!(
        result.contains("![Logo][logo]"),
        "Reference-style image should be preserved, got:\n{}",
        result
    );
}

#[test]
fn test_preserve_reference_style_link() {
    // Reference-style links should be preserved as reference style
    let input =
        "Check the [documentation][docs] for more info.\n\n[docs]: https://example.com/docs";
    let result = parse_and_serialize_with_source(input);
    // Should preserve reference style
    assert!(
        result.contains("[documentation][docs]"),
        "Reference-style link should be preserved, got:\n{}",
        result
    );
}

#[test]
fn test_code_span_with_backticks() {
    // Code spans containing backticks should use double backticks as delimiters
    let input = "Here is `` `code` `` in text.";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("`` `code` ``"),
        "Code span with backtick should use double backtick delimiters, got:\n{}",
        result
    );
}

#[test]
fn test_code_span_with_multiple_backticks() {
    // Code spans containing double backticks should use triple backticks
    let input = "Use ``` `` ``` for double backticks.";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("``` `` ```"),
        "Code span with double backticks should use triple backtick delimiters, got:\n{}",
        result
    );
}

#[test]
fn test_code_span_simple() {
    // Simple code spans without backticks should use single backticks
    let input = "Use `code` for inline code.";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("`code`"),
        "Simple code span should use single backticks, got:\n{}",
        result
    );
}

#[test]
fn test_code_span_starting_with_backtick() {
    // Code starting with backtick needs space padding
    let input = "The code `` `foo `` starts with a backtick.";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("`` `foo ``"),
        "Code starting with backtick should have space padding, got:\n{}",
        result
    );
}

#[test]
fn test_code_span_ending_with_backtick() {
    // Code ending with backtick needs space padding
    let input = "The code `` foo` `` ends with a backtick.";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("`` foo` ``"),
        "Code ending with backtick should have space padding, got:\n{}",
        result
    );
}

#[test]
fn test_code_span_with_trailing_space() {
    // Code span ending with a space should preserve the space without extra padding
    let input = "outputting to stderr with an `Error: ` prefix";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("`Error: `"),
        "Code span with trailing space should be preserved exactly, got:\n{}",
        result
    );
}

#[test]
fn test_code_span_with_leading_space() {
    // Code span starting with a space should preserve the space without extra padding
    let input = "The ` Error` message appeared.";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("` Error`"),
        "Code span with leading space should be preserved exactly, got:\n{}",
        result
    );
}

#[test]
fn test_code_span_with_leading_and_trailing_space() {
    // Code span with space at both start and end - per CommonMark, the parser
    // strips one space from each end. To preserve the original, we need to
    // add the spaces back in the output.
    let input = "Use ` -  ` for list items.";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("` -  `"),
        "Code span with leading and trailing space should be preserved, got:\n{}",
        result
    );
}

#[test]
fn test_reference_link_multiline_text_normalized() {
    // Reference link text spanning multiple lines should be normalized to single line
    let input = "Click [here for\nmore info][1].\n\n[1]: https://example.com";
    let result = parse_and_serialize_with_source(input);
    // The link text should be normalized (newline -> space)
    assert!(
        result.contains("[here for more info]"),
        "Reference link text should be normalized to single line, got:\n{}",
        result
    );
}

#[test]
fn test_reference_link_idempotent() {
    // Reference style link should be idempotent after formatting
    let input = "Click [here for more info][1].\n\n[1]: https://example.com";
    let result = parse_and_serialize_with_source(input);
    let result2 = parse_and_serialize_with_source(&result);
    assert_eq!(
        result, result2,
        "Reference link should be idempotent.\nFirst pass:\n{}\nSecond pass:\n{}",
        result, result2
    );
}

#[test]
fn test_code_block_in_list_item() {
    // Code block inside a list item should be properly indented
    let input = " -  Example:\n\n    ~~~~\n    code here\n    ~~~~";
    let result = parse_and_serialize_with_source(input);
    // Code block should be on a new line with proper indentation
    assert!(
        result.contains("Example:\n\n    ~~~~"),
        "Code block in list item should have blank line and indentation, got:\n{}",
        result
    );
    assert!(
        result.contains("    code here"),
        "Code block content should be indented, got:\n{}",
        result
    );
}

#[test]
fn test_code_block_in_list_item_no_language() {
    // Code block without language identifier should not add one
    let input = " -  Item:\n\n    ~~~~\n    code\n    ~~~~";
    let result = parse_and_serialize_with_source(input);
    // Should use ~~~~ without language identifier
    assert!(
        result.contains("~~~~\n"),
        "Code block should not have language identifier added, got:\n{}",
        result
    );
}

// Edge case tests

#[test]
fn test_empty_paragraph() {
    // Empty content should not crash
    let input = "\n\n\n";
    let result = parse_and_serialize(input);
    assert!(result.is_empty() || result.chars().all(|c| c.is_whitespace()));
}

#[test]
fn test_deeply_nested_list() {
    let input = " -  Level 1\n    -  Level 2\n        -  Level 3\n            -  Level 4";
    let result = parse_and_serialize_with_source(input);
    assert!(result.contains("Level 1"));
    assert!(result.contains("Level 4"));
}

#[test]
fn test_link_with_special_characters_in_url() {
    let input = "[link](https://example.com/path?query=1&other=2#anchor)";
    let result = parse_and_serialize(input);
    assert!(
        result.contains("https://example.com/path?query=1&other=2#anchor"),
        "URL with special characters should be preserved, got:\n{}",
        result
    );
}

#[test]
fn test_image_with_empty_alt() {
    let input = "![](image.png)";
    let result = parse_and_serialize(input);
    assert!(
        result.contains("![](image.png)"),
        "Image with empty alt should be preserved, got:\n{}",
        result
    );
}

#[test]
fn test_code_span_with_newlines_in_content() {
    // Code spans cannot contain literal newlines, but escaped content should work
    let input = "`code`";
    let result = parse_and_serialize(input);
    assert!(result.contains("`code`"));
}

#[test]
fn test_escaped_characters_in_text() {
    let input = r"Text with \* escaped \[ characters \]";
    let result = parse_and_serialize(input);
    // Escaped characters should be preserved
    assert!(result.contains(r"\*") || result.contains("*"));
}

#[test]
fn test_preserve_escaped_brackets_in_plain_text() {
    let input = r"Path: \[identifier\]";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(result, "Path: \\[identifier\\]\n");
}

#[test]
fn test_preserve_escaped_brackets_at_line_start() {
    let input = r"\[foo\]

[foo]: https://example.com";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(result, "\\[foo\\]\n");
}

#[test]
fn test_preserve_escaped_ascii_punctuation_idempotent() {
    let escaped_chars = [
        '[', ']', '(', ')', '!', '#', '<', '>', '{', '}', '|', '~', '-', '+', '.', '/', ':', ';',
        '=', '?', '@', '^', '$', '&',
    ];
    let escaped = escaped_chars
        .iter()
        .map(|ch| format!(r"\{}", ch))
        .collect::<Vec<_>>()
        .join(" ");
    let input = format!("Escaped punctuation: {}", escaped);

    let first_pass = parse_and_serialize_with_source_and_width(&input, 200);
    for ch in escaped_chars {
        let single_escape = format!(r"\{}", ch);
        let double_escape = format!(r"\\{}", ch);
        assert!(
            first_pass.contains(&single_escape),
            "Output should preserve {}",
            single_escape
        );
        assert!(
            !first_pass.contains(&double_escape),
            "Output should not double-escape {}",
            single_escape
        );
    }

    let second_pass = parse_and_serialize_with_source_and_width(first_pass.trim_end(), 200);
    assert_eq!(first_pass, second_pass);
}

#[test]
fn test_multiple_consecutive_code_blocks() {
    let input = "~~~~ rust\nfn main() {}\n~~~~\n\n~~~~ python\ndef main():\n    pass\n~~~~";
    let result = parse_and_serialize(input);
    assert!(result.contains("rust"));
    assert!(result.contains("python"));
}

#[test]
fn test_table_with_empty_cells() {
    let input = "| A | B |\n|---|---|\n|   | X |";
    let result = parse_and_serialize(input);
    assert!(result.contains("|"));
    assert!(result.contains("X"));
}

#[test]
fn test_blockquote_with_multiple_paragraphs() {
    let input = "> First paragraph\n>\n> Second paragraph";
    let result = parse_and_serialize(input);
    assert!(result.contains("> First paragraph"));
    assert!(result.contains("> Second paragraph"));
}

#[test]
fn test_link_text_with_emphasis() {
    let input = "[*emphasized* link](https://example.com)";
    let result = parse_and_serialize(input);
    assert!(
        result.contains("*emphasized*"),
        "Emphasis in link text should be preserved, got:\n{}",
        result
    );
}

#[test]
fn test_heading_with_special_characters() {
    let input = "# Heading with `code` and *emphasis*";
    let result = parse_and_serialize(input);
    assert!(result.contains("`code`"));
    assert!(result.contains("*emphasis*"));
}

#[test]
fn test_very_long_word_in_paragraph() {
    let input = "This is a supercalifragilisticexpialidociousandmuchmuchlongerwordthatcannotbewrapped word.";
    let result = parse_and_serialize(input);
    // Long words should not cause crashes and should be preserved
    assert!(
        result
            .contains("supercalifragilisticexpialidociousandmuchmuchlongerwordthatcannotbewrapped")
    );
}

#[test]
fn test_strikethrough_text() {
    let input = "~~strikethrough~~";
    let result = parse_and_serialize(input);
    // Strikethrough may or may not be supported, but should not crash
    assert!(!result.is_empty());
}

#[test]
fn test_mixed_ordered_unordered_lists() {
    let input = " 1. Ordered item\n\n -  Unordered item";
    let result = parse_and_serialize(input);
    assert!(result.contains("1."));
    assert!(result.contains("-"));
}

#[test]
fn test_horizontal_rule() {
    let input = "Before\n\n---\n\nAfter";
    let result = parse_and_serialize(input);
    // Horizontal rules should be preserved with default style
    assert!(result.contains("Before"));
    assert!(
        result
            .contains("- - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -")
    );
    assert!(result.contains("After"));
}

#[test]
fn test_thematic_break_default_leading_spaces() {
    let input = "Before\n\n---\n\nAfter";
    let result = parse_and_serialize(input);
    // Default leading_spaces is 3
    assert!(
        result.contains(
            "\n   - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -\n"
        ),
        "Expected 3 leading spaces by default, got:\n{}",
        result
    );
}

#[test]
fn test_thematic_break_custom_style() {
    let input = "Before\n\n---\n\nAfter";
    let mut options = Options::default();
    options.thematic_break_style = ThematicBreakStyle::new("---".to_string()).unwrap();
    options.thematic_break_leading_spaces = LeadingSpaces::new(0).unwrap();
    let result = parse_and_serialize_with_options(input, &options);
    assert!(
        result.contains("\n---\n"),
        "Expected custom style thematic break, got:\n{}",
        result
    );
}

#[test]
fn test_thematic_break_leading_spaces() {
    let input = "Before\n\n---\n\nAfter";
    let mut options = Options::default();
    options.thematic_break_style = ThematicBreakStyle::new("*  *  *".to_string()).unwrap();
    options.thematic_break_leading_spaces = LeadingSpaces::new(3).unwrap();
    let result = parse_and_serialize_with_options(input, &options);
    // 3 leading spaces should be applied
    assert!(
        result.contains("\n   *  *  *\n"),
        "Expected 3 leading spaces, got:\n{}",
        result
    );
}

#[test]
fn test_thematic_break_idempotent() {
    // Test that formatting twice produces the same result (fixes the bug)
    let input = "Before\n\n---\n\nAfter";
    let first_pass = parse_and_serialize(input);
    let second_pass = parse_and_serialize(&first_pass);
    assert_eq!(
        first_pass, second_pass,
        "Thematic break formatting should be idempotent"
    );
}

#[test]
fn test_thematic_break_various_input_styles() {
    // Test various input styles are normalized to default style
    let inputs = vec!["---", "***", "___", "- - -", "* * *", "_ _ _"];
    let expected = "- - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -";
    for input in inputs {
        let full_input = format!("Before\n\n{}\n\nAfter", input);
        let result = parse_and_serialize(&full_input);
        assert!(
            result.contains(expected),
            "Input '{}' should be normalized to '{}', got:\n{}",
            input,
            expected,
            result
        );
    }
}

#[test]
fn test_unicode_in_heading() {
    let input = "# 한글 제목";
    let result = parse_and_serialize(input);
    assert!(
        result.contains("한글 제목"),
        "Unicode heading should be preserved, got:\n{}",
        result
    );
}

#[test]
fn test_unicode_in_link_text() {
    let input = "[한글 링크](https://example.com)";
    let result = parse_and_serialize(input);
    assert!(
        result.contains("한글 링크"),
        "Unicode in link text should be preserved, got:\n{}",
        result
    );
}

#[test]
fn test_footnote_with_multiple_paragraphs() {
    let input = "Text[^1]\n\n[^1]: First paragraph of footnote";
    let result = parse_and_serialize(input);
    assert!(result.contains("[^1]"));
}

#[test]
fn test_nested_emphasis() {
    let input = "***bold and italic***";
    let result = parse_and_serialize(input);
    // Should preserve some form of emphasis
    assert!(result.contains("*"));
}

#[test]
fn test_code_block_with_blank_lines() {
    let input = "~~~~ text\nline 1\n\nline 3\n~~~~";
    let result = parse_and_serialize(input);
    assert!(result.contains("line 1"));
    assert!(result.contains("line 3"));
}

#[test]
fn test_gfm_task_list_checked() {
    let input = " - [x] Completed task";
    let result = parse_and_serialize(input);
    assert_eq!(result, " -  [x] Completed task\n");
}

#[test]
fn test_gfm_task_list_unchecked() {
    let input = " - [ ] Pending task";
    let result = parse_and_serialize(input);
    assert_eq!(result, " -  [ ] Pending task\n");
}

#[test]
fn test_gfm_task_list_mixed() {
    let input = " - [x] Done\n - [ ] Todo\n - [x] Also done";
    let result = parse_and_serialize(input);
    assert!(result.contains("[x] Done"));
    assert!(result.contains("[ ] Todo"));
    assert!(result.contains("[x] Also done"));
}

#[test]
fn test_gfm_task_list_nested() {
    let input = " - [x] Parent task\n    - [ ] Child task";
    let result = parse_and_serialize(input);
    assert!(result.contains("[x] Parent task"));
    assert!(result.contains("[ ] Child task"));
}

#[test]
fn test_definition_list_no_extra_blank_line() {
    let input = "Term\n:   Definition here";
    let result = parse_and_serialize(input);
    assert_eq!(result, "Term\n:   Definition here\n");
}

#[test]
fn test_definition_list_multiple_items() {
    let input = "Term1\n:   Definition1\n\nTerm2\n:   Definition2";
    let result = parse_and_serialize(input);
    assert!(result.contains("Term1\n:   Definition1"));
    assert!(result.contains("Term2\n:   Definition2"));
    // Should have blank line between items, but not between term and definition
    assert!(!result.contains("Term1\n\n:"));
}

#[test]
fn test_abbreviation_definition_preserved() {
    let input = "*[JSX]: JavaScript XML";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(result, "*[JSX]: JavaScript XML\n");
}

#[test]
fn test_abbreviation_definition_multiple() {
    let input = "*[HTML]: HyperText Markup Language\n\n*[CSS]: Cascading Style Sheets";
    let result = parse_and_serialize_with_source(input);
    assert!(result.contains("*[HTML]: HyperText Markup Language"));
    assert!(result.contains("*[CSS]: Cascading Style Sheets"));
}

#[test]
fn test_definition_list_with_list_as_first_child() {
    let input = "Pros\n:    -  First item\n     -  Second item";
    let result = parse_and_serialize(input);
    // The list should be on the same line as the colon to ensure idempotent formatting
    // Format: `:    -  First item` (colon + 4 spaces + list marker)
    assert!(result.contains("Pros\n:    -  First item"));
    assert!(result.contains("     -  Second item"));
}

#[test]
fn test_reference_used_in_multiple_sections_not_duplicated() {
    let input = r#"Section
-------

### First

Text with [link].

### Second

More text with [link].

[link]: https://example.com
"#;
    let result = parse_and_serialize_with_source(input);
    // The reference should appear only once, after first use
    let count = result.matches("[link]: https://example.com").count();
    assert_eq!(
        count, 1,
        "Reference should appear exactly once, but found {} times in:\n{}",
        count, result
    );
}

#[test]
fn test_heading_with_reference_link() {
    let input = r#"[BotKit] by Fedify
==================

[BotKit]: https://botkit.fedify.dev/
"#;
    let result = parse_and_serialize_with_source(input);
    // The reference link in the heading should be preserved
    assert!(
        result.contains("[BotKit] by Fedify"),
        "Heading should contain reference link syntax, got:\n{}",
        result
    );
    assert!(
        result.contains("[BotKit]: https://botkit.fedify.dev/"),
        "Reference definition should be preserved, got:\n{}",
        result
    );
}

#[test]
fn test_footnote_at_section_end_before_subheading() {
    let input = r#"Section
-------

Text with footnote[^1].

### Subsection

More text here.

[^1]: This is a footnote.
"#;
    let result = parse_and_serialize_with_source(input);
    // Footnote should appear before the subsection, not at document end
    let footnote_pos = result.find("[^1]: This is a footnote.").unwrap();
    let subsection_pos = result.find("### Subsection").unwrap();
    assert!(
        footnote_pos < subsection_pos,
        "Footnote should appear before subsection, got:\n{}",
        result
    );
}

#[test]
fn test_footnote_definition_wrapped_at_80_chars() {
    let input = r#"Text[^1].

[^1]: This is a very long footnote definition that definitely exceeds eighty characters and should be wrapped.
"#;
    let result = parse_and_serialize_with_source(input);
    // Check that no line exceeds 80 characters
    for line in result.lines() {
        assert!(
            line.len() <= 80,
            "Line exceeds 80 characters: '{}' (len={})",
            line,
            line.len()
        );
    }
    // Should still contain the footnote content
    assert!(
        result.contains("This is a very long footnote"),
        "Footnote content should be preserved, got:\n{}",
        result
    );
}

#[test]
fn test_footnote_continuation_indent_matches_prefix() {
    let input = r#"Text[^note].

[^note]: This is a long footnote with a longer name that should wrap with proper indentation.
"#;
    let result = parse_and_serialize_with_source(input);
    // The continuation line should be indented to align with content after "[^note]: "
    // "[^note]: " is 9 characters, so continuation should have 9 spaces
    assert!(
        result.contains("\n         "), // 9 spaces
        "Continuation should be indented with 9 spaces to match '[^note]: ', got:\n{}",
        result
    );
}

#[test]
fn test_table_warns_on_unescaped_pipe_in_cell() {
    use crate::format_with_warnings;

    let input = r#"| Property | Type | Required |
|----------|------|----------|
| `strategy` | `"a" | "b"` | Yes |"#;
    let result = format_with_warnings(input, &crate::Options::default()).unwrap();
    assert_eq!(result.warnings.len(), 1);
    assert!(result.warnings[0].message.contains("unescaped"));
    assert_eq!(result.warnings[0].line, 3);
}

#[test]
fn test_heading_setext_h1_disabled() {
    let options = Options {
        setext_h1: false,
        ..Options::default()
    };
    let result = parse_and_serialize_with_options("# Document Title", &options);
    assert_eq!(result, "# Document Title\n");
}

#[test]
fn test_heading_setext_h1_enabled() {
    let options = Options {
        setext_h1: true,
        ..Options::default()
    };
    let result = parse_and_serialize_with_options("# Document Title", &options);
    assert_eq!(result, "Document Title\n==============\n");
}

#[test]
fn test_heading_setext_h2_disabled() {
    let options = Options {
        setext_h2: false,
        ..Options::default()
    };
    let result = parse_and_serialize_with_options("## Section Title", &options);
    assert_eq!(result, "## Section Title\n");
}

#[test]
fn test_heading_setext_h2_enabled() {
    let options = Options {
        setext_h2: true,
        ..Options::default()
    };
    let result = parse_and_serialize_with_options("## Section Title", &options);
    assert_eq!(result, "Section Title\n-------------\n");
}

#[test]
fn test_list_unordered_marker_asterisk() {
    let options = Options {
        unordered_marker: crate::UnorderedMarker::Asterisk,
        ..Options::default()
    };
    let result = parse_and_serialize_with_options(" -  Item one\n -  Item two", &options);
    assert_eq!(result, " *  Item one\n *  Item two\n");
}

#[test]
fn test_list_unordered_marker_plus() {
    let options = Options {
        unordered_marker: crate::UnorderedMarker::Plus,
        ..Options::default()
    };
    let result = parse_and_serialize_with_options(" -  Item one\n -  Item two", &options);
    assert_eq!(result, " +  Item one\n +  Item two\n");
}

#[test]
fn test_list_unordered_marker_default() {
    let options = Options::default();
    let result = parse_and_serialize_with_options(" *  Item one\n *  Item two", &options);
    assert_eq!(result, " -  Item one\n -  Item two\n");
}

#[test]
fn test_list_leading_spaces_zero() {
    let options = Options {
        leading_spaces: LeadingSpaces::new(0).unwrap(),
        ..Options::default()
    };
    let result = parse_and_serialize_with_options(" -  Item one\n -  Item two", &options);
    assert_eq!(result, "-  Item one\n-  Item two\n");
}

#[test]
fn test_list_leading_spaces_two() {
    let options = Options {
        leading_spaces: LeadingSpaces::new(2).unwrap(),
        ..Options::default()
    };
    let result = parse_and_serialize_with_options(" -  Item one\n -  Item two", &options);
    assert_eq!(result, "  -  Item one\n  -  Item two\n");
}

#[test]
fn test_list_trailing_spaces_one() {
    let options = Options {
        trailing_spaces: TrailingSpaces::new(1).unwrap(),
        ..Options::default()
    };
    let result = parse_and_serialize_with_options(" -  Item one\n -  Item two", &options);
    assert_eq!(result, " - Item one\n - Item two\n");
}

#[test]
fn test_list_trailing_spaces_three() {
    let options = Options {
        trailing_spaces: TrailingSpaces::new(3).unwrap(),
        ..Options::default()
    };
    let result = parse_and_serialize_with_options(" -  Item one\n -  Item two", &options);
    assert_eq!(result, " -   Item one\n -   Item two\n");
}

#[test]
fn test_list_indent_width_two() {
    // indent_width=2: nested list has 2 spaces indent before " -  " prefix
    // Result: 2 spaces + " -  " = "   -  " (3 spaces before marker)
    let options = Options {
        indent_width: IndentWidth::new(2).unwrap(),
        ..Options::default()
    };
    let result = parse_and_serialize_with_options(" -  Item one\n     -  Nested", &options);
    assert_eq!(result, " -  Item one\n   -  Nested\n");
}

#[test]
fn test_list_indent_width_default() {
    // indent_width=4 (default): nested list has 4 spaces indent before " -  " prefix
    // Result: 4 spaces + " -  " = "     -  " (5 spaces before marker)
    let options = Options::default();
    let result = parse_and_serialize_with_options(" -  Item one\n     -  Nested", &options);
    assert_eq!(result, " -  Item one\n     -  Nested\n");
}

#[test]
fn test_ordered_list_odd_level_marker() {
    let options = Options {
        odd_level_marker: crate::OrderedMarker::Parenthesis,
        ..Options::default()
    };
    // trailing_spaces=2, so "N)  " format
    let result = parse_and_serialize_with_options("1. First\n2. Second", &options);
    assert_eq!(result, "1)  First\n2)  Second\n");
}

#[test]
fn test_ordered_list_even_level_marker() {
    let options = Options {
        even_level_marker: crate::OrderedMarker::Period,
        ..Options::default()
    };
    // Nested ordered list (level 2)
    // trailing_spaces=2, so "N.  " format for nested items
    let result = parse_and_serialize_with_options(
        "1. First\n    1. Nested first\n    2. Nested second",
        &options,
    );
    assert!(result.contains("1.  Nested first"), "got: {}", result);
    assert!(result.contains("2.  Nested second"), "got: {}", result);
}

#[test]
fn test_ordered_list_alternating_markers() {
    let options = Options::default();
    // Level 1 uses '.', level 2 uses ')'
    // trailing_spaces=2, so "N.  " for level 1, "N)  " for level 2
    let result = parse_and_serialize_with_options(
        "1. First\n    1. Nested first\n    2. Nested second",
        &options,
    );
    assert!(result.contains("1.  First"), "got: {}", result);
    assert!(result.contains("1)  Nested first"), "got: {}", result);
    assert!(result.contains("2)  Nested second"), "got: {}", result);
}

use crate::{FenceChar, IndentWidth, LeadingSpaces, MinFenceLength, TrailingSpaces};

#[test]
fn test_code_block_fence_char_backtick() {
    let options = Options {
        fence_char: FenceChar::Backtick,
        ..Options::default()
    };
    let result = parse_and_serialize_with_options("~~~~ rust\nfn main() {}\n~~~~", &options);
    assert!(result.starts_with("````"), "got: {}", result);
    assert!(result.contains("rust"), "got: {}", result);
}

#[test]
fn test_code_block_fence_char_default() {
    let options = Options::default();
    let result = parse_and_serialize_with_options("``` rust\nfn main() {}\n```", &options);
    assert!(result.starts_with("~~~~"), "got: {}", result);
}

#[test]
fn test_code_block_min_fence_length_three() {
    let options = Options {
        min_fence_length: MinFenceLength::new(3).unwrap(),
        ..Options::default()
    };
    let result = parse_and_serialize_with_options("~~~~ rust\nfn main() {}\n~~~~", &options);
    assert!(result.starts_with("~~~"), "got: {}", result);
    assert!(!result.starts_with("~~~~"), "got: {}", result);
}

#[test]
fn test_code_block_min_fence_length_six() {
    let options = Options {
        min_fence_length: MinFenceLength::new(6).unwrap(),
        ..Options::default()
    };
    let result = parse_and_serialize_with_options("~~~~ rust\nfn main() {}\n~~~~", &options);
    assert!(result.starts_with("~~~~~~"), "got: {}", result);
}

#[test]
fn test_code_block_space_after_fence_false() {
    let options = Options {
        space_after_fence: false,
        ..Options::default()
    };
    let result = parse_and_serialize_with_options("~~~~ rust\nfn main() {}\n~~~~", &options);
    assert!(result.contains("~~~~rust"), "got: {}", result);
}

#[test]
fn test_code_block_space_after_fence_true() {
    let options = Options {
        space_after_fence: true,
        ..Options::default()
    };
    let result = parse_and_serialize_with_options("~~~~rust\nfn main() {}\n~~~~", &options);
    assert!(result.contains("~~~~ rust"), "got: {}", result);
}

#[test]
fn test_ordered_list_long_list() {
    // For a list with 10+ items, marker width stays fixed at 4
    // Single-digit: "N.  " (2 trailing), double-digit: "NN. " (1 trailing)
    let input =
        "1. One\n2. Two\n3. Three\n4. Four\n5. Five\n6. Six\n7. Seven\n8. Eight\n9. Nine\n10. Ten";
    let result = parse_and_serialize(input);
    // Single-digit numbers have 2 trailing spaces
    assert!(result.contains("1.  One"), "got:\n{}", result);
    assert!(result.contains("9.  Nine"), "got:\n{}", result);
    // Double-digit numbers have 1 trailing space to maintain 4-char marker width
    assert!(result.contains("10. Ten"), "got:\n{}", result);
}

#[test]
fn test_ordered_list_pad_small_list() {
    // For lists with only single-digit items, no extra padding is needed
    let input = "1. One\n2. Two\n3. Three";
    let result = parse_and_serialize(input);
    // No extra padding since max number is single-digit
    assert!(result.contains("1.  One"), "got:\n{}", result);
    assert!(result.contains("2.  Two"), "got:\n{}", result);
    assert!(result.contains("3.  Three"), "got:\n{}", result);
}

#[test]
fn test_ordered_list_nested_long() {
    // Nested ordered lists maintain fixed 4-char marker width
    let input = "1. Parent one\n2. Parent two\n    1. Child one\n    2. Child two\n    3. Child three\n    4. Child four\n    5. Child five\n    6. Child six\n    7. Child seven\n    8. Child eight\n    9. Child nine\n    10. Child ten";
    let result = parse_and_serialize(input);
    // Parent list: "N.  " format
    assert!(result.contains("1.  Parent one"), "got:\n{}", result);
    assert!(result.contains("2.  Parent two"), "got:\n{}", result);
    // Child list has 10 items, nested with 4-space indent
    // Single-digit: 4 spaces + "N)  " (4 chars) = 8 total indent
    // Double-digit: 4 spaces + "NN) " (4 chars) = 8 total indent
    assert!(result.contains("    1)  Child one"), "got:\n{}", result);
    assert!(result.contains("    9)  Child nine"), "got:\n{}", result);
    assert!(result.contains("    10) Child ten"), "got:\n{}", result);
}

// Tests for undefined reference warnings

#[test]
fn test_undefined_reference_warning() {
    // When a reference link is used but not defined, a warning should be emitted
    let input = "See [undefined reference] for details.";
    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(result.warnings.len(), 1);
    assert!(result.warnings[0].message.contains("undefined reference"));
    assert!(
        result.warnings[0]
            .message
            .contains("undefined reference link")
    );
}

#[test]
fn test_defined_reference_no_warning() {
    // When a reference link is properly defined, no warning should be emitted
    let input = "See [defined reference] for details.\n\n[defined reference]: https://example.com";
    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(
        result.warnings.len(),
        0,
        "Expected no warnings but got: {:?}",
        result.warnings
    );
}

#[test]
fn test_multiple_undefined_references_warning() {
    // Multiple undefined references should each generate a warning
    let input = "See [foo] and [bar] for details.\n\n[foo]: https://example.com";
    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(
        result.warnings.len(),
        1,
        "Expected 1 warning for [bar] but got: {:?}",
        result.warnings
    );
    assert!(result.warnings[0].message.contains("bar"));
}

#[test]
fn test_undefined_full_reference_warning() {
    // Full reference style [text][label] with undefined label
    let input = "See [some text][undefined-label] for details.";
    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(result.warnings.len(), 1);
    assert!(result.warnings[0].message.contains("undefined-label"));
}

#[test]
fn test_abbreviation_definition_no_warning() {
    // PHP Markdown Extra abbreviation definitions (*[ABBR]: Full Text)
    // should not cause warnings when [ABBR] is used in the document
    let input = "The HTML specification is maintained by the W3C.\n\n*[HTML]: Hyper Text Markup Language\n*[W3C]: World Wide Web Consortium";
    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(
        result.warnings.len(),
        0,
        "Expected no warnings for abbreviations but got: {:?}",
        result.warnings
    );
}

#[test]
fn test_abbreviation_with_undefined_reference() {
    // When document has abbreviation definitions, but also undefined references,
    // only the undefined references should trigger warnings
    let input = "See the HTML spec and [undefined ref].\n\n*[HTML]: Hyper Text Markup Language";
    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(
        result.warnings.len(),
        1,
        "Expected 1 warning for [undefined ref] but got: {:?}",
        result.warnings
    );
    assert!(result.warnings[0].message.contains("undefined ref"));
}

#[test]
fn test_abbreviation_exemption_is_case_sensitive() {
    let input = "*[HTML]: Hyper Text Markup Language\n\nSee [html].";
    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(result.warnings[0].line, 3);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [html]"
    );
}

#[test]
fn test_reference_after_abbreviation_no_warning() {
    // Reference definitions that follow abbreviation definitions (without a blank line)
    // may not be parsed by comrak as reference definitions. We should still detect
    // these from the source and not warn about them.
    let input = "See [RabbitMQ] for more.\n\n*[AMQP]: Advanced Message Queuing Protocol\n[RabbitMQ]: https://www.rabbitmq.com/";
    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(
        result.warnings.len(),
        0,
        "Expected no warnings but got: {:?}",
        result.warnings
    );
}

#[test]
fn test_reference_after_abbreviation_preserves_link_brackets() {
    let input = "See [RabbitMQ] and [`AmqpMessageQueue`] for more.\n\n*[AMQP]: Advanced Message Queuing Protocol\n[`AmqpMessageQueue`]: https://jsr.io/@fedify/amqp/doc/mq/~/AmqpMessageQueue\n[RabbitMQ]: https://www.rabbitmq.com/";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("[RabbitMQ]"),
        "Reference-style link text should preserve brackets, got:\n{}",
        result
    );
    assert!(
        result.contains("[`AmqpMessageQueue`]"),
        "Code-style link text should preserve brackets, got:\n{}",
        result
    );
    assert!(
        !result.contains("\\[RabbitMQ]"),
        "Reference-style link text should not escape opening bracket, got:\n{}",
        result
    );
    assert!(
        !result.contains("\\[`AmqpMessageQueue`]"),
        "Code-style link text should not escape opening bracket, got:\n{}",
        result
    );
}

#[test]
fn test_no_warning_in_disable_enable_region() {
    // Undefined references inside hongdown-disable/enable regions
    // should not produce warnings
    let input = "Normal text.\n\n<!-- hongdown-disable -->\n\n[undefined ref] should not warn.\n\n<!-- hongdown-enable -->\n\nMore normal text.";
    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(
        result.warnings.len(),
        0,
        "Expected no warnings for disabled region but got: {:?}",
        result.warnings
    );
}

#[test]
fn test_no_warning_in_disable_next_line() {
    // Undefined reference on the line after hongdown-disable-next-line
    // should not produce a warning
    let input =
        "<!-- hongdown-disable-next-line -->\n[undefined ref] should not warn.\n\nNormal text.";
    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(
        result.warnings.len(),
        0,
        "Expected no warnings for disabled line but got: {:?}",
        result.warnings
    );
}

#[test]
fn test_no_warning_in_disable_file() {
    // Undefined references after hongdown-disable-file should not produce warnings
    let input = "<!-- hongdown-disable-file -->\n\n[undefined ref] should not warn.";
    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(
        result.warnings.len(),
        0,
        "Expected no warnings for disabled file but got: {:?}",
        result.warnings
    );
}

#[test]
fn test_no_warning_in_disable_next_section() {
    // disable-next-section disables content from the directive until the next h2/h1 heading.
    // Content BETWEEN the directive and the next heading should not produce warnings.
    let input = "First section\n-------------\n\nNormal text.\n\n<!-- hongdown-disable-next-section -->\n\n[undefined ref] should not warn.\n\nSecond section\n--------------\n\nNormal text.";
    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(
        result.warnings.len(),
        0,
        "Expected no warnings for disabled section but got: {:?}",
        result.warnings
    );
}

#[test]
fn test_warning_before_disable_region() {
    // Undefined references before a disabled region should still warn
    let input = "[undefined before] warning expected.\n\n<!-- hongdown-disable -->\n\n[undefined inside] no warning.\n\n<!-- hongdown-enable -->\n\nNormal text.";
    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(
        result.warnings.len(),
        1,
        "Expected 1 warning for text before disabled region but got: {:?}",
        result.warnings
    );
    assert!(result.warnings[0].message.contains("undefined before"));
}

#[test]
fn test_warning_after_enable() {
    // Undefined references after hongdown-enable should warn
    let input = "Normal text.\n\n<!-- hongdown-disable -->\n\n[undefined inside] no warning.\n\n<!-- hongdown-enable -->\n\n[undefined after] warning expected.";
    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(
        result.warnings.len(),
        1,
        "Expected 1 warning for text after enabled region but got: {:?}",
        result.warnings
    );
    assert!(result.warnings[0].message.contains("undefined after"));
}

#[test]
fn test_warning_after_disable_next_line() {
    // Undefined references after the disabled line should still warn
    let input = "<!-- hongdown-disable-next-line -->\n[undefined on disabled line] no warning.\n\n[undefined after] warning expected.";
    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(
        result.warnings.len(),
        1,
        "Expected 1 warning for text after disabled line but got: {:?}",
        result.warnings
    );
    assert!(result.warnings[0].message.contains("undefined after"));
}

#[test]
fn test_warning_after_disable_next_section() {
    // disable-next-section only disables content until the next h2/h1 heading.
    // Content in the next section (after the heading) should still produce warnings.
    let input = "First section\n-------------\n\nNormal text.\n\n<!-- hongdown-disable-next-section -->\n\n[undefined in disabled] no warning.\n\nSecond section\n--------------\n\n[undefined in second] warning expected.";
    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(
        result.warnings.len(),
        1,
        "Expected 1 warning for text after section heading but got: {:?}",
        result.warnings
    );
    assert!(result.warnings[0].message.contains("undefined in second"));
}

#[test]
fn test_heading_with_image() {
    // Images in headings should be preserved
    let result = parse_and_serialize("# ![logo](./logo.svg) Title");
    assert_eq!(
        result,
        "![logo](./logo.svg) Title\n=========================\n"
    );
}

#[test]
fn test_heading_with_image_no_alt() {
    // Images without alt text in headings should be preserved
    let result = parse_and_serialize("# ![](./logo.svg) Title");
    assert_eq!(result, "![](./logo.svg) Title\n=====================\n");
}

#[test]
fn test_heading_with_image_only() {
    // Heading containing only an image
    let result = parse_and_serialize("# ![logo](./logo.svg)");
    assert_eq!(result, "![logo](./logo.svg)\n===================\n");
}

#[test]
fn test_heading_link_with_nested_image_stays_inline() {
    let input = "# Project [**![CI](badge.svg)**](https://ci.example.com/)";
    let result = parse_and_serialize(input);

    assert!(
        result.starts_with("Project [**![CI](badge.svg)**](https://ci.example.com/)\n"),
        "Image-bearing link should stay inline, got:\n{result}"
    );
    assert!(!result.contains("[**![CI](badge.svg)**]:"));
    assert_eq!(parse_and_serialize(&result), result);
}

#[test]
fn test_multiline_heading_link_with_nested_image_stays_inline() {
    let input = "Project [**![CI](badge.svg)**\nbadge](https://ci.example.com/)\n===============================";
    let result = parse_and_serialize(input);

    assert!(
        result.starts_with("Project [**![CI](badge.svg)** badge](https://ci.example.com/)\n"),
        "Soft break should become a space, got:\n{result:?}"
    );
    assert!(!result.contains('\0'));
    assert_eq!(parse_and_serialize(&result), result);
}

#[test]
fn test_setext_heading_with_image_on_previous_line() {
    // When image is on a separate line before setext heading text,
    // they form a single heading (per Markdown spec)
    let result = parse_and_serialize("![](./logo.svg)\nTitle\n=====");
    assert_eq!(result, "![](./logo.svg) Title\n=====================\n");
}

#[test]
fn test_wrap_multiline_paragraph_no_orphan_words() {
    // When wrapping a paragraph with multiple original lines, ensure that
    // short words are not left orphaned on their own lines when the next
    // original line would fit on its own
    let input = "app's appropriate handler for `/users/[handle]`.  Or if you define an actor dispatcher\nfor `/users/{handle}` in Fedify, and the request is made with `Accept:\napplication/activity+json` header, Fedify will dispatch the request to the\nappropriate actor dispatcher.";
    let result = parse_and_serialize(input);
    // Should not have "the" alone on a line followed by "appropriate" starting
    // a new paragraph-like segment - this would happen if we break prematurely
    // and process "appropriate actor dispatcher." as a separate line
    assert!(
        !result.contains("the\nappropriate"),
        "Word 'the' should not be orphaned when next line fits on its own. Got:\n{}",
        result
    );
}

#[test]
fn test_definition_list_in_blockquote() {
    // Definition list inside blockquote should preserve the > prefix
    let input = "> Term\n> :   Definition here.";
    let result = parse_and_serialize(input);
    assert_eq!(result, "> Term\n> :   Definition here.\n");
}

#[test]
fn test_definition_list_in_blockquote_multiline() {
    // Multi-line definition in blockquote should preserve > prefix on all lines
    let input = "> `FC<Props>`\n> :   Applies the type argument `Props` to the generic type `FC`.\n>\n> `<Container>`\n> :   Opens a component tag.";
    let result = parse_and_serialize(input);
    assert!(
        result.contains("> :   "),
        "Definition list marker should have > prefix in blockquote. Got:\n{}",
        result
    );
    // Should not have definition marker without > prefix
    assert!(
        !result.contains("\n:   "),
        "Definition list should not lose > prefix. Got:\n{}",
        result
    );
}

#[test]
fn test_definition_list_with_alert() {
    // Alert inside definition list should preserve 4-space indent
    let input = "term\n:   First paragraph.\n\n    > [!NOTE]\n    > This is a note.\n    > It has multiple lines.";
    let result = parse_and_serialize(input);
    assert_eq!(
        result,
        "term\n:   First paragraph.\n\n    > [!NOTE]\n    > This is a note.\n    > It has multiple lines.\n"
    );
}

#[test]
fn test_definition_list_with_blockquote() {
    // Blockquote inside definition list should preserve 4-space indent
    let input = "term\n:   First paragraph.\n\n    > This is a quote.\n    > With multiple lines.";
    let result = parse_and_serialize(input);
    assert_eq!(
        result,
        "term\n:   First paragraph.\n\n    > This is a quote.\n    > With multiple lines.\n"
    );
}

#[test]
fn test_definition_list_with_alert_as_first_child() {
    // Alert as first child in definition list (note: `:   >` format required)
    let input = "term\n:   > [!TIP]\n    > This is a tip.";
    let result = parse_and_serialize(input);
    assert_eq!(result, "term\n:\n    > [!TIP]\n    > This is a tip.\n");
}

#[test]
fn test_code_block_default_no_language() {
    // By default, code blocks without a language identifier should stay without one
    let result = parse_and_serialize("```\nsome code\n```");
    assert_eq!(
        result, "~~~~\nsome code\n~~~~\n",
        "Code block without language should not have language identifier added by default"
    );
}

#[test]
fn test_code_block_custom_default_language() {
    // When default_language is set, it should be used for code blocks without a language
    let options = Options {
        default_language: "text".to_string(),
        ..Options::default()
    };
    let result = parse_and_serialize_with_options("```\nsome code\n```", &options);
    assert_eq!(
        result, "~~~~ text\nsome code\n~~~~\n",
        "Code block without language should use default_language option"
    );
}

#[test]
fn test_shortcut_link_followed_by_footnote() {
    // When an inline link is immediately followed by a footnote reference,
    // formatting converts the inline link to a reference-style link.
    // If we use shortcut style [link], the output [link][^1] is ambiguous -
    // it could be parsed as a full reference link with label "^1".
    // Use collapsed reference [link][] to disambiguate.
    let input = "See [example](https://example.com)[^1] for details.\n\n[^1]: Footnote.";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("[example][][^1]"),
        "Shortcut link followed by footnote needs empty brackets for disambiguation, got:\n{}",
        result
    );
}

#[test]
fn test_sole_html_comment_has_no_leading_blank_lines() {
    // A document whose only content is an HTML comment must not gain leading
    // blank lines: it is a "trailing" HTML block with nothing preceding it.
    let result = parse_and_serialize_with_source("<!-- hi -->\n");
    assert_eq!(result, "<!-- hi -->\n");
}

#[test]
fn test_leading_html_comment_block_then_content() {
    // An HTML comment that opens the document (followed by prose) keeps its
    // position with no spurious leading blank lines.
    let result = parse_and_serialize_with_source("<!-- hi -->\n\nText.\n");
    assert_eq!(result, "<!-- hi -->\n\nText.\n");
}

#[test]
fn test_blank_line_preserved_between_trailing_html_comments() {
    // Two HTML comments separated by a blank line must keep that separation
    // rather than being concatenated onto adjacent lines.
    let result = parse_and_serialize_with_source("<!-- a -->\n\n<!-- b -->\n");
    assert_eq!(result, "<!-- a -->\n\n<!-- b -->\n");
}

#[test]
fn test_adjacent_trailing_html_comments_stay_adjacent() {
    let result = parse_and_serialize_with_source("<!-- a -->\n<!-- b -->\n");
    assert_eq!(result, "<!-- a -->\n<!-- b -->\n");
}

#[test]
fn test_trailing_html_comment_after_references() {
    // Trailing HTML comments (like cSpell ignore directives) should remain
    // at the end of the document after reference definitions.
    let input = r#"See the [docs] for more info.

[docs]: https://example.com/docs

<!-- cSpell: ignore: mybot -->
"#;
    let result = parse_and_serialize_with_source(input);
    // The HTML comment should be at the very end, after the reference definition
    // with a blank line before it
    assert!(
        result.ends_with("\n\n<!-- cSpell: ignore: mybot -->\n"),
        "Trailing HTML comment should remain at the end with blank line before, got:\n{}",
        result
    );
    // Reference definition should come before the HTML comment
    let lines: Vec<&str> = result.lines().collect();
    let comment_pos = lines.iter().position(|l| l.contains("cSpell")).unwrap();
    let ref_pos = lines.iter().position(|l| l.starts_with("[docs]:")).unwrap();
    assert!(
        ref_pos < comment_pos,
        "Reference definition should come before trailing HTML comment, got:\n{}",
        result
    );
}

#[test]
fn test_trailing_html_comment_with_external_link() {
    // When a document has an external link (which gets converted to reference style)
    // and a trailing HTML comment, the comment should stay at the very end.
    let input = r#"Check [example](https://example.com) for details.

<!-- cSpell: ignore: mybot -->
"#;
    let result = parse_and_serialize_with_source(input);
    // The HTML comment should be at the very end, after the reference definition
    // with a blank line before it
    assert!(
        result.ends_with("\n\n<!-- cSpell: ignore: mybot -->\n"),
        "Trailing HTML comment should remain at the end with blank line before, got:\n{}",
        result
    );
    // The reference definition should come before the comment
    let lines: Vec<&str> = result.lines().collect();
    let comment_pos = lines.iter().position(|l| l.contains("cSpell")).unwrap();
    let ref_pos = lines
        .iter()
        .position(|l| l.starts_with("[example]:"))
        .unwrap();
    assert!(
        ref_pos < comment_pos,
        "Reference definition should come before trailing HTML comment"
    );
}

#[test]
fn test_multiple_trailing_html_comments() {
    // Multiple trailing HTML comments should all stay at the end
    let input = r#"See [docs](https://example.com/docs) here.

<!-- Comment 1 -->
<!-- Comment 2 -->
"#;
    let result = parse_and_serialize_with_source(input);
    // There should be a blank line before the first trailing comment
    assert!(
        result.ends_with("\n\n<!-- Comment 1 -->\n<!-- Comment 2 -->\n"),
        "Multiple trailing HTML comments should remain at end with blank line before, got:\n{}",
        result
    );
}

#[test]
fn test_html_comment_not_at_end_stays_in_place() {
    // HTML comments that are not at the end should stay in their original position
    let input = r#"First paragraph.

<!-- Middle comment -->

Second paragraph with [link](https://example.com).
"#;
    let result = parse_and_serialize_with_source(input);
    // The middle comment should come before "Second paragraph"
    let lines: Vec<&str> = result.lines().collect();
    let comment_pos = lines
        .iter()
        .position(|l| l.contains("Middle comment"))
        .unwrap();
    let second_para_pos = lines
        .iter()
        .position(|l| l.contains("Second paragraph"))
        .unwrap();
    assert!(
        comment_pos < second_para_pos,
        "Middle HTML comment should stay before second paragraph"
    );
}

#[test]
fn test_definition_list_in_alert_with_multiple_items() {
    // Multiple definition list items inside an alert should preserve the > prefix
    // on blank lines between items, so the alert doesn't get split into multiple pieces
    let input = r#"> [!TIP]
> It takes several kinds of objects as an argument, such as `Actor`, `string`,
> and `URL`:
>
> `Actor`
> :   The actor to follow.
>
> `URL`
> :   The URI of the actor to follow.
>     E.g., `new URL("https://example.com/users/alice")`.
>
> `string`
> :   The URI or the fediverse handle of the actor to follow.
>     E.g., `"https://example.com/users/alice"` or `"@alice@example.com"`."#;
    let result = parse_and_serialize(input);

    // The blank lines between definition items should have "> " prefix
    // to keep them inside the alert
    assert!(
        !result.contains("\n\n> `URL`"),
        "Definition list items should not be separated by empty lines without >. Got:\n{}",
        result
    );
    assert!(
        !result.contains("\n\n> `string`"),
        "Definition list items should not be separated by empty lines without >. Got:\n{}",
        result
    );

    // Should contain blank quote lines between items
    assert!(
        result.contains(">\n> `URL`") || result.contains(">\n>\n> `URL`"),
        "Should have > prefix on blank lines between items. Got:\n{}",
        result
    );
}

#[test]
fn test_nested_blockquote_preserved() {
    // Nested blockquotes should preserve their nesting level
    let input = "> Outer\n>\n> > Inner";
    let result = parse_and_serialize(input);
    assert!(
        result.contains("> > Inner") || result.contains(">> Inner"),
        "Nested blockquote should preserve double > prefix. Got:\n{}",
        result
    );
}

#[test]
fn test_nested_blockquote_with_alert() {
    // Alert nested inside blockquote should preserve both levels
    let input = r#"> Outer blockquote:
>
> > [!TIP]
> > This is a tip inside nested blockquote."#;
    let result = parse_and_serialize(input);
    assert!(
        result.contains("> > [!TIP]") || result.contains(">> [!TIP]"),
        "Alert inside blockquote should preserve double > prefix. Got:\n{}",
        result
    );
    assert!(
        result.contains("> > This is a tip") || result.contains(">> This is a tip"),
        "Alert content should preserve double > prefix. Got:\n{}",
        result
    );
}

#[test]
fn test_nested_blockquote_with_definition_list() {
    // Definition list inside nested blockquote should preserve all levels
    let input = r#"> Here's a blockquote inside another blockquote:
>
> > [!TIP]
> > It takes several kinds of objects:
> >
> > `Actor`
> > :   The actor to follow.
> >
> > `URL`
> > :   The URI of the actor."#;
    let result = parse_and_serialize(input);

    // Should not flatten to single >
    assert!(
        !result.contains("\n> `Actor`\n> :"),
        "Definition list should not lose outer blockquote prefix. Got:\n{}",
        result
    );

    // Should preserve double > on blank lines between items
    assert!(
        result.contains("> >\n> > `URL`") || result.contains("> >\n> >\n> > `URL`"),
        "Blank lines between items should have double > prefix. Got:\n{}",
        result
    );
}

#[test]
fn test_footnote_reference_definitions_stay_below_footnote() {
    // When a footnote contains reference links, the reference definitions
    // should remain below the footnote definition, not move above it.
    // See: https://github.com/dahlia/hongdown/issues/XXX
    let input = r#"Text
====

The text.[^1]
Blocks are usually used for paragraphs.

[^1]: More precisely, the `Text` type has two type parameters: the first one
      is the type of the element: `"block"` or `"inline"`, and the second one
      is [`TContextData`], the [Fedify context data].

[`TContextData`]: https://fedify.dev/manual/federation#tcontextdata
[Fedify context data]: https://fedify.dev/manual/context
"#;
    let result = parse_and_serialize(input);

    // The footnote definition should come before the reference definitions
    let footnote_pos = result.find("[^1]:").expect("footnote not found");
    let ref1_pos = result
        .find("[`TContextData`]:")
        .expect("TContextData ref not found");
    let ref2_pos = result
        .find("[Fedify context data]:")
        .expect("Fedify context data ref not found");

    assert!(
        footnote_pos < ref1_pos,
        "Footnote should come before TContextData reference definition.\nGot:\n{}",
        result
    );
    assert!(
        footnote_pos < ref2_pos,
        "Footnote should come before Fedify context data reference definition.\nGot:\n{}",
        result
    );
}

#[test]
fn test_footnote_references_at_section_boundary() {
    // When a footnote with reference links is in a section followed by another section,
    // both the footnote and its reference definitions should appear before the next section,
    // with the footnote coming first and references coming after.
    let input = r#"Title
=====

Introduction paragraph.


First section
-------------

Some text with footnote.[^1]

[^1]: This footnote references [`Link1`] and [Link2].

[`Link1`]: https://example.com/link1
[Link2]: https://example.com/link2


Second section
--------------

More content here.
"#;
    let result = parse_and_serialize(input);

    // Find positions of key elements
    let first_section_pos = result
        .find("First section")
        .expect("First section not found");
    let second_section_pos = result
        .find("Second section")
        .expect("Second section not found");
    let footnote_pos = result.find("[^1]:").expect("footnote not found");
    let ref1_pos = result.find("[`Link1`]:").expect("Link1 ref not found");
    let ref2_pos = result.find("[Link2]:").expect("Link2 ref not found");

    // All should be between first and second section
    assert!(
        footnote_pos > first_section_pos && footnote_pos < second_section_pos,
        "Footnote should be in first section.\nGot:\n{}",
        result
    );
    assert!(
        ref1_pos > first_section_pos && ref1_pos < second_section_pos,
        "Link1 reference should be in first section.\nGot:\n{}",
        result
    );
    assert!(
        ref2_pos > first_section_pos && ref2_pos < second_section_pos,
        "Link2 reference should be in first section.\nGot:\n{}",
        result
    );

    // Footnote should come before references
    assert!(
        footnote_pos < ref1_pos,
        "Footnote should come before Link1 reference.\nGot:\n{}",
        result
    );
    assert!(
        footnote_pos < ref2_pos,
        "Footnote should come before Link2 reference.\nGot:\n{}",
        result
    );
}

#[test]
fn test_body_reference_shared_with_later_footnote_stays_in_body_section() {
    let input = r#"Section one
-----------

Body [guide] link.

Section two
-----------

Text with a footnote[^1].

[^1]: See [guide] as well.

[guide]: https://example.com/guide
"#;
    let result = parse_and_serialize(input);
    let expected = r#"Section one
-----------

Body [guide] link.

[guide]: https://example.com/guide


Section two
-----------

Text with a footnote[^1].

[^1]: See [guide] as well.
"#;

    assert_eq!(result, expected);
    assert_eq!(parse_and_serialize(&result), expected);
}

#[test]
fn test_numbered_body_reference_shared_with_later_footnote_stays_in_body_section() {
    let input = r#"Section one
-----------

Body [guide](https://example.com/b) link.

Section two
-----------

Text with a footnote[^1].

[^1]: Compare [guide](https://example.com/a) and [guide](https://example.com/b).
"#;
    let result = parse_and_serialize(input);
    let expected = r#"Section one
-----------

Body [guide][guide 2] link.

[guide 2]: https://example.com/b


Section two
-----------

Text with a footnote[^1].

[^1]: Compare [guide] and [guide][guide 2].

[guide]: https://example.com/a
"#;

    assert_eq!(result, expected);
    assert_eq!(parse_and_serialize(&result), expected);
}

#[test]
fn test_shared_reference_stays_below_footnote_in_same_section() {
    let input = r#"Section
-------

Body [shared] link with a footnote[^1].

[^1]: See [other] and [shared].

[other]: https://example.com/other
[shared]: https://example.com/shared
"#;
    let result = parse_and_serialize(input);
    let expected = r#"Section
-------

Body [shared] link with a footnote[^1].

[^1]: See [other] and [shared].

[shared]: https://example.com/shared

[other]: https://example.com/other
"#;

    assert_eq!(result, expected);
    assert_eq!(parse_and_serialize(&result), expected);
}

#[test]
fn test_reference_shared_by_footnotes_stays_in_first_section() {
    let input = r#"Section one
-----------

First footnote[^a].

Section two
-----------

Second footnote[^b].

[^b]: Later [guide].
[^a]: Earlier [guide].

[guide]: https://example.com/guide
"#;
    let result = parse_and_serialize(input);
    let expected = r#"Section one
-----------

First footnote[^a].

[^a]: Earlier [guide].

[guide]: https://example.com/guide


Section two
-----------

Second footnote[^b].

[^b]: Later [guide].
"#;

    assert_eq!(result, expected);
    assert_eq!(parse_and_serialize(&result), expected);
}

#[test]
fn test_preserve_html_entities() {
    // HTML entities like &lt; and &gt; should be preserved, not decoded
    let input = "HTML에는 &lt;strong&gt;태그 등 여러 가지 태그가 있습니다.";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(
        result,
        "HTML에는 &lt;strong&gt;태그 등 여러 가지 태그가 있습니다.\n"
    );
}

#[test]
fn test_preserve_html_entity_amp() {
    // &amp; should be preserved
    let input = "Tom &amp; Jerry";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(result, "Tom &amp; Jerry\n");
}

#[test]
fn test_preserve_html_entity_nbsp() {
    // &nbsp; should be preserved
    let input = "Hello&nbsp;world";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(result, "Hello&nbsp;world\n");
}

#[test]
fn test_preserve_numeric_html_entity() {
    // Numeric entities like &#60; should be preserved
    let input = "Entity: &#60;tag&#62;";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(result, "Entity: &#60;tag&#62;\n");
}

#[test]
fn test_preserve_actual_html_tags() {
    // Actual HTML tags should be kept as-is (not escaped)
    let input = "HTML에는 <strong>태그 등</strong> 여러 가지 태그가 있습니다.";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(
        result,
        "HTML에는 <strong>태그 등</strong> 여러 가지 태그가 있습니다.\n"
    );
}

#[test]
fn test_mixed_html_and_entities() {
    // Mixed actual HTML and entities should both be preserved correctly
    let input = "Use <code>&lt;div&gt;</code> for containers.";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(result, "Use <code>&lt;div&gt;</code> for containers.\n");
}

#[test]
fn test_footnote_definitions_before_reference_definitions() {
    // When a section has both footnote definitions and link reference definitions,
    // footnote definitions should come before link reference definitions.
    let input = r#"Section
-------

See [example] and footnote[^1].

[example]: https://example.com
[^1]: Footnote content.
"#;
    let result = parse_and_serialize(input);

    // Find positions
    let footnote_pos = result.find("[^1]:").expect("footnote not found");
    let reference_pos = result.find("[example]:").expect("reference not found");

    // Footnote should come before reference
    assert!(
        footnote_pos < reference_pos,
        "Footnote definition should come before link reference definition.\nGot:\n{}",
        result
    );
}

#[test]
fn test_numeric_footnotes_sorted_at_end() {
    // Numeric footnotes should be sorted by number and placed at the end
    // (similar to link reference definitions)
    let input = r#"This[^2] sentence[^non-numeric-b] has some footnotes.[^1]
This sentence[^non-numeric-a] also has a footnote.[^3]

[^2]: This is the second footnote.
[^non-numeric-b]: This is another non-numeric footnote.
[^1]: This is the first footnote.
[^non-numeric-a]: This is a non-numeric footnote.
[^3]: This is the third footnote.
"#;
    let result = parse_and_serialize_with_footnotes(input);

    // Check that non-numeric footnotes come before numeric ones
    let non_numeric_b_pos = result
        .find("[^non-numeric-b]:")
        .expect("non-numeric-b not found");
    let non_numeric_a_pos = result
        .find("[^non-numeric-a]:")
        .expect("non-numeric-a not found");
    let footnote_1_pos = result.find("[^1]:").expect("footnote 1 not found");
    let footnote_2_pos = result.find("[^2]:").expect("footnote 2 not found");
    let footnote_3_pos = result.find("[^3]:").expect("footnote 3 not found");

    // Non-numeric footnotes should come before numeric footnotes
    assert!(
        non_numeric_b_pos < footnote_1_pos && non_numeric_a_pos < footnote_1_pos,
        "Non-numeric footnotes should come before numeric ones.\nGot:\n{}",
        result
    );

    // Numeric footnotes should be sorted: 1 < 2 < 3
    assert!(
        footnote_1_pos < footnote_2_pos && footnote_2_pos < footnote_3_pos,
        "Numeric footnotes should be sorted by number.\nGot:\n{}",
        result
    );
}

#[test]
fn test_single_numeric_footnote_not_sorted() {
    // A single numeric footnote should stay in insertion order
    let input = r#"Text[^foo] and[^1] and[^bar].

[^foo]: Foo footnote.
[^1]: Numeric footnote.
[^bar]: Bar footnote.
"#;
    let result = parse_and_serialize_with_footnotes(input);

    // With only one numeric footnote, it stays in insertion order
    let foo_pos = result.find("[^foo]:").expect("foo not found");
    let one_pos = result.find("[^1]:").expect("1 not found");
    let bar_pos = result.find("[^bar]:").expect("bar not found");

    assert!(
        foo_pos < one_pos && one_pos < bar_pos,
        "Single numeric footnote should stay in insertion order.\nGot:\n{}",
        result
    );
}

#[test]
fn test_hard_line_break_in_blockquote() {
    // Hard line breaks (two trailing spaces) in a block quote should preserve
    // the `>` prefix on the continuation line.
    // Two trailing spaces create a hard line break (LineBreak node).
    let input = "> This is a block quote with a hard line break.  \n> This is the second line of the block quote.";
    let result = parse_and_serialize(input);
    assert_eq!(
        result,
        "> This is a block quote with a hard line break.  \n> This is the second line of the block quote.\n",
        "Hard line break in blockquote should preserve > prefix on continuation line"
    );
}

#[test]
fn test_hard_line_break_in_nested_blockquote() {
    // Hard line breaks in nested blockquotes should preserve all levels of `>` prefix.
    let input = "> > This is a nested quote.  \n> > This is after hard line break.";
    let result = parse_and_serialize(input);
    assert!(
        result.contains("> > This is after hard line break.")
            || result.contains(">> This is after hard line break."),
        "Hard line break in nested blockquote should preserve double > prefix.\nGot:\n{}",
        result
    );
}

#[test]
fn test_multiple_hard_line_breaks_in_blockquote() {
    // Multiple hard line breaks in a block quote should all preserve the prefix.
    let input = "> Line one.  \n> Line two.  \n> Line three.";
    let result = parse_and_serialize(input);
    assert_eq!(
        result, "> Line one.  \n> Line two.  \n> Line three.\n",
        "Multiple hard line breaks should preserve > prefix on all lines"
    );
}

#[test]
fn test_hard_line_break_in_alert() {
    // Hard line breaks in GitHub alerts should preserve the `>` prefix.
    let input = "> [!NOTE]\n> First line.  \n> Second line after hard break.";
    let result = parse_and_serialize(input);
    assert!(
        result.contains("> Second line after hard break."),
        "Hard line break in alert should preserve > prefix.\nGot:\n{}",
        result
    );
}

#[test]
fn test_hard_line_break_in_blockquote_with_emphasis() {
    // Hard line breaks with inline formatting should work correctly.
    let input = "> *First* line.  \n> *Second* line.";
    let result = parse_and_serialize(input);
    assert_eq!(
        result, "> *First* line.  \n> *Second* line.\n",
        "Hard line break with emphasis should preserve > prefix"
    );
}

#[test]
fn test_multiline_code_span_in_list_item() {
    // Code spans that span multiple lines in the source should be normalized
    // to a single line with spaces (per CommonMark spec: newlines in code spans
    // become spaces). The wrapping logic should not break inside code spans.
    let input = " -  Changed the type of `TextFormatterOptions.value` to `(value: unknown,
       inspect: (value: unknown, options?: { colors?: boolean }) => string)
       => string` (was `(value: unknown) => string`).";
    let result = parse_and_serialize_with_source(input);
    // The code span should not be broken apart - it should be kept intact
    // (either on one line or properly wrapped without breaking inside)
    assert!(
        !result.contains("i        nspect"),
        "Code span should not be broken with extra spaces inside.\nGot:\n{}",
        result
    );
    assert!(
        !result.contains("=        >"),
        "Code span should not be broken with extra spaces inside.\nGot:\n{}",
        result
    );
    // The code span content should be present (newlines converted to spaces)
    assert!(
        result.contains("(value: unknown, inspect:"),
        "Code span should have newlines converted to spaces.\nGot:\n{}",
        result
    );
}

#[test]
fn test_code_block_empty_line_in_blockquote() {
    let input = "> Here is a code block with an empty line:
>
> ~~~~ python
> def example_function():
>
>     print(\"Hello, World!\")
> ~~~~
";
    let result = parse_and_serialize(input);
    // Empty lines inside code blocks within blockquotes should be just ">"
    // without a trailing space
    assert_eq!(
        result,
        "> Here is a code block with an empty line:
>
> ~~~~ python
> def example_function():
>
>     print(\"Hello, World!\")
> ~~~~
"
    );
}

#[test]
fn test_code_block_empty_line_in_definition_list() {
    let input = "Foo
:   The following is a code block with an empty line.

    ~~~~ python
    print(\"Hello\")

    print(\"world\")
    ~~~~

Bar
:   Another definition.
";
    let result = parse_and_serialize(input);
    // Empty lines inside code blocks within definition lists should have no indentation
    assert_eq!(
        result,
        "Foo
:   The following is a code block with an empty line.

    ~~~~ python
    print(\"Hello\")

    print(\"world\")
    ~~~~

Bar
:   Another definition.
"
    );
}

#[test]
fn test_serialize_table_with_fullwidth_characters() {
    // Full-width characters (CJK, emoji) should take 2 display columns
    let input = "| Name | Value |\n| ---- | ----: |\n| 한글 | 100 |\n| AB | 2000 |";
    let result = parse_and_serialize_with_table(input);
    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(lines.len(), 4, "Table should have 4 lines");

    // "한글" has 2 characters but takes 4 display columns (2 each)
    // "AB" has 2 characters and takes 2 display columns
    // So minimum column width should be 4 for Name column
    // For right-aligned Value column:
    // "Value" = 5 display columns (header)
    // "100" = 3 display columns, "2000" = 4 display columns
    // The pipes should align properly when displayed in a terminal

    // All rows should have pipes at the same display positions
    // Since 한글 takes 4 columns and AB takes 2, AB needs 2 extra spaces
    assert!(
        lines[2].contains("| 한글"),
        "Korean text should be in table, got:\n{}",
        result
    );
    assert!(
        lines[3].contains("| AB"),
        "ASCII text should be in table, got:\n{}",
        result
    );

    // Check that the right-aligned column aligns properly
    // The pipes after the Value column should be at the same byte position
    // when accounting for display width
    let pipe_positions_row2: Vec<_> = lines[2].match_indices('|').map(|(i, _)| i).collect();
    let pipe_positions_row3: Vec<_> = lines[3].match_indices('|').map(|(i, _)| i).collect();

    // In a properly formatted table with full-width support,
    // the row with "한글" (4 display cols) should have different byte offsets
    // than the row with "AB" (2 display cols) for the second pipe
    // but the display width should be the same
    assert_eq!(
        pipe_positions_row2.len(),
        pipe_positions_row3.len(),
        "Both rows should have same number of pipes"
    );
}

#[test]
fn test_serialize_table_fullwidth_right_alignment() {
    // Right-aligned column with full-width characters
    let input = "| Item | Price |\n| ---: | ----: |\n| 사과 | 1000 |\n| AB | 50 |";
    let result = parse_and_serialize_with_table(input);

    // "사과" = 4 display columns, "AB" = 2 display columns
    // For right alignment, AB should have 2 extra spaces on the left
    // to align with 사과 in display width

    // When rendered in a terminal, both rows should have aligned pipes
    assert!(result.contains("사과"), "Korean text should be preserved");
    assert!(result.contains("AB"), "ASCII text should be preserved");

    // The actual validation: check that ASCII row has more padding
    let lines: Vec<&str> = result.lines().collect();
    let ascii_row = lines[3]; // |   AB |   50 |

    // In the ASCII row, there should be extra spaces before "AB" to compensate
    // for the display width difference
    assert!(
        ascii_row.contains("|   AB"),
        "AB should be padded with extra spaces for display width alignment, got:\n{}",
        result
    );
}

#[test]
fn test_serialize_table_fullwidth_center_alignment() {
    // Center-aligned column with full-width characters
    let input = "| Item | Value |\n| :--: | :---: |\n| 가 | A |\n| ABCD | 나 |";
    let result = parse_and_serialize_with_table(input);

    // "가" = 2 display columns, "ABCD" = 4 display columns
    // For center alignment, "가" needs 1 space on each side to match ABCD's width
    // "나" = 2 display columns, "A" = 1 display column
    // "A" needs more padding than "나" when centered

    assert!(result.contains("가"), "Korean text should be preserved");
    assert!(result.contains("나"), "Korean text should be preserved");

    // Check that the table renders with proper alignment
    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(lines.len(), 4, "Table should have 4 lines");
}

// =============================================================================
// East Asian Wide Character Wrapping Tests
// =============================================================================

#[test]
fn test_wrap_korean_text_at_display_width() {
    // Korean characters are 2 display columns each
    // "안녕하세요" = 5 chars * 2 cols = 10 display columns
    // "안녕하세요 안녕하세요" = 10 + 1 + 10 = 21 display columns > 20
    let input = "안녕하세요 안녕하세요 세계";
    let result = parse_and_serialize_with_width(input, 20);
    assert_eq!(
        result,
        r#"
안녕하세요
안녕하세요 세계
"#
        .trim_start_matches('\n')
    );
}

#[test]
fn test_wrap_mixed_ascii_korean() {
    // "Hello" = 5 cols, "안녕" = 4 cols, "World" = 5 cols
    let input = "Hello 안녕 World more text here";
    let result = parse_and_serialize_with_width(input, 20);
    assert_eq!(
        result,
        r#"
Hello 안녕 World
more text here
"#
        .trim_start_matches('\n')
    );
}

#[test]
fn test_wrap_japanese_text() {
    // Japanese hiragana/katakana/kanji are also 2 display columns each
    // Text with spaces to allow wrapping at word boundaries
    let input = "これは 日本語の テストです 行の折り返しが 正しく動作する";
    let result = parse_and_serialize_with_width(input, 30);
    assert_eq!(
        result,
        r#"
これは 日本語の テストです
行の折り返しが 正しく動作する
"#
        .trim_start_matches('\n')
    );
}

#[test]
fn test_wrap_chinese_text() {
    // Chinese characters are 2 display columns each
    // Text with spaces to allow wrapping at word boundaries
    let input = "这是 一个中文 测试 它应该在 正确的显示 宽度处换行";
    let result = parse_and_serialize_with_width(input, 30);
    assert_eq!(
        result,
        r#"
这是 一个中文 测试 它应该在
正确的显示 宽度处换行
"#
        .trim_start_matches('\n')
    );
}

#[test]
fn test_wrap_korean_in_list_item() {
    // List item with Korean text that needs wrapping
    // " -  " = 4 cols prefix, which counts toward the first line width.
    let input = " -  이것은 매우 긴 한국어 문장입니다 여러 줄로 나누어져야 합니다";
    let result = parse_and_serialize_with_width(input, 40);
    assert_eq!(
        result,
        r#"
 -  이것은 매우 긴 한국어 문장입니다
    여러 줄로 나누어져야 합니다
"#
        .trim_start_matches('\n')
    );
}

#[test]
fn test_unordered_list_first_line_respects_line_width() {
    let input = " -  This list item is long enough to expose whether the first line still ignores the marker width during wrapping.";
    let result = parse_and_serialize_with_width(input, 40);

    assert_all_non_empty_lines_fit_display_width(&result, 40);
}

#[test]
fn test_ordered_list_first_line_respects_line_width() {
    let input = "1.  This ordered list item is long enough to expose whether the first line still ignores the marker width during wrapping.";
    let result = parse_and_serialize_with_width(input, 40);

    assert_all_non_empty_lines_fit_display_width(&result, 40);
}

#[test]
fn test_list_item_in_alert_first_line_respects_line_width() {
    let input = "> [!NOTE]\n>  -  This list item inside an alert is long enough to expose whether the first line ignores the visible prefix width during wrapping.";
    let result = parse_and_serialize_with_source_and_width(input, 50);

    assert_all_non_empty_lines_fit_display_width(&result, 50);
}

#[test]
fn test_definition_list_paragraph_first_line_respects_line_width() {
    let input = "Term\n:   This is a very long definition paragraph that should reveal whether a prefixed first line can exceed the configured line width when wrapping happens.";
    let result = parse_and_serialize_with_source_and_width(input, 80);

    assert_all_non_empty_lines_fit_display_width(&result, 80);
}

#[test]
fn test_korean_line_exactly_at_width_limit() {
    // "가나다라마" = 5 chars * 2 cols = 10 display columns
    // "가나다라마 바사아자" = 10 + 1 + 8 = 19 cols, fits in 20
    let input = "가나다라마 바사아자";
    let result = parse_and_serialize_with_width(input, 20);
    assert_eq!(
        result,
        r#"
가나다라마 바사아자
"#
        .trim_start_matches('\n')
    );
}
// Setext Heading Display Width Tests
// =============================================================================

#[test]
fn test_setext_h1_with_fullwidth_characters() {
    // "한글 제목" = 4 wide chars (8 cols) + 1 space = 9 display columns
    // Input has 5 chars (character count), output should have 9 (display width)
    let input = "한글 제목\n=====\n";
    let result = parse_and_serialize(input);
    assert_eq!(
        result,
        r#"
한글 제목
=========
"#
        .trim_start_matches('\n')
    );
}

#[test]
fn test_setext_h2_with_fullwidth_characters() {
    // "한글 제목" = 4 wide chars (8 cols) + 1 space = 9 display columns
    // Input has 5 chars (character count), output should have 9 (display width)
    let input = "한글 제목\n-----\n";
    let result = parse_and_serialize(input);
    assert_eq!(
        result,
        r#"
한글 제목
---------
"#
        .trim_start_matches('\n')
    );
}

// =============================================================================
// Punctuation Transformation Tests (SmartyPants-style)
// =============================================================================

// Unicode constants for curly quotes (using escape sequences for Claude compatibility)
const LEFT_DOUBLE_QUOTE: char = '\u{201C}';
const RIGHT_DOUBLE_QUOTE: char = '\u{201D}';
const LEFT_SINGLE_QUOTE: char = '\u{2018}';
const RIGHT_SINGLE_QUOTE: char = '\u{2019}';
const ELLIPSIS: char = '\u{2026}';
const EM_DASH: char = '\u{2014}';
const EN_DASH: char = '\u{2013}';

#[test]
fn test_punctuation_curly_double_quotes_in_paragraph() {
    // Default: curly_double_quotes is enabled
    let input = "He said \"hello\" to her.";
    let result = parse_and_serialize(input);
    let expected = format!(
        "He said {}hello{} to her.\n",
        LEFT_DOUBLE_QUOTE, RIGHT_DOUBLE_QUOTE
    );
    assert_eq!(result, expected);
}

#[test]
fn test_punctuation_curly_double_quotes_disabled() {
    let mut options = Options::default();
    options.curly_double_quotes = false;
    let input = "He said \"hello\" to her.";
    let result = parse_and_serialize_with_options(input, &options);
    assert_eq!(result, "He said \"hello\" to her.\n");
}

#[test]
fn test_punctuation_curly_single_quotes_in_paragraph() {
    // Default: curly_single_quotes is enabled
    let input = "She said 'hello' to him.";
    let result = parse_and_serialize(input);
    let expected = format!(
        "She said {}hello{} to him.\n",
        LEFT_SINGLE_QUOTE, RIGHT_SINGLE_QUOTE
    );
    assert_eq!(result, expected);
}

#[test]
fn test_punctuation_curly_single_quotes_disabled() {
    let mut options = Options::default();
    options.curly_single_quotes = false;
    let input = "She said 'hello' to him.";
    let result = parse_and_serialize_with_options(input, &options);
    assert_eq!(result, "She said 'hello' to him.\n");
}

#[test]
fn test_punctuation_ellipsis_in_paragraph() {
    // Default: ellipsis is enabled
    let input = "Wait for it...";
    let result = parse_and_serialize(input);
    let expected = format!("Wait for it{}\n", ELLIPSIS);
    assert_eq!(result, expected);
}

#[test]
fn test_punctuation_ellipsis_disabled() {
    let mut options = Options::default();
    options.ellipsis = false;
    let input = "Wait for it...";
    let result = parse_and_serialize_with_options(input, &options);
    assert_eq!(result, "Wait for it...\n");
}

#[test]
fn test_punctuation_em_dash_default() {
    // Default: em_dash is "--"
    let input = "Hello--world";
    let result = parse_and_serialize(input);
    let expected = format!("Hello{}world\n", EM_DASH);
    assert_eq!(result, expected);
}

#[test]
fn test_punctuation_em_dash_disabled() {
    let mut options = Options::default();
    options.em_dash = crate::DashSetting::Disabled;
    let input = "Hello--world";
    let result = parse_and_serialize_with_options(input, &options);
    assert_eq!(result, "Hello--world\n");
}

#[test]
fn test_punctuation_em_dash_triple_hyphen() {
    let mut options = Options::default();
    options.em_dash =
        crate::DashSetting::Pattern(crate::DashPattern::new("---".to_string()).unwrap());
    let input = "Hello---world";
    let result = parse_and_serialize_with_options(input, &options);
    let expected = format!("Hello{}world\n", EM_DASH);
    assert_eq!(result, expected);
}

#[test]
fn test_punctuation_en_dash_disabled_by_default() {
    // Default: en_dash is disabled
    let input = "Pages 10--20";
    let result = parse_and_serialize(input);
    // Since em_dash is "--" by default, this becomes em-dash
    let expected = format!("Pages 10{}20\n", EM_DASH);
    assert_eq!(result, expected);
}

#[test]
fn test_punctuation_en_dash_enabled() {
    let mut options = Options::default();
    options.em_dash =
        crate::DashSetting::Pattern(crate::DashPattern::new("---".to_string()).unwrap());
    options.en_dash =
        crate::DashSetting::Pattern(crate::DashPattern::new("--".to_string()).unwrap());
    let input = "Pages 10--20 and a long---dash";
    let result = parse_and_serialize_with_options(input, &options);
    let expected = format!("Pages 10{}20 and a long{}dash\n", EN_DASH, EM_DASH);
    assert_eq!(result, expected);
}

#[test]
fn test_punctuation_apostrophes_disabled_by_default() {
    // Default: curly_apostrophes is disabled
    let input = "It's a test";
    let result = parse_and_serialize(input);
    assert_eq!(result, "It's a test\n");
}

#[test]
fn test_punctuation_apostrophes_enabled() {
    let mut options = Options::default();
    options.curly_apostrophes = true;
    let input = "It's a test";
    let result = parse_and_serialize_with_options(input, &options);
    let expected = format!("It{}s a test\n", RIGHT_SINGLE_QUOTE);
    assert_eq!(result, expected);
}

#[test]
fn test_punctuation_no_transform_in_inline_code() {
    // Code spans should NOT have punctuation transformed
    let input = "Use `\"hello\"` for strings.";
    let result = parse_and_serialize(input);
    // The quotes inside backticks should remain straight
    assert!(
        result.contains("`\"hello\"`"),
        "Quotes in code spans should not be transformed, got:\n{}",
        result
    );
}

#[test]
fn test_punctuation_no_transform_in_fenced_code_block() {
    // Fenced code blocks should NOT have punctuation transformed
    let input = "~~~~ python\nprint(\"Hello...\")\n~~~~";
    let result = parse_and_serialize(input);
    // The quotes and ellipsis inside the code block should remain unchanged
    assert!(
        result.contains("print(\"Hello...\")"),
        "Content in fenced code blocks should not be transformed, got:\n{}",
        result
    );
}

#[test]
fn test_punctuation_in_heading() {
    // Punctuation in headings should also be transformed
    let input = "# \"Hello\" World";
    let result = parse_and_serialize(input);
    let expected = format!(
        "{}Hello{} World\n=============\n",
        LEFT_DOUBLE_QUOTE, RIGHT_DOUBLE_QUOTE
    );
    assert_eq!(result, expected);
}

#[test]
fn test_punctuation_in_list_item() {
    // Punctuation in list items should be transformed
    let input = " -  He said \"yes\"";
    let result = parse_and_serialize(input);
    let expected = format!(
        " -  He said {}yes{}\n",
        LEFT_DOUBLE_QUOTE, RIGHT_DOUBLE_QUOTE
    );
    assert_eq!(result, expected);
}

#[test]
fn test_punctuation_in_blockquote() {
    // Punctuation in blockquotes should be transformed
    let input = "> \"Quote inside quote\"";
    let result = parse_and_serialize(input);
    let expected = format!(
        "> {}Quote inside quote{}\n",
        LEFT_DOUBLE_QUOTE, RIGHT_DOUBLE_QUOTE
    );
    assert_eq!(result, expected);
}

#[test]
fn test_punctuation_preserve_existing_curly_quotes() {
    // Already curly quotes should be preserved
    let input = format!("Already {}curly{}", LEFT_DOUBLE_QUOTE, RIGHT_DOUBLE_QUOTE);
    let result = parse_and_serialize(&input);
    // Should not double-transform
    assert!(
        result.contains(&format!("{}curly{}", LEFT_DOUBLE_QUOTE, RIGHT_DOUBLE_QUOTE)),
        "Existing curly quotes should be preserved, got:\n{}",
        result
    );
}

#[test]
fn test_punctuation_all_transforms_combined() {
    // Test multiple punctuation transforms in one paragraph
    let input = "He said \"It's... amazing--isn't it?\"";
    let mut options = Options::default();
    options.curly_apostrophes = true;
    let result = parse_and_serialize_with_options(input, &options);

    // Should have curly double quotes
    assert!(
        result.contains(LEFT_DOUBLE_QUOTE) && result.contains(RIGHT_DOUBLE_QUOTE),
        "Should have curly double quotes, got:\n{}",
        result
    );
    // Should have ellipsis
    assert!(
        result.contains(ELLIPSIS),
        "Should have ellipsis, got:\n{}",
        result
    );
    // Should have em-dash
    assert!(
        result.contains(EM_DASH),
        "Should have em-dash, got:\n{}",
        result
    );
    // Should have curly apostrophes
    assert!(
        result.contains(RIGHT_SINGLE_QUOTE),
        "Should have curly apostrophes, got:\n{}",
        result
    );
}

#[test]
fn test_punctuation_all_disabled() {
    let mut options = Options::default();
    options.curly_double_quotes = false;
    options.curly_single_quotes = false;
    options.curly_apostrophes = false;
    options.ellipsis = false;
    options.em_dash = crate::DashSetting::Disabled;
    options.en_dash = crate::DashSetting::Disabled;

    let input = "He said \"It's... amazing--isn't it?\"";
    let result = parse_and_serialize_with_options(input, &options);

    // Nothing should be transformed
    assert_eq!(result, "He said \"It's... amazing--isn't it?\"\n");
}

#[test]
fn test_punctuation_bracket_possessive_stays_straight() {
    // Possessive apostrophe after closing bracket in a link reference
    // should stay straight when curly_apostrophes is disabled (default)
    let input = "This package provides [Fedify]'s API.\n\n[Fedify]: https://fedify.dev/\n";
    let result = parse_and_serialize(input);
    // The apostrophe should remain straight
    assert_eq!(
        result,
        "This package provides [Fedify]'s API.\n\n[Fedify]: https://fedify.dev/\n"
    );
}

#[test]
fn test_punctuation_bracket_possessive_curly_when_enabled() {
    // Possessive apostrophe after closing bracket should become curly
    // when curly_apostrophes is enabled
    let mut options = Options::default();
    options.curly_apostrophes = true;

    let input = "This package provides [Fedify]'s API.\n\n[Fedify]: https://fedify.dev/\n";
    let result = parse_and_serialize_with_options(input, &options);
    let expected = format!(
        "This package provides [Fedify]{}s API.\n\n[Fedify]: https://fedify.dev/\n",
        RIGHT_SINGLE_QUOTE
    );
    assert_eq!(result, expected);
}

#[test]
fn test_punctuation_decade_abbreviation() {
    // '80s style decade abbreviations
    let input = "The '80s were great.";
    let result = parse_and_serialize(input);
    // The apostrophe before the decade should become right single quote
    let expected = format!("The {}80s were great.\n", RIGHT_SINGLE_QUOTE);
    assert_eq!(result, expected);
}

#[test]
fn test_punctuation_single_hyphen_em_dash_with_spaces() {
    // Single hyphen with spaces should transform when em_dash = "-"
    let mut options = Options::default();
    options.em_dash =
        crate::DashSetting::Pattern(crate::DashPattern::new("-".to_string()).unwrap());
    let input = "word - word";
    let result = parse_and_serialize_with_options(input, &options);
    let expected = format!("word {} word\n", EM_DASH);
    assert_eq!(result, expected);
}

#[test]
fn test_punctuation_single_hyphen_em_dash_without_spaces() {
    // Single hyphen without spaces should NOT transform when em_dash = "-"
    let mut options = Options::default();
    options.em_dash =
        crate::DashSetting::Pattern(crate::DashPattern::new("-".to_string()).unwrap());
    let input = "word-word";
    let result = parse_and_serialize_with_options(input, &options);
    // Hyphen should remain because it's not surrounded by spaces
    assert_eq!(result, "word-word\n");
}

#[test]
fn test_nested_list_followed_by_paragraph_no_extra_blank_line() {
    // Regression test: When a list item contains a nested list followed by a paragraph,
    // there should be exactly one blank line between them, not two.
    let input = " -  Foo bar.

     -  Baz.
     -  Qux.

    Quux.

 -  Another item.
";
    let result = parse_and_serialize(input);
    // There should be only one blank line between the nested list and "Quux."
    // (i.e., "\n\n    Quux", not "\n\n\n    Quux")
    assert!(
        !result.contains("\n\n\n"),
        "Should not have double blank lines between nested list and following paragraph.\nGot:\n{}",
        result
    );
    assert_eq!(result, input, "Output should match input exactly");
}

#[test]
fn test_heading_sentence_case_basic() {
    let input = "# Hello World";
    let mut options = Options::default();
    options.heading_sentence_case = true;
    let result = parse_and_serialize_with_options(input, &options);
    assert_eq!(result, "Hello world\n===========\n");
}

#[test]
fn test_heading_sentence_case_with_acronyms() {
    let input = "# Working With HTTP APIs";
    let mut options = Options::default();
    options.heading_sentence_case = true;
    let result = parse_and_serialize_with_options(input, &options);
    assert_eq!(result, "Working with HTTP APIs\n======================\n");
}

#[test]
fn test_heading_sentence_case_with_proper_nouns() {
    let input = "# Introduction To JavaScript";
    let mut options = Options::default();
    options.heading_sentence_case = true;
    let result = parse_and_serialize_with_options(input, &options);
    assert_eq!(
        result,
        "Introduction to JavaScript\n==========================\n"
    );
}

#[test]
fn test_heading_sentence_case_with_user_proper_nouns() {
    let input = "# Getting Started With MyAPI";
    let mut options = Options::default();
    options.heading_sentence_case = true;
    options.heading_proper_nouns = vec!["MyAPI".to_string()];
    let result = parse_and_serialize_with_options(input, &options);
    assert_eq!(
        result,
        "Getting started with MyAPI\n==========================\n"
    );
}

#[test]
fn test_heading_sentence_case_with_code_spans() {
    let input = "# Using `MyClass` In Your Code";
    let mut options = Options::default();
    options.heading_sentence_case = true;
    let result = parse_and_serialize_with_options(input, &options);
    assert_eq!(
        result,
        "Using `MyClass` in your code\n============================\n"
    );
}

#[test]
fn test_heading_sentence_case_disabled() {
    let input = "# Hello World";
    let options = Options::default();
    let result = parse_and_serialize_with_options(input, &options);
    assert_eq!(result, "Hello World\n===========\n");
}

#[test]
fn test_heading_sentence_case_atx_style() {
    let input = "### Working With APIs";
    let mut options = Options::default();
    options.heading_sentence_case = true;
    let result = parse_and_serialize_with_options(input, &options);
    assert_eq!(result, "### Working with APIs\n");
}

#[test]
fn test_heading_sentence_case_with_quotes() {
    let input = "# Smart Suggestion: \"Did You Mean?\"";
    let mut options = Options::default();
    options.heading_sentence_case = true;
    let result = parse_and_serialize_with_options(input, &options);
    assert_eq!(
        result,
        "Smart suggestion: \u{201C}Did you mean?\u{201D}\n=================================\n"
    );
}

#[test]
fn test_heading_sentence_case_non_latin() {
    let input = "# \u{D55C}\u{AE00} \u{C81C}\u{BAA9} With English";
    let mut options = Options::default();
    options.heading_sentence_case = true;
    let result = parse_and_serialize_with_options(input, &options);
    assert_eq!(
        result,
        "\u{D55C}\u{AE00} \u{C81C}\u{BAA9} with English\n======================\n"
    );
}

#[test]
fn test_heading_sentence_case_starting_with_code_span() {
    // Regression test: when a heading starts with a code span, the word
    // following the code span should NOT be capitalized (the code span itself
    // counts as the first word).
    let input = "# `Foo` object";
    let mut options = Options::default();
    options.heading_sentence_case = true;
    let result = parse_and_serialize_with_options(input, &options);
    assert_eq!(result, "`Foo` object\n============\n");
}

#[test]
fn test_heading_sentence_case_proper_noun_in_parentheses() {
    // Regression test: proper nouns inside parentheses should be preserved.
    // Even though "Deno" is in the built-in proper nouns list, it was being
    // lowercased because find_proper_noun() didn't strip leading punctuation.
    let input = "# Test (Deno only)";
    let mut options = Options::default();
    options.heading_sentence_case = true;
    let result = parse_and_serialize_with_options(input, &options);
    assert_eq!(result, "Test (Deno only)\n================\n");
}

#[test]
fn test_heading_sentence_case_preserves_explicit_anchor_name() {
    // Regression test: trailing explicit anchor names should not be modified
    // by sentence-case conversion. Only the visible heading text should change.
    // With default heading_anchor_align = 0, anchor is right-aligned to line_width (80).
    let input = "## Test Section {#myAPI}";
    let mut options = Options::default();
    options.heading_sentence_case = true;
    let result = parse_and_serialize_with_options(input, &options);
    // "Test section" (12) + 60 spaces + "{#myAPI}" (8) = 80 chars; underline = 80 '-'
    assert_eq!(
        result,
        "Test section                                                            {#myAPI}\n\
         --------------------------------------------------------------------------------\n"
    );
}

// ============================================================================
// Heading anchor alignment tests
// ============================================================================

#[test]
fn test_heading_anchor_align_positive_gap_1() {
    let input = "# H1 {#h1}";
    let mut options = Options::default();
    options.heading_anchor_align = 1;
    let result = parse_and_serialize_with_options(input, &options);
    // "H1" (2) + 1 space + "{#h1}" (5) = 8 chars; underline = 8 '='
    assert_eq!(result, "H1 {#h1}\n========\n");
}

#[test]
fn test_heading_anchor_align_positive_gap_5() {
    let input = "## H2 {#h2}";
    let mut options = Options::default();
    options.heading_anchor_align = 5;
    let result = parse_and_serialize_with_options(input, &options);
    // "H2" (2) + 5 spaces + "{#h2}" (5) = 12 chars; underline = 12 '-'
    assert_eq!(result, "H2     {#h2}\n------------\n");
}

#[test]
fn test_heading_anchor_align_zero_right_aligns_setext_h1() {
    let input = "# Title {#t}";
    let mut options = Options::default();
    options.heading_anchor_align = 0;
    // line_width default = 80
    let result = parse_and_serialize_with_options(input, &options);
    // "Title" (5) + 71 spaces + "{#t}" (4) = 80 chars; underline = 80 '='
    assert_eq!(
        result,
        "Title                                                                       {#t}\n\
         ================================================================================\n"
    );
}

#[test]
fn test_heading_anchor_align_negative_right_aligns_shorter() {
    let input = "## Section {#s}";
    let mut options = Options::default();
    options.heading_anchor_align = -5;
    // line_width default = 80; target = 80 - 5 = 75
    let result = parse_and_serialize_with_options(input, &options);
    // "Section" (7) + 64 spaces + "{#s}" (4) = 75 chars; underline = 75 '-'
    assert_eq!(
        result,
        "Section                                                                {#s}\n\
         ---------------------------------------------------------------------------\n"
    );
}

#[test]
fn test_heading_anchor_align_zero_right_aligns_atx() {
    let input = "### Sub {#sub}";
    let mut options = Options::default();
    options.heading_anchor_align = 0;
    // line_width default = 80; ATX prefix "### " = 4 chars
    // "Sub" (3) + 67 spaces + "{#sub}" (6) = 76 chars; total line 4 + 76 = 80
    let result = parse_and_serialize_with_options(input, &options);
    assert_eq!(
        result,
        "### Sub                                                                   {#sub}\n"
    );
}

#[test]
fn test_heading_anchor_align_zero_no_line_width_fallback() {
    let input = "# Title {#t}";
    let mut options = Options::default();
    options.heading_anchor_align = 0;
    options.line_width = None;
    let result = parse_and_serialize_with_options(input, &options);
    // line_width = None → fall back to 1 space
    assert_eq!(result, "Title {#t}\n==========\n");
}

#[test]
fn test_heading_anchor_align_too_wide_fallback() {
    // Body so wide that it cannot fit at the target column: fall back to 1 space.
    // "A" * 77 + anchor "{#x}" (4) → min width = 77 + 1 + 4 = 82 > 80
    let body = "A".repeat(77);
    let input = format!("# {} {{#x}}", body);
    let mut options = Options::default();
    options.heading_anchor_align = 0; // line_width = 80
    let result = parse_and_serialize_with_options(&input, &options);
    // available = 80 - 77 - 4 = -1 → max(-1, 1) = 1 space
    let expected_heading = format!("{} {{#x}}", body); // 77 + 1 + 4 = 82 chars
    let underline = "=".repeat(expected_heading.len());
    assert_eq!(result, format!("{}\n{}\n", expected_heading, underline));
}

#[test]
fn test_heading_anchor_align_no_effect_without_anchor() {
    // Gap option must have no effect on headings that carry no explicit anchor.
    let input = "# Plain Heading";
    let mut options = Options::default();
    options.heading_anchor_align = 5;
    let result = parse_and_serialize_with_options(input, &options);
    assert_eq!(result, "Plain Heading\n=============\n");
}

#[test]
fn test_heading_anchor_align_idempotent() {
    // Format with gap=0 and line_width=80, then re-format: output must be identical.
    let input = "# Hello {#hello}";
    let options = Options::default(); // heading_anchor_align = 0, line_width = Some(80)
    let first = parse_and_serialize_with_options(input, &options);
    let second = parse_and_serialize_with_options(&first, &options);
    assert_eq!(first, second);
}

// ============================================================================
// Code block formatter tests
// ============================================================================

fn parse_and_serialize_with_options_and_warnings(
    input: &str,
    format_options: &Options,
) -> SerializeResult {
    let arena = Arena::new();
    let options = comrak_options();
    let root = parse_document(&arena, input, &options);
    serialize_with_source_and_warnings(root, format_options, Some(input))
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_code_block_with_formatter_success() {
    use crate::CodeFormatter;

    // Use 'cat' as a simple formatter that returns input unchanged
    let mut options = Options::default();
    options.code_formatters.insert(
        "text".to_string(),
        CodeFormatter {
            command: vec!["cat".to_string()],
            timeout_secs: 5,
        },
    );

    let input = "~~~~ text\nhello world\n~~~~\n";
    let result = parse_and_serialize_with_options(input, &options);
    assert_eq!(result, "~~~~ text\nhello world\n~~~~\n");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_code_block_formatter_transforms_content() {
    use crate::CodeFormatter;

    // Use 'tr' to uppercase the content
    let mut options = Options::default();
    options.code_formatters.insert(
        "upper".to_string(),
        CodeFormatter {
            command: vec!["tr".to_string(), "a-z".to_string(), "A-Z".to_string()],
            timeout_secs: 5,
        },
    );

    let input = "~~~~ upper\nhello world\n~~~~\n";
    let result = parse_and_serialize_with_options(input, &options);
    assert_eq!(result, "~~~~ upper\nHELLO WORLD\n~~~~\n");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_code_block_formatter_failure_preserves_original() {
    use crate::CodeFormatter;

    let mut options = Options::default();
    options.code_formatters.insert(
        "text".to_string(),
        CodeFormatter {
            command: vec!["false".to_string()], // always fails
            timeout_secs: 5,
        },
    );

    let input = "~~~~ text\nhello world\n~~~~\n";
    let result = parse_and_serialize_with_options_and_warnings(input, &options);
    // Original content should be preserved
    assert_eq!(result.output, "~~~~ text\nhello world\n~~~~\n");
    // Warning should be generated
    assert!(!result.warnings.is_empty());
    assert!(result.warnings[0].message.contains("failed"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_code_block_no_formatter_for_language() {
    let options = Options::default(); // no formatters configured

    let input = "~~~~ rust\nfn main() {}\n~~~~\n";
    let result = parse_and_serialize_with_options(input, &options);
    // Content should be unchanged
    assert_eq!(result, "~~~~ rust\nfn main() {}\n~~~~\n");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_code_block_formatter_exact_language_match() {
    use crate::CodeFormatter;

    let mut options = Options::default();
    options.code_formatters.insert(
        "javascript".to_string(),
        CodeFormatter {
            command: vec!["tr".to_string(), "a-z".to_string(), "A-Z".to_string()],
            timeout_secs: 5,
        },
    );

    // 'js' should NOT match 'javascript' formatter
    let input = "~~~~ js\nhello\n~~~~\n";
    let result = parse_and_serialize_with_options(input, &options);
    // Content should be unchanged because 'js' != 'javascript'
    assert_eq!(result, "~~~~ js\nhello\n~~~~\n");

    // 'javascript' should match
    let input2 = "~~~~ javascript\nhello\n~~~~\n";
    let result2 = parse_and_serialize_with_options(input2, &options);
    assert_eq!(result2, "~~~~ javascript\nHELLO\n~~~~\n");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_code_block_formatter_timeout() {
    use crate::CodeFormatter;

    let mut options = Options::default();
    options.code_formatters.insert(
        "slow".to_string(),
        CodeFormatter {
            command: vec!["sleep".to_string(), "10".to_string()],
            timeout_secs: 1,
        },
    );

    let input = "~~~~ slow\nhello\n~~~~\n";
    let result = parse_and_serialize_with_options_and_warnings(input, &options);
    // Original content should be preserved
    assert_eq!(result.output, "~~~~ slow\nhello\n~~~~\n");
    // Warning should be generated
    assert!(!result.warnings.is_empty());
    assert!(result.warnings[0].message.contains("timed out"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_code_block_formatter_with_default_language() {
    use crate::CodeFormatter;

    let mut options = Options::default();
    options.default_language = "text".to_string();
    options.code_formatters.insert(
        "text".to_string(),
        CodeFormatter {
            command: vec!["tr".to_string(), "a-z".to_string(), "A-Z".to_string()],
            timeout_secs: 5,
        },
    );

    // Code block without language should use default_language and apply formatter
    let input = "~~~~\nhello\n~~~~\n";
    let result = parse_and_serialize_with_options(input, &options);
    assert_eq!(result, "~~~~ text\nHELLO\n~~~~\n");
}

// ============================================================================
// hongdown-no-format tests
// ============================================================================

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_code_block_no_format_skips_formatter() {
    use crate::CodeFormatter;

    let mut options = Options::default();
    options.code_formatters.insert(
        "upper".to_string(),
        CodeFormatter {
            command: vec!["tr".to_string(), "a-z".to_string(), "A-Z".to_string()],
            timeout_secs: 5,
        },
    );

    // With hongdown-no-format, the formatter should be skipped
    let input = "~~~~ upper hongdown-no-format\nhello world\n~~~~\n";
    let result = parse_and_serialize_with_options(input, &options);
    // Content should NOT be uppercased
    assert_eq!(result, "~~~~ upper hongdown-no-format\nhello world\n~~~~\n");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_code_block_no_format_preserves_keyword_in_output() {
    let options = Options::default();

    let input = "~~~~ python hongdown-no-format\ndef hello(): pass\n~~~~\n";
    let result = parse_and_serialize_with_options(input, &options);
    // hongdown-no-format should be preserved in output
    assert!(result.contains("hongdown-no-format"));
    assert_eq!(
        result,
        "~~~~ python hongdown-no-format\ndef hello(): pass\n~~~~\n"
    );
}

#[test]
fn test_code_block_no_format_without_formatter_configured() {
    let options = Options::default();

    // Even without a formatter, hongdown-no-format should be preserved
    let input = "~~~~ rust hongdown-no-format\nfn main() {}\n~~~~\n";
    let result = parse_and_serialize_with_options(input, &options);
    assert_eq!(result, "~~~~ rust hongdown-no-format\nfn main() {}\n~~~~\n");
}

#[test]
fn test_code_block_no_format_idempotent() {
    let options = Options::default();

    let input = "~~~~ js hongdown-no-format\nconst x = 1;\n~~~~\n";
    let result1 = parse_and_serialize_with_options(input, &options);
    let result2 = parse_and_serialize_with_options(&result1, &options);
    // Formatting should be idempotent
    assert_eq!(result1, result2);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_code_block_no_format_in_list_item() {
    use crate::CodeFormatter;

    let mut options = Options::default();
    options.code_formatters.insert(
        "upper".to_string(),
        CodeFormatter {
            command: vec!["tr".to_string(), "a-z".to_string(), "A-Z".to_string()],
            timeout_secs: 5,
        },
    );

    let input = " -  Item:\n\n    ~~~~ upper hongdown-no-format\n    hello\n    ~~~~\n";
    let result = parse_and_serialize_with_options(input, &options);
    // Content should NOT be uppercased
    assert!(result.contains("hello"));
    assert!(!result.contains("HELLO"));
    assert!(result.contains("hongdown-no-format"));
}

#[test]
fn test_code_block_no_format_multiple_metadata() {
    let options = Options::default();

    // hongdown-no-format can appear with other metadata
    let input = "~~~~ python some-other-info hongdown-no-format\ncode\n~~~~\n";
    let result = parse_and_serialize_with_options(input, &options);
    assert!(result.contains("hongdown-no-format"));
    assert!(result.contains("some-other-info"));
}

// =============================================================================
// Regression tests for hongdown-disable directive with footnotes/references
// =============================================================================

#[test]
fn test_footnote_definition_before_hongdown_disable() {
    // Footnote definition should NOT move below the hongdown-disable directive.
    // The footnote definition should be flushed BEFORE the disable directive.
    // See: https://github.com/dahlia/hongdown/issues/XXX
    let input = r#"Blah blah blah blah.[^1]

[^1]: This is a footnote.

<!-- hongdown-disable -->

### Foo bar

Some content here.

<!-- hongdown-enable -->
"#;
    let result = parse_and_serialize_with_source(input);

    // The footnote definition should appear BEFORE the disable directive
    let footnote_pos = result.find("[^1]:").expect("footnote definition not found");
    let disable_pos = result
        .find("<!-- hongdown-disable -->")
        .expect("disable directive not found");

    assert!(
        footnote_pos < disable_pos,
        "Footnote definition should appear before hongdown-disable directive.\nGot:\n{}",
        result
    );
}

#[test]
fn test_reference_definition_before_hongdown_disable() {
    // Reference link definition should NOT move below the hongdown-disable directive.
    // The reference definition should be flushed BEFORE the disable directive.
    let input = r#"Blah blah blah blah.[Example]

[Example]: https://example.com/

<!-- hongdown-disable -->

### Foo bar

Some content here.

<!-- hongdown-enable -->
"#;
    let result = parse_and_serialize_with_source(input);

    // The reference definition should appear BEFORE the disable directive
    let reference_pos = result
        .find("[Example]:")
        .expect("reference definition not found");
    let disable_pos = result
        .find("<!-- hongdown-disable -->")
        .expect("disable directive not found");

    assert!(
        reference_pos < disable_pos,
        "Reference definition should appear before hongdown-disable directive.\nGot:\n{}",
        result
    );
}

#[test]
fn test_multiple_footnotes_and_references_before_hongdown_disable() {
    // Multiple footnote and reference definitions should all appear before disable directive.
    let input = r#"First paragraph with [link1] and footnote[^1].

Second paragraph with [link2] and footnote[^2].

[link1]: https://link1.example.com/
[^1]: First footnote.
[link2]: https://link2.example.com/
[^2]: Second footnote.

<!-- hongdown-disable -->

### Disabled section

Some content here.

<!-- hongdown-enable -->
"#;
    let result = parse_and_serialize_with_source(input);

    let disable_pos = result
        .find("<!-- hongdown-disable -->")
        .expect("disable directive not found");

    // All definitions should appear before the disable directive
    let footnote1_pos = result.find("[^1]:").expect("footnote 1 not found");
    let footnote2_pos = result.find("[^2]:").expect("footnote 2 not found");
    let link1_pos = result.find("[link1]:").expect("link1 not found");
    let link2_pos = result.find("[link2]:").expect("link2 not found");

    assert!(
        footnote1_pos < disable_pos,
        "Footnote 1 should appear before hongdown-disable.\nGot:\n{}",
        result
    );
    assert!(
        footnote2_pos < disable_pos,
        "Footnote 2 should appear before hongdown-disable.\nGot:\n{}",
        result
    );
    assert!(
        link1_pos < disable_pos,
        "Link 1 reference should appear before hongdown-disable.\nGot:\n{}",
        result
    );
    assert!(
        link2_pos < disable_pos,
        "Link 2 reference should appear before hongdown-disable.\nGot:\n{}",
        result
    );
}

#[test]
fn test_footnote_definition_before_hongdown_disable_file() {
    // Footnote definition should NOT move below the hongdown-disable-file directive.
    let input = r#"Blah blah blah blah.[^1]

[^1]: This is a footnote.

<!-- hongdown-disable-file -->

### Foo bar

Some content here.
"#;
    let result = parse_and_serialize_with_source(input);

    // The footnote definition should appear BEFORE the disable-file directive
    let footnote_pos = result.find("[^1]:").expect("footnote definition not found");
    let disable_pos = result
        .find("<!-- hongdown-disable-file -->")
        .expect("disable-file directive not found");

    assert!(
        footnote_pos < disable_pos,
        "Footnote definition should appear before hongdown-disable-file directive.\nGot:\n{}",
        result
    );
}

#[test]
fn test_footnote_definition_before_hongdown_disable_next_line() {
    // Footnote definition should NOT move below the hongdown-disable-next-line directive.
    let input = r#"Blah blah blah blah.[^1]

[^1]: This is a footnote.

<!-- hongdown-disable-next-line -->
### Foo bar

Some content here.
"#;
    let result = parse_and_serialize_with_source(input);

    // The footnote definition should appear BEFORE the disable-next-line directive
    let footnote_pos = result.find("[^1]:").expect("footnote definition not found");
    let disable_pos = result
        .find("<!-- hongdown-disable-next-line -->")
        .expect("disable-next-line directive not found");

    assert!(
        footnote_pos < disable_pos,
        "Footnote definition should appear before hongdown-disable-next-line directive.\nGot:\n{}",
        result
    );
}

#[test]
fn test_footnote_definition_before_hongdown_disable_next_section() {
    // Footnote definition should NOT move below the hongdown-disable-next-section directive.
    let input = r#"Blah blah blah blah.[^1]

[^1]: This is a footnote.

<!-- hongdown-disable-next-section -->

## Foo bar

Some content here.
"#;
    let result = parse_and_serialize_with_source(input);

    // The footnote definition should appear BEFORE the disable-next-section directive
    let footnote_pos = result.find("[^1]:").expect("footnote definition not found");
    let disable_pos = result
        .find("<!-- hongdown-disable-next-section -->")
        .expect("disable-next-section directive not found");

    assert!(
        footnote_pos < disable_pos,
        "Footnote definition should appear before hongdown-disable-next-section directive.\nGot:\n{}",
        result
    );
}

// =============================================================================
// Regression tests for definition list with list as first child (idempotency)
// =============================================================================

#[test]
fn test_definition_list_with_unordered_list_first_child_idempotent() {
    // Definition list with an unordered list as the first child should be idempotent.
    // The formatter must produce output that parses back to the same structure.
    // Bug: Previously, the formatter output :\n followed by indented list, which
    // broke the definition list structure on subsequent formatting passes.
    let input = "Pros\n:    -  First item\n     -  Second item\n";
    let first_pass = parse_and_serialize(input);

    // Format again - should be identical (idempotent)
    let second_pass = parse_and_serialize(&first_pass);
    assert_eq!(
        first_pass, second_pass,
        "Formatting should be idempotent.\nFirst pass:\n{}\nSecond pass:\n{}",
        first_pass, second_pass
    );

    // Format a third time to ensure stability
    let third_pass = parse_and_serialize(&second_pass);
    assert_eq!(
        second_pass, third_pass,
        "Formatting should be idempotent.\nSecond pass:\n{}\nThird pass:\n{}",
        second_pass, third_pass
    );
}

#[test]
fn test_definition_list_with_ordered_list_first_child_idempotent() {
    // Definition list with an ordered list as the first child should be idempotent.
    let input = "Steps\n:    1.  First step\n     2.  Second step\n";
    let first_pass = parse_and_serialize(input);

    // Format again - should be identical (idempotent)
    let second_pass = parse_and_serialize(&first_pass);
    assert_eq!(
        first_pass, second_pass,
        "Formatting should be idempotent.\nFirst pass:\n{}\nSecond pass:\n{}",
        first_pass, second_pass
    );
}

#[test]
fn test_definition_list_multiple_items_with_lists_idempotent() {
    // Multiple definition list items, each with a list as first child.
    let input = r#"Pros
:    -  The actor URI is more predictable.

Cons
:    -  Changing the WebFinger username may break the existing network.
"#;
    let first_pass = parse_and_serialize(input);
    let second_pass = parse_and_serialize(&first_pass);

    assert_eq!(
        first_pass, second_pass,
        "Formatting should be idempotent.\nFirst pass:\n{}\nSecond pass:\n{}",
        first_pass, second_pass
    );
}

#[test]
fn test_definition_list_with_list_continuation_line_indentation() {
    // List item continuation lines inside definition details should align with the first line.
    // The `:    -  ` prefix is 8 characters (`:` + 4 spaces + `-` + 2 spaces),
    // so continuation lines should be indented 8 spaces (not 9).
    let input = r#"Pros
:    -  The actor URI is more predictable and human-readable,
        which makes debugging easier.

Cons
:    -  Changing the WebFinger username may break the existing network.
        Hence, the fediverse handle is immutable in practice.
"#;
    let result = parse_and_serialize(input);

    // The continuation line should have exactly 8 spaces indentation
    assert!(
        result.contains(":    -  The actor URI is more predictable and human-readable,\n        which makes debugging easier."),
        "Continuation line should be indented with 8 spaces to align with first line content, got:\n{}",
        result
    );
    assert!(
        result.contains(":    -  Changing the WebFinger username may break the existing network.\n        Hence, the fediverse handle is immutable in practice."),
        "Continuation line should be indented with 8 spaces to align with first line content, got:\n{}",
        result
    );

    // Verify idempotency
    let second_pass = parse_and_serialize(&result);
    assert_eq!(
        result, second_pass,
        "Formatting should be idempotent.\nFirst pass:\n{}\nSecond pass:\n{}",
        result, second_pass
    );
}

#[test]
fn test_definition_list_with_nested_ordered_list_continuation_indentation() {
    let input = "\
Term
:    -  Outer.

         1)  First line
             second line
";
    let result = parse_and_serialize(input);

    assert_eq!(
        result, input,
        "Nested ordered list continuation should keep its indentation.\nExpected:\n{}\nGot:\n{}",
        input, result
    );
}

#[test]
fn test_possessive_apostrophe_after_digit_stays_straight() {
    // Possessive apostrophe after a digit (like version number) should stay straight
    // when curly_apostrophes is disabled (default)
    let input = "Version 1.2.3's highlight.";
    let result = parse_and_serialize(input);
    assert_eq!(
        result, "Version 1.2.3's highlight.\n",
        "Apostrophe after digit should remain straight, got:\n{}",
        result
    );
}

#[test]
fn test_possessive_apostrophe_after_digit_curly_when_enabled() {
    // Possessive apostrophe after a digit should become curly
    // when curly_apostrophes is enabled
    let mut options = Options::default();
    options.curly_apostrophes = true;
    let input = "Version 1.2.3's highlight.";
    let result = parse_and_serialize_with_options(input, &options);
    let expected = format!("Version 1.2.3{}s highlight.\n", RIGHT_SINGLE_QUOTE);
    assert_eq!(
        result, expected,
        "Apostrophe after digit should become curly when enabled, got:\n{}",
        result
    );
}

#[test]
fn test_serialize_windows_path_in_italic() {
    let input = r"*C:\Users\Alice\Documents*";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(result, "*C:\\\\Users\\\\Alice\\\\Documents*\n");
}

#[test]
fn test_serialize_windows_path_idempotent() {
    let input = r"*C:\Users\Alice\Documents*";
    let first_pass = parse_and_serialize_with_source(input);
    let second_pass = parse_and_serialize_with_source(&first_pass.trim_end());
    let third_pass = parse_and_serialize_with_source(&second_pass.trim_end());

    assert_eq!(
        first_pass, second_pass,
        "First and second pass should match"
    );
    assert_eq!(
        second_pass, third_pass,
        "Second and third pass should match"
    );
}

#[test]
fn test_serialize_mixed_escapes_in_italic() {
    // Mix of path backslashes and Markdown escapes
    let input = r"*node\_modules at C:\Program Files*";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(result, "*node\\_modules at C:\\\\Program Files*\n");
}

#[test]
fn test_serialize_multiple_backslashes() {
    let input = r"*path\\to\\file*";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(result, "*path\\\\to\\\\file*\n");

    // Test idempotency
    let second_pass = parse_and_serialize_with_source(&result.trim_end());
    assert_eq!(result, second_pass);
}

#[test]
fn test_serialize_backslash_before_space() {
    // Test backslash followed by space (not escaping the closing marker)
    let input = r"*C:\Program Files\ folder*";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(result, "*C:\\\\Program Files\\\\ folder*\n");
}

#[test]
fn test_serialize_windows_path_in_strong() {
    let input = r"**C:\Users**";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(result, "**C:\\\\Users**\n");
}

#[test]
fn test_serialize_windows_path_plain_text() {
    let input = r"C:\Users\Alice\Documents";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(result, "C:\\\\Users\\\\Alice\\\\Documents\n");
}

// Regression test: code blocks inside footnotes should be preserved
#[test]
fn test_footnote_with_code_block() {
    let input = r#"Testing a footnote with a code block.[^1]

[^1]: Here is a code block inside a footnote:

      ~~~~ python
      def hello_world():
          print("Hello, world!")
      ~~~~
"#;
    let result = parse_and_serialize_with_footnotes(input);

    // The code block should be preserved in the output
    assert!(
        result.contains("def hello_world():"),
        "Code block content should be preserved in footnote.\nGot:\n{}",
        result
    );
    assert!(
        result.contains("~~~~"),
        "Code fence should be preserved in footnote.\nGot:\n{}",
        result
    );
    // The blank line between text and code block should be preserved
    assert!(
        result.contains("footnote:\n\n"),
        "Blank line between text and code block should be preserved.\nGot:\n{}",
        result
    );
}

#[test]
fn test_footnote_with_blockquote() {
    let input = r#"Text with footnote.[^1]

[^1]: Footnote with a blockquote:

      > This is a quote
      > in the footnote.
"#;
    let result = parse_and_serialize_with_footnotes(input);

    assert!(
        result.contains("> This is a quote"),
        "Blockquote should be preserved in footnote.\nGot:\n{}",
        result
    );
    assert!(
        result.contains("> in the footnote."),
        "Multiline blockquote should preserve line breaks.\nGot:\n{}",
        result
    );
}

#[test]
fn test_footnote_with_unordered_list() {
    let input = r#"Text with footnote.[^1]

[^1]: Footnote with a list:

       -  First item
       -  Second item
       -  Third item
"#;
    let result = parse_and_serialize_with_footnotes(input);

    assert!(
        result.contains(" -  First item"),
        "List should be preserved in footnote.\nGot:\n{}",
        result
    );
    assert!(
        result.contains(" -  Second item"),
        "All list items should be preserved.\nGot:\n{}",
        result
    );
}

#[test]
fn test_footnote_with_ordered_list() {
    let input = r#"Text with footnote.[^1]

[^1]: Footnote with an ordered list:

      1.  First item
      2.  Second item
      3.  Third item
"#;
    let result = parse_and_serialize_with_footnotes(input);

    assert!(
        result.contains("1.  First item"),
        "Ordered list should be preserved in footnote.\nGot:\n{}",
        result
    );
    assert!(
        result.contains("2.  Second item"),
        "All ordered list items should be preserved.\nGot:\n{}",
        result
    );
}

#[test]
fn test_footnote_with_multiple_paragraphs_separated() {
    let input = r#"Text with footnote.[^1]

[^1]: First paragraph in footnote.

      Second paragraph here.

      Third paragraph here.
"#;
    let result = parse_and_serialize_with_footnotes(input);

    assert!(
        result.contains("First paragraph in footnote."),
        "First paragraph should be preserved.\nGot:\n{}",
        result
    );
    assert!(
        result.contains("Second paragraph here."),
        "Second paragraph should be preserved.\nGot:\n{}",
        result
    );
    assert!(
        result.contains("Third paragraph here."),
        "Third paragraph should be preserved.\nGot:\n{}",
        result
    );
    // Check that paragraphs are separated by blank lines
    assert!(
        result.contains("footnote.\n\n"),
        "Blank line should separate paragraphs.\nGot:\n{}",
        result
    );
}

#[test]
fn test_footnote_with_mixed_blocks() {
    let input = r#"Text with footnote.[^1]

[^1]: First paragraph.

      > A blockquote.

      ~~~~ python
      print("code")
      ~~~~

       -  A list item
"#;
    let result = parse_and_serialize_with_footnotes(input);

    assert!(
        result.contains("First paragraph."),
        "Paragraph should be preserved.\nGot:\n{}",
        result
    );
    assert!(
        result.contains("> A blockquote."),
        "Blockquote should be preserved.\nGot:\n{}",
        result
    );
    assert!(
        result.contains("print(\"code\")"),
        "Code block should be preserved.\nGot:\n{}",
        result
    );
    assert!(
        result.contains(" -  A list item"),
        "List should be preserved.\nGot:\n{}",
        result
    );
}

// Regression tests for footnote wrapping with custom line_width
// Bug: footnotes were hardcoded to wrap at 80 chars instead of respecting line_width option

#[test]
fn test_footnote_respects_custom_line_width_simple() {
    let options = Options {
        line_width: Some(crate::LineWidth::new(60).unwrap()),
        ..Options::default()
    };
    let input = r#"Text[^1].

[^1]: This is a very long footnote definition that exceeds sixty characters and should be wrapped according to the custom line width setting.
"#;
    let result = parse_and_serialize_with_options(input, &options);

    // All lines should respect the 60-char limit
    for line in result.lines() {
        assert!(
            line.len() <= 60,
            "Line exceeds 60 characters: '{}' (len={})",
            line,
            line.len()
        );
    }

    // Footnote content should be preserved
    assert!(
        result.contains("This is a very long footnote"),
        "Footnote content should be preserved, got:\n{}",
        result
    );
}

#[test]
fn test_footnote_respects_custom_line_width_with_long_paragraph() {
    let options = Options {
        line_width: Some(crate::LineWidth::new(60).unwrap()),
        ..Options::default()
    };
    let input = r#"Text[^1].

[^1]: First paragraph.

      This is a very long second paragraph in the footnote that should be wrapped according to the custom line width of sixty characters for proper formatting.
"#;
    let result = parse_and_serialize_with_footnotes_and_options(input, &options);

    // For footnotes with block elements, the continuation indent ("[^1]: " = 6 chars)
    // plus the block indent (6 spaces) means content starts at column 12.
    // So the actual content should wrap at 60 - 12 = 48 chars per line.
    // We verify that no line of actual content (excluding indentation) exceeds reasonable bounds.

    // Content should be preserved
    assert!(
        result.contains("First paragraph."),
        "First paragraph should be preserved, got:\n{}",
        result
    );
    assert!(
        result.contains("This is a very long second paragraph"),
        "Second paragraph should be preserved, got:\n{}",
        result
    );

    // Check that text is wrapped (not all on one line)
    let lines: Vec<&str> = result.lines().collect();
    let long_lines: Vec<&str> = lines
        .iter()
        .filter(|line| line.contains("This is a very long"))
        .copied()
        .collect();

    // The long paragraph should be split across multiple lines
    assert!(
        long_lines.len() < 2,
        "Long paragraph should not be all on one line, got:\n{}",
        result
    );
}

#[test]
fn test_footnote_respects_custom_line_width_with_list() {
    let options = Options {
        line_width: Some(crate::LineWidth::new(60).unwrap()),
        ..Options::default()
    };
    let input = r#"Text[^1].

[^1]: A list in footnote:

       -  This is a very long list item that exceeds sixty characters and should wrap properly with continuation indent.
       -  Second item also very long to test wrapping behavior for list items inside footnotes.
"#;
    let result = parse_and_serialize_with_footnotes_and_options(input, &options);

    // List content should be preserved
    assert!(
        result.contains("This is a very long list item"),
        "List item content should be preserved, got:\n{}",
        result
    );
    assert!(
        result.contains("Second item also very long"),
        "Second list item should be preserved, got:\n{}",
        result
    );

    // Check that long list items are wrapped (split across multiple lines)
    let lines: Vec<&str> = result.lines().collect();
    let first_item_lines: Vec<&str> = lines
        .iter()
        .filter(|line| line.contains("very long list item"))
        .copied()
        .collect();

    // Should not have the entire long text on a single line
    assert!(
        first_item_lines.is_empty() || !first_item_lines[0].contains("and should wrap"),
        "Long list item should be wrapped, got:\n{}",
        result
    );
}

#[test]
fn test_footnote_respects_custom_line_width_with_ordered_list() {
    let options = Options {
        line_width: Some(crate::LineWidth::new(60).unwrap()),
        ..Options::default()
    };
    let input = r#"Text[^1].

[^1]: An ordered list in footnote:

      1.  This is the first item with a very long description that exceeds sixty characters.
      2.  This is the second item also with long text to test wrapping for ordered lists.
"#;
    let result = parse_and_serialize_with_footnotes_and_options(input, &options);

    // Ordered list content should be preserved
    assert!(
        result.contains("This is the first item"),
        "First ordered item should be preserved, got:\n{}",
        result
    );
    assert!(
        result.contains("This is the second item"),
        "Second ordered item should be preserved, got:\n{}",
        result
    );

    // Check that long ordered list items are wrapped
    let lines: Vec<&str> = result.lines().collect();
    let first_item_lines: Vec<&str> = lines
        .iter()
        .filter(|line| line.contains("first item with"))
        .copied()
        .collect();

    // Should not have the entire long text on a single line
    assert!(
        first_item_lines.is_empty() || !first_item_lines[0].contains("exceeds sixty"),
        "Long ordered list item should be wrapped, got:\n{}",
        result
    );
}

#[test]
fn test_blockquote_with_code_fence_in_list_item_idempotent() {
    // A blockquote containing a fenced code block, nested inside a list item,
    // was not formatted idempotently: the code fence lost its list-continuation
    // indentation on the first pass, then lost its blank separator on the second
    // pass.  After formatting, the output must be stable (idempotent).
    let input = "1.  Item:\n\n    > Blockquote.\n    >\n    > ~~~~ text\n    > code\n    > ~~~~\n";
    let first = parse_and_serialize(input);
    let second = parse_and_serialize(&first);
    assert_eq!(
        first, second,
        "Formatting should be idempotent.\nFirst pass:\n{}\nSecond pass:\n{}",
        first, second
    );
}

#[test]
fn test_no_line_width_paragraph_not_wrapped() {
    let long_text = "This is a very long paragraph that would normally be wrapped at 80 columns but with no line width limit it should remain as a single continuous line without any automatic line breaks being inserted.";
    let result = parse_and_serialize_no_wrap(long_text);
    assert_eq!(result, format!("{}\n", long_text));
}

#[test]
fn test_no_line_width_multiline_input_merged() {
    // With no wrapping, soft breaks (original newlines within a paragraph)
    // should be merged into a single line
    let input = "word1\nword2\nword3";
    let result = parse_and_serialize_no_wrap(input);
    assert_eq!(result, "word1 word2 word3\n");
}

#[test]
fn test_no_line_width_hard_break_preserved() {
    // Hard breaks (two trailing spaces + newline) should still be preserved
    let input = "line one  \nline two";
    let result = parse_and_serialize_no_wrap(input);
    assert_eq!(result, "line one  \nline two\n");
}

#[test]
fn test_inline_math_preserved_with_source() {
    // TeX math between `$` must be preserved verbatim: the backslash before
    // `\text` must not be escaped to `\\text`.
    let input = "An inline TeX formula: $O(\\text{some text})$.";
    let result = parse_and_serialize_with_source(input);
    assert_eq!(result, "An inline TeX formula: $O(\\text{some text})$.\n");
}

#[test]
fn test_inline_math_preserved_without_source() {
    // Without source context, math is reconstructed from the node literal and
    // still must not be escaped.
    let input = "An inline TeX formula: $O(\\text{some text})$.";
    let result = parse_and_serialize(input);
    assert_eq!(result, "An inline TeX formula: $O(\\text{some text})$.\n");
}

#[test]
fn test_inline_math_with_spaces_not_wrapped() {
    // Math containing spaces must stay on a single line even when the
    // surrounding paragraph is wrapped.
    let input = "This is a fairly long paragraph that should wrap somewhere and \
                 it contains math $x + y + z = w$ near the very end of the line.";
    let result = parse_and_serialize_with_source(input);
    // The formula stays intact (no newline inserted inside it)…
    assert!(
        result.contains("$x + y + z = w$"),
        "formula was split across lines: {result:?}"
    );
    // …and the paragraph actually wrapped onto multiple lines within width.
    assert!(
        result.lines().count() > 1,
        "paragraph did not wrap: {result:?}"
    );
    for line in result.lines() {
        assert!(line.width() <= 80, "line exceeds 80 columns: {line:?}");
    }
}

#[test]
fn test_inline_math_not_punctuation_transformed() {
    // Punctuation transformations (em dash, curly quotes) must not touch math
    // content, even though they apply to the surrounding text.
    let input = "A -- B and the set $\\{a -- b\\}$ with \"quotes\" too.";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("$\\{a -- b\\}$"),
        "math content was transformed: {result:?}"
    );
    // Surrounding text is still transformed.
    assert!(
        result.contains(" — "),
        "surrounding em dash missing: {result:?}"
    );
}

#[test]
fn test_display_math_preserved() {
    // Display math (`$$…$$`) spanning multiple lines is preserved verbatim and
    // is not mangled by paragraph wrapping.
    let input = "Before.\n\n$$\nx = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}\n$$\n\nAfter.";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("$$\nx = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}\n$$"),
        "display math not preserved: {result:?}"
    );
}

#[test]
fn test_inline_math_in_heading_preserved() {
    // Math in heading text goes through a different collection path and must
    // also be preserved verbatim.
    let input = "## The $O(n \\log n)$ algorithm";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("$O(n \\log n)$"),
        "math in heading not preserved: {result:?}"
    );
}

#[test]
fn test_inline_math_idempotent() {
    // Formatting math twice must be stable.
    let input = "An inline TeX formula: $O(\\text{some text})$.";
    let once = parse_and_serialize_with_source(input);
    let twice = parse_and_serialize_with_source(&once);
    assert_eq!(once, twice);
}

#[test]
fn test_display_math_in_list_item_preserved() {
    // Display math nested in a list item must keep its line structure and the
    // list continuation indentation, and must be idempotent.
    let input = "1.  Solve the equation:\n\n    $$\n    x = y + z\n    $$\n";
    let once = parse_and_serialize_with_source(input);
    assert!(
        once.contains("$$\n    x = y + z\n    $$"),
        "display math in list not indented correctly: {once:?}"
    );
    let twice = parse_and_serialize_with_source(&once);
    assert_eq!(once, twice, "nested display math is not idempotent");
}

#[test]
fn test_display_math_in_blockquote_preserved() {
    let input = "> Given:\n>\n> $$\n> a^2 + b^2 = c^2\n> $$\n";
    let once = parse_and_serialize_with_source(input);
    assert!(
        once.contains("> $$\n> a^2 + b^2 = c^2\n> $$"),
        "display math in blockquote not prefixed correctly: {once:?}"
    );
    let twice = parse_and_serialize_with_source(&once);
    assert_eq!(once, twice, "blockquote display math is not idempotent");
}

#[test]
fn test_private_use_char_not_corrupted_by_math_sentinels() {
    // U+E000 (a Private Use Area codepoint, e.g. a Nerd Font glyph) is valid
    // document content and must survive formatting untouched, even alongside
    // inline math.  This guards against using PUA as an internal sentinel.
    let input = "Icon \u{E000} and math $a + b$ together.";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains('\u{E000}'),
        "private-use character was dropped: {result:?}"
    );
    assert!(result.contains("$a + b$"));
}

#[test]
fn test_code_block_in_list_item_in_blockquote() {
    // Blank lines around a fenced code block nested in a blockquoted list
    // item must keep the `>` prefix; otherwise the blockquote is split and
    // the output is not idempotent.
    // https://github.com/dahlia/hongdown/issues/24
    let input = ">  -  First item:
>
>     ~~~~ sh
>     echo hello
>     ~~~~
>
>     Trailing paragraph.
";
    let once = parse_and_serialize_with_source(input);
    assert_eq!(once, input, "already-formatted input should be unchanged");
    let twice = parse_and_serialize_with_source(&once);
    assert_eq!(once, twice, "blockquoted list code block is not idempotent");
}

#[test]
fn test_code_block_in_list_item_in_alert() {
    // Regression test for the full reproduction case from
    // https://github.com/dahlia/hongdown/issues/24
    let input = "> [!TIP]
> Here are some setup tips:
>
>  -  `SECRET_KEY` is a random string.  You can generate one with:
>
>     ~~~~ sh
>     openssl rand -hex 32
>     ~~~~
>
>  -  `INSTANCE_ACTOR_KEY` is a JWK.  You can generate one with:
>
>     ~~~~ sh
>     mise run keygen
>     ~~~~
>
>     Quote this value in `.env`.
";
    let once = parse_and_serialize_with_source(input);
    assert_eq!(once, input, "already-formatted input should be unchanged");
    let twice = parse_and_serialize_with_source(&once);
    assert_eq!(once, twice, "alert list code block is not idempotent");
}

#[test]
fn test_duplicate_link_text_different_urls_get_distinct_labels() {
    // Two links with the same text but different destinations must not share
    // a reference label; otherwise the first link silently changes target.
    let input = " -  Read [guide](https://example.com/first).\n \
                 -  Read [guide](https://example.com/second).\n";
    let result = parse_and_serialize(input);
    assert!(
        result.contains("[guide]: https://example.com/first"),
        "first destination should be preserved, got:\n{}",
        result
    );
    assert!(
        result.contains("[guide 2]: https://example.com/second"),
        "second destination should get a distinct label, got:\n{}",
        result
    );
    assert!(
        result.contains("[guide][guide 2]"),
        "colliding link should use full reference syntax, got:\n{}",
        result
    );
}

#[test]
fn test_numbered_link_label_skips_undefined_reference() {
    let input = "See [guide 2].\n\n\
                  -  Read [guide](https://example.com/first).\n\
                  -  Read [guide](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(
        result.output,
        concat!(
            "See [guide 2].\n\n",
            " -  Read [guide].\n",
            " -  Read [guide][guide 3].\n\n",
            "[guide]: https://example.com/first\n",
            "[guide 3]: https://example.com/second\n",
        )
    );
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [guide 2]"
    );

    let second = parse_and_serialize_with_warnings(&result.output);
    assert_eq!(second.output, result.output);
    assert_eq!(second.warnings.len(), 1);
    assert_eq!(
        second.warnings[0].message,
        "undefined reference link: [guide 2]"
    );
}

#[test]
fn test_numbered_link_label_preserves_emphasis_boundaries() {
    let input = "See [_foo_ 2].\n\n\
                  -  Read [_foo_](https://example.com/first).\n\
                  -  Read [_foo_](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.output.contains("[_foo_][_foo_ 3]"),
        "the unresolved emphasized label should remain occupied, got:\n{}",
        result.output
    );
    assert!(!result.output.contains("[_foo_ 2]:"));
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [_foo_ 2]"
    );

    let second = parse_and_serialize_with_warnings(&result.output);
    assert_eq!(second.output, result.output);
}

#[test]
fn test_whitespace_only_explicit_label_is_collapsed() {
    let input = "See [guide 2][ ].\n\n\
                  -  Read [guide](https://example.com/first).\n\
                  -  Read [guide](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.output.contains("[guide][guide 3]"),
        "the collapsed reference label should remain occupied, got:\n{}",
        result.output
    );
    assert!(!result.output.contains("[guide 2]:"));
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [guide 2]"
    );

    let second = parse_and_serialize_with_warnings(&result.output);
    assert_eq!(second.output, result.output);
    assert_eq!(
        second.warnings[0].message,
        "undefined reference link: [guide 2]"
    );
}

#[test]
fn test_numbered_link_label_skips_markup_in_undefined_reference() {
    let input = "See [foo *bar* 2].\n\n\
                  -  Read [foo *bar*](https://example.com/first).\n\
                  -  Read [foo *bar*](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(
        result.output,
        concat!(
            "See [foo *bar* 2].\n\n",
            " -  Read [foo *bar*].\n",
            " -  Read [foo *bar*][foo *bar* 3].\n\n",
            "[foo *bar*]: https://example.com/first\n",
            "[foo *bar* 3]: https://example.com/second\n",
        )
    );
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [foo *bar* 2]"
    );
}

#[test]
fn test_numbered_link_label_skips_soft_break_in_undefined_reference() {
    let input = "See [foo\nbar 2].\n\n\
                  -  Read [foo\nbar](https://example.com/first).\n\
                  -  Read [foo\nbar](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(
        result.output,
        concat!(
            "See [foo\nbar 2].\n\n",
            " -  Read [foo bar].\n",
            " -  Read [foo bar][foo bar 3].\n\n",
            "[foo bar]: https://example.com/first\n",
            "[foo bar 3]: https://example.com/second\n",
        )
    );
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [foo bar 2]"
    );
}

#[test]
fn test_numbered_link_label_skips_multiline_blockquote_reference() {
    let input = "> See [foo\n\
                 > bar 2].\n\n\
                  -  Read [foo\nbar](https://example.com/first).\n\
                  -  Read [foo\nbar](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.output.contains("[foo bar][foo bar 3]"),
        "blockquote prefixes must not become part of the label, got:\n{}",
        result.output
    );
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [foo bar 2]"
    );

    let second = parse_and_serialize_with_warnings(&result.output);
    assert_eq!(second.output, result.output);
    assert_eq!(
        second.warnings[0].message,
        "undefined reference link: [foo bar 2]"
    );
}

#[test]
fn test_lazy_blockquote_reference_preserves_literal_marker() {
    let input = "> See [foo\n    > bar 2].\n\n\
                  -  Read [foo > bar](https://example.com/first).\n\
                  -  Read [foo > bar](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.output.contains("[foo > bar][foo > bar 3]"),
        "a literal marker on a lazy continuation must remain in the label, got:\n{}",
        result.output
    );
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [foo > bar 2]"
    );
    assert!(!result.output.contains("[foo > bar 2]:"));
}

#[test]
fn test_list_blockquote_reference_strips_container_indent() {
    let input = " -  > See [foo\n    > bar 2].\n\n\
                  -  Read [foo bar](https://example.com/first).\n\
                  -  Read [foo bar](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.output.contains("[foo bar][foo bar 3]"),
        "list indentation should not become part of a blockquote label, got:\n{}",
        result.output
    );
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [foo bar 2]"
    );
    assert!(!result.output.contains("[foo bar 2]:"));

    let second = parse_and_serialize_with_warnings(&result.output);
    assert_eq!(second.output, result.output);
    assert_eq!(
        second.warnings[0].message,
        "undefined reference link: [foo bar 2]"
    );
}

#[test]
fn test_list_blockquote_reference_strips_tabbed_container_indent() {
    let input = " -  > See [foo\n\
                 \t> bar 2].\n\n\
                  -  Read [foo bar](https://example.com/first).\n\
                  -  Read [foo bar](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.output.contains("[foo bar][foo bar 3]"),
        "tab indentation should not become part of a blockquote label, got:\n{}",
        result.output
    );
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [foo bar 2]"
    );
    assert!(!result.output.contains("[foo bar 2]:"));

    let second = parse_and_serialize_with_warnings(&result.output);
    assert_eq!(second.output, result.output);
    assert_eq!(
        second.warnings[0].message,
        "undefined reference link: [foo bar 2]"
    );
}

#[test]
fn test_list_blockquote_lazy_reference_preserves_literal_marker() {
    let input = " -  > See [foo\n        > bar 2].\n\n\
                  -  Read [foo > bar](https://example.com/first).\n\
                  -  Read [foo > bar](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.output.contains("[foo > bar][foo > bar 3]"),
        "indentation beyond the list container should keep a literal marker, got:\n{}",
        result.output
    );
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [foo > bar 2]"
    );
    assert!(!result.output.contains("[foo > bar 2]:"));
}

#[test]
fn test_numbered_link_label_skips_hard_break_in_undefined_reference() {
    let input = "See [foo  \nbar 2].\n\n\
                  -  Read [foo\nbar](https://example.com/first).\n\
                  -  Read [foo\nbar](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.output.contains("[foo bar][foo bar 3]"),
        "the hard-break label should remain occupied, got:\n{}",
        result.output
    );
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [foo bar 2]"
    );
}

#[test]
fn test_undefined_reference_in_description_term() {
    let result = parse_and_serialize_with_warnings("[missing]\n: detail\n");
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [missing]"
    );
}

#[test]
fn test_disabled_description_term_reserves_undefined_reference() {
    let input = "<!-- hongdown-disable -->\n\n\
                 [guide 2]\n\
                 : detail\n\n\
                 <!-- hongdown-enable -->\n\n\
                  -  Read [guide](https://example.com/first).\n\
                  -  Read [guide](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.output.contains("[guide][guide 3]"),
        "the copied description term should reserve its label, got:\n{}",
        result.output
    );
    assert!(result.warnings.is_empty());
}

#[test]
fn test_multiline_description_term_reserves_undefined_reference() {
    let input = "See\n\
                 continued [guide 2]\n\
                 : detail\n\n\
                  -  Read [guide](https://example.com/first).\n\
                  -  Read [guide](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.output.contains("[guide][guide 3]"),
        "every line of the description term should reserve labels, got:\n{}",
        result.output
    );
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [guide 2]"
    );
}

#[test]
fn test_adjacent_inline_link_is_not_an_explicit_reference_label() {
    let input = "See [guide 2][bar](/bar).\n\n\
                  -  Read [guide](https://example.com/first).\n\
                  -  Read [guide](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.output.contains("[guide][guide 3]"),
        "the unresolved shortcut should reserve its own label, got:\n{}",
        result.output
    );
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [guide 2]"
    );
}

#[test]
fn test_numbered_link_label_uses_rendered_undefined_reference() {
    let input = "See [foo_bar 2].\n\n\
                  -  Read [foo_bar](https://example.com/first).\n\
                  -  Read [foo_bar](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(
        result.output,
        concat!(
            "See [foo\\_bar 2].\n\n",
            " -  Read [foo\\_bar].\n",
            " -  Read [foo\\_bar][foo\\_bar 3].\n\n",
            "[foo\\_bar]: https://example.com/first\n",
            "[foo\\_bar 3]: https://example.com/second\n",
        )
    );
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [foo\\_bar 2]"
    );
}

#[test]
fn test_numbered_link_label_skips_code_in_undefined_reference() {
    let input = "See [foo `bar` 2].\n\n\
                  -  Read [foo `bar`](https://example.com/first).\n\
                  -  Read [foo `bar`](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(
        result.output,
        concat!(
            "See [foo `bar` 2].\n\n",
            " -  Read [foo `bar`].\n",
            " -  Read [foo `bar`][foo `bar` 3].\n\n",
            "[foo `bar`]: https://example.com/first\n",
            "[foo `bar` 3]: https://example.com/second\n",
        )
    );
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [foo `bar` 2]"
    );
}

#[test]
fn test_numbered_link_label_uses_emitted_inline_spans() {
    for (source_label, emitted_label) in [
        ("foo -- bar", "foo — bar"),
        ("foo &amp;", "foo &amp;"),
        ("foo $x$", "foo $x$"),
        ("foo <i>bar</i>", "foo <i>bar</i>"),
    ] {
        let input = format!(
            "See [{source_label} 2].\n\n\
             -  Read [{source_label}](https://example.com/first).\n\
             -  Read [{source_label}](https://example.com/second).\n"
        );
        let result = parse_and_serialize_with_warnings(&input);
        assert!(
            result
                .output
                .contains(&format!("[{emitted_label}][{emitted_label} 3]")),
            "the occupied label should be skipped for {source_label:?}, got:\n{}",
            result.output
        );
        assert!(
            !result.output.contains(&format!("[{emitted_label} 2]:")),
            "the unresolved label should not be defined for {source_label:?}, got:\n{}",
            result.output
        );
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(
            result.warnings[0].message,
            format!("undefined reference link: [{emitted_label} 2]")
        );
    }
}

#[test]
fn test_cross_node_full_reference_warns_once_for_explicit_label() {
    let first = parse_and_serialize_with_warnings("[foo *bar*][baz]\n");
    assert_eq!(first.warnings.len(), 1);
    assert_eq!(first.warnings[0].message, "undefined reference link: [baz]");

    let second = parse_and_serialize_with_warnings("[foo][baz *qux*]\n");
    assert_eq!(second.warnings.len(), 1);
    assert_eq!(
        second.warnings[0].message,
        "undefined reference link: [baz *qux*]"
    );
}

#[test]
fn test_rendered_escaped_reference_is_not_reserved() {
    let input = "See \\[foo_bar 2].\n\n\
                  -  Read [foo_bar](https://example.com/first).\n\
                  -  Read [foo_bar](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(
        result.output,
        concat!(
            "See \\[foo\\_bar 2].\n\n",
            " -  Read [foo\\_bar].\n",
            " -  Read [foo\\_bar][foo\\_bar 2].\n\n",
            "[foo\\_bar]: https://example.com/first\n",
            "[foo\\_bar 2]: https://example.com/second\n",
        )
    );
    assert!(result.warnings.is_empty());
}

#[test]
fn test_rendered_source_only_definition_is_not_undefined() {
    let input = "See [foo_bar].\n\n*[X]: abbreviation\n[foo_bar]: https://example.com/\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(result.warnings.is_empty());
}

#[test]
fn test_wrapped_source_only_reference_is_not_undefined() {
    let input = "*[X]: abbreviation\n\
                 [foo bar]: https://example.com/\n\n\
                 See [foo\n\
                 bar] here.\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.warnings.is_empty(),
        "wrapped labels should match whitespace-normalized definitions: {:?}",
        result.warnings
    );
}

#[test]
fn test_abbreviation_label_reserves_numbered_reference() {
    let input = "See [guide 2].\n\n\
                 *[guide 2]: abbreviation\n\n\
                  -  Read [guide](https://example.com/first).\n\
                  -  Read [guide](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.output.contains("[guide][guide 3]"),
        "a warning-exempt abbreviation label should remain occupied, got:\n{}",
        result.output
    );
    assert!(
        !result
            .output
            .contains("[guide 2]: https://example.com/second")
    );
    assert!(result.warnings.is_empty());

    let second = parse_and_serialize_with_warnings(&result.output);
    assert_eq!(second.output, result.output);
    assert!(second.warnings.is_empty());
}

#[test]
fn test_verbatim_abbreviation_does_not_reserve_rendered_variant() {
    let input = "*[foo_bar 2]: abbreviation\n\n\
                  -  Read [foo_bar](https://example.com/first).\n\
                  -  Read [foo_bar](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.output.contains("[foo\\_bar][foo\\_bar 2]"),
        "the parser-distinct rendered variant should remain free, got:\n{}",
        result.output
    );
    assert!(!result.output.contains("[foo\\_bar 3]:"));
    assert!(result.output.contains("*[foo_bar 2]: abbreviation"));
    assert!(result.warnings.is_empty());

    let second = parse_and_serialize_with_warnings(&result.output);
    assert_eq!(second.output, result.output);
    assert!(second.warnings.is_empty());
}

#[test]
fn test_verbatim_source_definition_does_not_reserve_rendered_variant() {
    let input = "*[X]: abbreviation\n\
                 [foo_bar 2]: https://example.com/existing\n\n\
                  -  Read [foo_bar](https://example.com/first).\n\
                  -  Read [foo_bar](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.output.contains("[foo\\_bar][foo\\_bar 2]"),
        "the parser-distinct rendered variant should remain free, got:\n{}",
        result.output
    );
    assert!(!result.output.contains("[foo\\_bar 3]:"));
    assert!(
        result
            .output
            .contains("[foo_bar 2]: https://example.com/existing")
    );
    assert!(result.warnings.is_empty());

    let second = parse_and_serialize_with_warnings(&result.output);
    assert_eq!(second.output, result.output);
    assert!(second.warnings.is_empty());
}

#[test]
fn test_prose_sharing_verbatim_label_reserves_rendered_variant() {
    let input = "See [foo_bar 2].\n\n\
                 *[foo_bar 2]: abbreviation\n\n\
                  -  Read [foo_bar](https://example.com/first).\n\
                  -  Read [foo_bar](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.output.contains("[foo\\_bar][foo\\_bar 3]"),
        "the rendered prose occurrence should remain occupied, got:\n{}",
        result.output
    );
    assert!(!result.output.contains("[foo\\_bar 2]:"));
    assert!(result.output.contains("*[foo_bar 2]: abbreviation"));
    assert!(result.warnings.is_empty());

    let second = parse_and_serialize_with_warnings(&result.output);
    assert_eq!(second.output, result.output);
}

#[test]
fn test_source_only_definition_label_reserves_numbered_reference() {
    let input = "See [guide 2].\n\n\
                 *[X]: abbreviation\n\
                 [guide 2]: https://example.com/existing\n\n\
                  -  Read [guide](https://example.com/first).\n\
                  -  Read [guide](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.output.contains("[guide][guide 3]"),
        "a source-only definition label should remain occupied, got:\n{}",
        result.output
    );
    assert!(
        !result
            .output
            .contains("[guide 2]: https://example.com/second")
    );
    assert!(result.warnings.is_empty());

    let second = parse_and_serialize_with_warnings(&result.output);
    assert_eq!(second.output, result.output);
    assert!(second.warnings.is_empty());
}

#[test]
fn test_unused_source_definition_can_supply_numbered_reference() {
    let input = "[guide 2]: https://example.com/b\n\n\
                  -  Read [guide](https://example.com/a).\n\
                  -  Read [guide](https://example.com/b).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.output.contains("[guide][guide 2]"),
        "the matching source label should be reused, got:\n{}",
        result.output
    );
    assert!(!result.output.contains("[guide][guide 3]"));
    assert!(result.warnings.is_empty());

    let second = parse_and_serialize_with_warnings(&result.output);
    assert_eq!(second.output, result.output);
    assert!(second.warnings.is_empty());
}

#[test]
fn test_code_block_definition_does_not_reserve_numbered_reference() {
    for code_block in [
        "~~~~ text\n[guide 2]: example\n~~~~",
        "    [guide 2]: example",
    ] {
        let input = format!(
            "{code_block}\n\n\
             -  Read [guide](https://example.com/first).\n\
             -  Read [guide](https://example.com/second).\n"
        );
        let result = parse_and_serialize_with_warnings(&input);
        assert!(
            result.output.contains("[guide][guide 2]"),
            "a definition lookalike in code should leave label 2 free, got:\n{}",
            result.output
        );
        assert!(!result.output.contains("[guide][guide 3]"));
        assert!(result.warnings.is_empty());
    }
}

#[test]
fn test_undefined_reference_preserves_surrounding_quote_state() {
    let input = "Opening \" then [foo\"bar 2].\n\n\
                  -  Read [foo”bar](https://example.com/first).\n\
                  -  Read [foo”bar](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.output.contains("Opening “ then \\[foo”bar 2]."),
        "the unresolved label should use the paragraph's quote state, got:\n{}",
        result.output
    );
    assert!(
        result.output.contains("[foo”bar][foo”bar 3]"),
        "the emitted unresolved label should remain occupied, got:\n{}",
        result.output
    );
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [foo”bar 2]"
    );

    let second = parse_and_serialize_with_warnings(&result.output);
    assert_eq!(second.output, result.output);
    assert!(second.warnings.is_empty());
}

#[test]
fn test_undefined_reference_preserves_resolved_link_context() {
    let input = "[\"][ref] [foo\"bar 2].\n\n\
                  -  Read [foo“bar](https://example.com/first).\n\
                  -  Read [foo“bar](https://example.com/second).\n\n\
                 [ref]: /url\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.output.contains("[foo“bar][foo“bar 3]"),
        "the emitted unresolved label should remain occupied, got:\n{}",
        result.output
    );
    assert!(!result.output.contains("[foo“bar 2]:"));
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [foo“bar 2]"
    );

    let second = parse_and_serialize_with_warnings(&result.output);
    assert_eq!(second.output, result.output);
}

#[test]
fn test_heading_undefined_reference_preserves_resolved_link_context() {
    let input = "# [\"][ref] [foo\"bar 2]\n\n\
                  -  Read [foo“bar](https://example.com/first).\n\
                  -  Read [foo“bar](https://example.com/second).\n\n\
                 [ref]: /url\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.output.contains("[foo“bar][foo“bar 3]"),
        "the emitted heading label should remain occupied, got:\n{}",
        result.output
    );
    assert!(!result.output.contains("[foo“bar 2]:"));
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [foo“bar 2]"
    );

    let second = parse_and_serialize_with_warnings(&result.output);
    assert_eq!(second.output, result.output);
}

#[test]
fn test_numbered_link_label_skips_escaped_closing_bracket() {
    let input = "See [foo\\]bar 2].\n\n\
                  -  Read [foo\\]bar](https://example.com/first).\n\
                  -  Read [foo\\]bar](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(
        result.output,
        concat!(
            "See [foo\\]bar 2].\n\n",
            " -  Read [foo\\]bar].\n",
            " -  Read [foo\\]bar][foo\\]bar 3].\n\n",
            "[foo\\]bar]: https://example.com/first\n",
            "[foo\\]bar 3]: https://example.com/second\n",
        )
    );
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [foo\\]bar 2]"
    );
}

#[test]
fn test_undefined_reference_after_consumed_definition() {
    let result = parse_and_serialize_with_warnings("[a]: /url\nSee [missing].\n");
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(result.warnings[0].line, 2);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [missing]"
    );
}

#[test]
fn test_heading_undefined_reference_uses_heading_rendering() {
    let input = "# [foo <i>bar</i> 2]\n\n\
                  -  Read [foo <i>bar</i>](https://example.com/first).\n\
                  -  Read [foo <i>bar</i>](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.output.contains("[foo <i>bar</i>][foo <i>bar</i> 2]"),
        "the heading's rendered label should not occupy the HTML label, got:\n{}",
        result.output
    );
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [foo bar 2]"
    );
}

#[test]
fn test_heading_undefined_reference_ignores_brackets_in_html() {
    let input = "# See [foo <i title=\"]\">bar</i> 2]\n\n\
                  -  Read [foo bar](https://example.com/first).\n\
                  -  Read [foo bar](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.output.contains("[foo bar][foo bar 3]"),
        "the stripped heading label should remain occupied, got:\n{}",
        result.output
    );
    assert!(!result.output.contains("[foo bar 2]:"));
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [foo bar 2]"
    );

    let second = parse_and_serialize_with_warnings(&result.output);
    assert_eq!(second.output, result.output);
    assert_eq!(
        second.warnings[0].message,
        "undefined reference link: [foo bar 2]"
    );
}

#[test]
fn test_heading_undefined_reference_uses_sentence_case_rendering() {
    let input = "# See [foo \"bar\" 2]\n\n\
                  -  Read [foo “bar”](https://example.com/first).\n\
                  -  Read [foo “bar”](https://example.com/second).\n";
    let mut options = Options::default();
    options.heading_sentence_case = true;
    options.curly_double_quotes = false;

    let result = parse_and_serialize_with_warnings_and_options(input, &options);
    assert!(
        result.output.contains("[foo “bar”][foo “bar” 3]"),
        "the sentence-cased heading label should remain occupied, got:\n{}",
        result.output
    );
    assert!(
        !result.output.contains("[foo “bar” 2]:"),
        "the unresolved heading label must not be defined, got:\n{}",
        result.output
    );
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [foo “bar” 2]"
    );
}

#[test]
fn test_heading_undefined_reference_uses_noun_directives() {
    let options = Options {
        heading_sentence_case: true,
        ..Options::default()
    };
    for (directive, heading_label, link_label, emitted_label) in [
        ("hongdown-proper-nouns: MyAPI", "MyAPI", "MyAPI", "MyAPI"),
        (
            "hongdown-common-nouns: Python",
            "Python",
            "python",
            "python",
        ),
    ] {
        let input = format!(
            "<!-- {directive} -->\n\n\
             # See [{heading_label} 2]\n\n\
              -  Read [{link_label}](https://example.com/first).\n\
              -  Read [{link_label}](https://example.com/second).\n"
        );
        let result = parse_and_serialize_with_warnings_and_options(&input, &options);
        assert!(
            result.output.contains(&format!("See [{emitted_label} 2]")),
            "the heading should use the noun directive, got:\n{}",
            result.output
        );
        assert!(
            result
                .output
                .contains(&format!("[{link_label}][{link_label} 3]")),
            "the occupied label should be skipped, got:\n{}",
            result.output
        );
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(
            result.warnings[0].message,
            format!("undefined reference link: [{emitted_label} 2]")
        );
    }
}

#[test]
fn test_numbered_link_label_skips_bang_prefixed_reference() {
    let input = "See [!guide 2].\n\n\
                  -  Read [!guide](https://example.com/first).\n\
                  -  Read [!guide](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.output.contains("[!guide][!guide 3]"),
        "the unresolved bang-prefixed label should remain occupied, got:\n{}",
        result.output
    );
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [!guide 2]"
    );
}

#[test]
fn test_numbered_link_label_skips_mdx_reference() {
    let input = "See [foo {bar} 2].\n\n\
                  -  Read [foo {bar}](https://example.com/first).\n\
                  -  Read [foo {bar}](https://example.com/second).\n";
    let options = Options {
        mdx: true,
        ..Options::default()
    };
    let result = crate::format_with_warnings(input, &options).unwrap();
    assert!(
        result.output.contains("[foo {bar}][foo {bar} 3]"),
        "the restored MDX label should remain occupied, got:\n{}",
        result.output
    );
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [foo {bar} 2]"
    );
}

#[test]
fn test_copied_heading_reserves_verbatim_reference_label() {
    let input = "<!-- hongdown-disable-next-line -->\n\
                 # [foo <i>bar</i> 2]\n\n\
                  -  Read [foo <i>bar</i>](https://example.com/first).\n\
                  -  Read [foo <i>bar</i>](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.output.contains("[foo <i>bar</i>][foo <i>bar</i> 3]"),
        "the copied heading's verbatim label should remain occupied, got:\n{}",
        result.output
    );
    assert!(result.warnings.is_empty());
}

#[test]
fn test_disabled_reference_keeps_escaped_label_free() {
    let input = "<!-- hongdown-disable-next-line -->\n\
                 See [foo_bar 2].\n\n\
                  -  Read [foo_bar](https://example.com/first).\n\
                  -  Read [foo_bar](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.output.contains("[foo\\_bar][foo\\_bar 2]"),
        "parser-distinct escaped labels should remain available, got:\n{}",
        result.output
    );
    assert!(result.warnings.is_empty());

    let second = parse_and_serialize_with_warnings(&result.output);
    assert_eq!(second.output, result.output);
    assert!(second.warnings.is_empty());
}

#[test]
fn test_bracket_in_inline_code_is_not_reference_terminator() {
    let result = parse_and_serialize_with_warnings("See [foo `bar 2]` baz].\n");
    assert!(
        result.warnings.is_empty(),
        "a bracket inside code cannot terminate a reference label: {:?}",
        result.warnings
    );
}

#[test]
fn test_unresolved_reference_inside_link_text_remains_occupied() {
    let input = "[x [guide 2]](/u)\n\n\
                  -  Read [guide](https://example.com/first).\n\
                  -  Read [guide](https://example.com/second).\n";
    let result = parse_and_serialize_with_warnings(input);
    assert!(
        result.output.contains("[x [guide 2]](/u)"),
        "the outer link should retain its destination, got:\n{}",
        result.output
    );
    assert!(
        result.output.contains("[guide][guide 3]"),
        "the nested unresolved label should remain occupied, got:\n{}",
        result.output
    );
    assert!(!result.output.contains("[guide 2]:"));
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [guide 2]"
    );

    let second = parse_and_serialize_with_warnings(&result.output);
    assert_eq!(second.output, result.output);
    assert_eq!(
        second.warnings[0].message,
        "undefined reference link: [guide 2]"
    );
}

#[test]
fn test_heading_reference_marker_does_not_collide_with_source() {
    let input = format!(
        "# {} [foo 2]\n\n\
         -  Read [foo](https://example.com/first).\n\
         -  Read [foo](https://example.com/second).\n",
        '\u{e000}'
    );
    let result = parse_and_serialize_with_warnings(&input);
    assert!(
        result.output.contains("[foo][foo 3]"),
        "a source PUA character must not corrupt the occupied label, got:\n{}",
        result.output
    );
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0].message,
        "undefined reference link: [foo 2]"
    );
}

#[test]
fn test_numbered_link_label_uses_complete_container_source() {
    let escaped = "\\\\[guide 2]\n\n\
                   -  Read [guide](https://example.com/first).\n\
                   -  Read [guide](https://example.com/second).\n";
    let escaped_result = parse_and_serialize_with_warnings(escaped);
    assert!(
        escaped_result.output.contains("[guide][guide 3]"),
        "both source backslashes must be reconstructed, got:\n{}",
        escaped_result.output
    );
    assert_eq!(escaped_result.warnings.len(), 1);
    assert_eq!(
        escaped_result.warnings[0].message,
        "undefined reference link: [guide 2]"
    );

    let table = "| Value |\n\
                 | ----- |\n\
                 | [foo\\|bar 2] |\n\n\
                  -  Read [foo\\|bar](https://example.com/first).\n\
                  -  Read [foo\\|bar](https://example.com/second).\n";
    let table_result = parse_and_serialize_with_warnings(table);
    assert!(
        table_result.output.contains("[foo\\|bar][foo\\|bar 3]"),
        "the complete table-cell label must remain occupied, got:\n{}",
        table_result.output
    );
}

#[test]
fn test_undefined_reference_scan_handles_many_inline_spans() {
    let input = "[missing] *markup* ".repeat(1_000);
    let result = parse_and_serialize_with_warnings(&input);
    assert_eq!(result.warnings.len(), 1_000);
}

#[test]
fn test_duplicate_link_text_same_url_reuses_label() {
    // Identical targets should still share a single definition.
    let input = " -  Read [guide](https://example.com/).\n \
                 -  Read [guide](https://example.com/).\n";
    let result = parse_and_serialize(input);
    let count = result.matches("[guide]: https://example.com/").count();
    assert_eq!(
        count, 1,
        "identical targets should share one definition, got:\n{}",
        result
    );
    assert!(
        !result.contains("guide 2"),
        "identical targets should not be renamed, got:\n{}",
        result
    );
}

#[test]
fn test_non_ascii_edge_whitespace_keeps_reference_labels_distinct() {
    // Comrak trims only CommonMark ASCII whitespace from reference-label
    // edges, so a no-break space must remain in both the emitted link and its
    // definition.
    // https://github.com/dahlia/hongdown/issues/31
    for text in ["\u{a0}foo", "foo\u{a0}"] {
        let input = format!(
            "See [foo](https://example.com/x) and \
             [{text}](https://example.com/y) too.\n"
        );
        let expected = format!(
            "See [foo] and [{text}] too.\n\n\
             [foo]: https://example.com/x\n\
             [{text}]: https://example.com/y\n"
        );
        let result = parse_and_serialize_with_warnings(&input);
        assert_eq!(result.output, expected, "failed for {text:?}");
        assert!(result.warnings.is_empty());

        let second = parse_and_serialize_with_warnings(&result.output);
        assert_eq!(second.output, result.output);
        assert!(second.warnings.is_empty());
    }
}

#[test]
fn test_numbered_label_after_non_ascii_edge_whitespace_is_idempotent() {
    let input = "See [foo\u{a0}](https://example.com/x) and \
                 [foo\u{a0}](https://example.com/y) too.\n";
    let expected = "See [foo\u{a0}] and [foo\u{a0}][foo 2] too.\n\n\
                    [foo\u{a0}]: https://example.com/x\n\
                    [foo 2]: https://example.com/y\n";

    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(result.output, expected);
    assert!(result.warnings.is_empty());

    let second = parse_and_serialize_with_warnings(&result.output);
    assert_eq!(second.output, result.output);
    assert!(second.warnings.is_empty());
}

#[test]
fn test_generated_label_normalization_preserves_link_content() {
    for (text, label) in [(" foo ", "foo"), ("`a  b`", "`a b`")] {
        let input = format!("See [{text}](https://example.com/).\n");
        let expected = format!(
            "See [{text}][{label}].\n\n\
             [{label}]: https://example.com/\n"
        );

        let result = parse_and_serialize_with_warnings(&input);
        assert_eq!(result.output, expected, "failed for {text:?}");
        assert!(result.warnings.is_empty());

        let second = parse_and_serialize_with_warnings(&result.output);
        assert_eq!(second.output, result.output);
        assert!(second.warnings.is_empty());
    }
}

#[test]
fn test_inline_external_link_whitespace_is_normalized() {
    // https://github.com/dahlia/hongdown/issues/28
    for text in ["a  b", "a\tb"] {
        let input = format!("See [{text}](https://example.com/).\n");
        let expected = "See [a b].\n\n[a b]: https://example.com/\n";

        let result = parse_and_serialize_with_warnings(&input);
        assert_eq!(result.output, expected, "failed for {text:?}");
        assert!(result.warnings.is_empty());

        let second = parse_and_serialize_with_warnings(&result.output);
        assert_eq!(second.output, result.output);
        assert!(second.warnings.is_empty());
    }
}

#[test]
fn test_inline_external_link_edge_whitespace_is_normalized() {
    for (text, displayed) in [("  a", " a"), ("a  ", "a "), ("a\t\t", "a ")] {
        let input = format!("See [{text}](https://example.com/).\n");
        let expected = format!(
            "See [{displayed}][a].\n\n\
             [a]: https://example.com/\n"
        );

        let result = parse_and_serialize_with_warnings(&input);
        assert_eq!(result.output, expected, "failed for {text:?}");
        assert!(result.warnings.is_empty());

        let second = parse_and_serialize_with_warnings(&result.output);
        assert_eq!(second.output, result.output);
        assert!(second.warnings.is_empty());
    }
}

#[test]
fn test_non_ascii_internal_whitespace_is_preserved() {
    for whitespace in ['\u{a0}', '\u{3000}'] {
        let text = format!("a{whitespace}b");
        let input = format!("See [{text}](https://example.com/).\n");
        let expected = format!(
            "See [{text}][a b].\n\n\
             [a b]: https://example.com/\n"
        );

        let result = parse_and_serialize_with_warnings(&input);
        assert_eq!(
            result.output, expected,
            "failed for U+{:04X}",
            whitespace as u32
        );
        assert!(result.warnings.is_empty());

        let second = parse_and_serialize_with_warnings(&result.output);
        assert_eq!(second.output, result.output);
        assert!(second.warnings.is_empty());
    }
}

#[test]
fn test_mixed_inline_external_link_whitespace_is_normalized_selectively() {
    // Whitespace in ordinary text should collapse without changing the
    // contents of whitespace-sensitive inline constructs.
    for (text, displayed, label) in [
        ("a  b `c  d` e  f", "a b `c  d` e f", "a b `c d` e f"),
        ("a  b $c  d$ e  f", "a b $c  d$ e f", "a b $c d$ e f"),
        (
            "a  b <span>c  d</span> e  f",
            "a b <span>c  d</span> e f",
            "a b <span>c d</span> e f",
        ),
        (
            "<span>*a  </span>* c  d",
            "<span>*a  </span>* c d",
            "<span>*a </span>* c d",
        ),
        (
            "<span>**a  </span>** c  d",
            "<span>**a  </span>** c d",
            "<span>**a </span>** c d",
        ),
    ] {
        let input = format!("See [{text}](https://example.com/).\n");
        let expected = format!(
            "See [{displayed}][{label}].\n\n\
             [{label}]: https://example.com/\n"
        );

        let result = parse_and_serialize_with_warnings(&input);
        assert_eq!(result.output, expected, "failed for {text:?}");
        assert!(result.warnings.is_empty());

        let second = parse_and_serialize_with_warnings(&result.output);
        assert_eq!(second.output, result.output);
        assert!(second.warnings.is_empty());
    }
}

#[test]
fn test_nested_image_in_external_link_is_preserved() {
    let input = "See [**![badge](badge.svg)** text](https://example.com/).\n";

    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(result.output, input);
    assert!(result.warnings.is_empty());

    let second = parse_and_serialize_with_warnings(&result.output);
    assert_eq!(second.output, result.output);
    assert!(second.warnings.is_empty());
}

#[test]
fn test_non_ascii_edge_whitespace_preserves_inline_span_spaces() {
    for (text, label) in [
        ("`a  b`", "`a b`"),
        ("$a  b$", "$a b$"),
        ("<span>a  b</span>", "<span>a b</span>"),
    ] {
        let input = format!("See [\u{a0}{text}](https://example.com/).\n");
        let expected = format!(
            "See [\u{a0}{text}][\u{a0}{label}].\n\n\
             [\u{a0}{label}]: https://example.com/\n"
        );

        let result = parse_and_serialize_with_warnings(&input);
        assert_eq!(result.output, expected, "failed for {text:?}");
        assert!(result.warnings.is_empty());

        let second = parse_and_serialize_with_warnings(&result.output);
        assert_eq!(second.output, result.output);
        assert!(second.warnings.is_empty());
    }
}

#[test]
fn test_escaped_and_unescaped_reference_labels_remain_distinct() {
    let input = "[one][foo_bar] and [two][foo\\_bar].\n\n\
                 [foo_bar]: https://example.com/\n\
                 [foo\\_bar]: https://example.com/\n";
    let result = parse_and_serialize_with_warnings(input);
    assert_eq!(result.output, input);

    let second = parse_and_serialize_with_warnings(&result.output);
    assert_eq!(second.output, result.output);
    assert!(second.warnings.is_empty());
}

#[test]
fn test_duplicate_link_text_same_url_different_title_distinct_labels() {
    // The title is part of the link target, so differing titles collide too.
    let input = " -  Read [guide](https://example.com/ \"First\").\n \
                 -  Read [guide](https://example.com/ \"Second\").\n";
    let result = parse_and_serialize(input);
    assert!(
        result.contains("[guide]: https://example.com/ \"First\""),
        "first title should be preserved, got:\n{}",
        result
    );
    assert!(
        result.contains("[guide 2]: https://example.com/ \"Second\""),
        "second title should get a distinct label, got:\n{}",
        result
    );
}

#[test]
fn test_duplicate_link_labels_idempotent() {
    let input = " -  Read [guide](https://example.com/first).\n \
                 -  Read [guide](https://example.com/second).\n \
                 -  Read [guide](https://example.com/third).\n";
    let result = parse_and_serialize_with_source(input);
    let result2 = parse_and_serialize_with_source(&result);
    assert_eq!(
        result, result2,
        "distinct reference labels should be idempotent.\nFirst pass:\n{}\nSecond pass:\n{}",
        result, result2
    );
    assert!(
        result.contains("[guide 3]: https://example.com/third"),
        "third destination should be preserved, got:\n{}",
        result
    );
}

#[test]
fn test_link_label_collision_across_sections() {
    // Reference definitions are document-wide, so a label emitted in an
    // earlier section must not be redefined with a different target later.
    let input = "Intro\n\n\
                 Section A\n---------\n\n\
                 Read [guide](https://example.com/first).\n\n\
                 Section B\n---------\n\n\
                 Read [guide](https://example.com/second).\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("[guide]: https://example.com/first"),
        "first section's destination should be preserved, got:\n{}",
        result
    );
    assert!(
        result.contains("[guide 2]: https://example.com/second"),
        "second section's destination should not be dropped, got:\n{}",
        result
    );
}

#[test]
fn test_link_label_collision_case_insensitive() {
    // CommonMark matches reference labels case-insensitively, so labels
    // differing only in case would collide in the output.
    let input = "Read [Guide](https://example.com/first) and \
                 [guide](https://example.com/second).\n";
    let result = parse_and_serialize(input);
    assert!(
        result.contains("[Guide]: https://example.com/first"),
        "first destination should be preserved, got:\n{}",
        result
    );
    assert!(
        result.contains("[guide 2]: https://example.com/second"),
        "case-insensitive collision should get a distinct label, got:\n{}",
        result
    );
}

#[test]
fn test_case_insensitive_label_same_url_reuses_definition() {
    let input = "Read [Guide](https://example.com/) and \
                 [guide](https://example.com/).\n";
    let result = parse_and_serialize(input);
    let count = result.matches("]: https://example.com/").count();
    assert_eq!(
        count, 1,
        "labels differing only in case with the same target should share one \
         definition, got:\n{}",
        result
    );
}

#[test]
fn test_image_and_link_label_collision() {
    // A generated link label must not clobber an image's reference definition.
    let input = "See ![logo] and [logo](https://example.com/page).\n\n\
                 [logo]: https://example.com/logo.png\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("[logo]: https://example.com/logo.png"),
        "image destination should be preserved, got:\n{}",
        result
    );
    assert!(
        result.contains("[logo 2]: https://example.com/page"),
        "link destination should get a distinct label, got:\n{}",
        result
    );
}

#[test]
fn test_link_label_collision_inside_footnote() {
    let input = "Text[^1] and more[^2].\n\n\
                 [^1]: See [guide](https://example.com/first).\n\n\
                 [^2]: See [guide](https://example.com/second).\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("[guide]: https://example.com/first"),
        "first footnote's destination should be preserved, got:\n{}",
        result
    );
    assert!(
        result.contains("[guide 2]: https://example.com/second"),
        "second footnote's destination should be preserved, got:\n{}",
        result
    );
}

#[test]
fn test_link_label_collision_in_heading() {
    let input = "[guide](https://example.com/first) heading\n\
                 -----------------------------------------\n\n\
                 Read [guide](https://example.com/second).\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("[guide]: https://example.com/first"),
        "heading link's destination should be preserved, got:\n{}",
        result
    );
    assert!(
        result.contains("[guide 2]: https://example.com/second"),
        "body link's destination should be preserved, got:\n{}",
        result
    );
}

#[test]
fn test_numeric_link_label_collision() {
    // A numbered label derived from a numeric one is no longer numeric, so it
    // joins the regular references rather than the sorted numeric ones.
    let input = "See [1](https://example.com/a), [1](https://example.com/b) \
                 and [2](https://example.com/c).\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("[1]: https://example.com/a"),
        "first destination should be preserved, got:\n{}",
        result
    );
    assert!(
        result.contains("[1 2]: https://example.com/b"),
        "colliding numeric label should get a distinct label, got:\n{}",
        result
    );
    assert!(
        result.contains("[2]: https://example.com/c"),
        "unrelated numeric label should be untouched, got:\n{}",
        result
    );
    let result2 = parse_and_serialize_with_source(&result);
    assert_eq!(
        result, result2,
        "numeric label collision should be idempotent.\nFirst pass:\n{}\nSecond pass:\n{}",
        result, result2
    );
}

#[test]
fn test_link_label_collision_with_explicit_reference_label() {
    // A hand-written label and a generated one that clash must each keep
    // their own destination, whichever comes first in the document.
    let input = "See [the guide][guide] and later \
                 [guide](https://example.com/other).\n\n\
                 [guide]: https://example.com/authored\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("[guide]: https://example.com/authored"),
        "the hand-written label should keep its destination, got:\n{}",
        result
    );
    assert!(
        result.contains("[guide 2]: https://example.com/other"),
        "the generated label should get a distinct label, got:\n{}",
        result
    );
    let result2 = parse_and_serialize_with_source(&result);
    assert_eq!(result, result2, "should be idempotent, got:\n{}", result2);
}

#[test]
fn test_link_label_collision_unicode_case_folding() {
    // CommonMark folds labels with Unicode default case folding, under which
    // "Straße" and "STRASSE" are the same label; plain lowercasing is not
    // enough to notice the collision.
    let input = "See [Straße](https://example.com/a) and \
                 [STRASSE](https://example.com/b).\n";
    let result = parse_and_serialize_with_source(input);
    assert!(
        result.contains("[Straße]: https://example.com/a"),
        "first destination should be preserved, got:\n{}",
        result
    );
    assert!(
        result.contains("[STRASSE 2]: https://example.com/b"),
        "case-folded collision should get a distinct label, got:\n{}",
        result
    );
    let result2 = parse_and_serialize_with_source(&result);
    assert_eq!(result, result2, "should be idempotent, got:\n{}", result2);
}

#[test]
fn test_link_label_collision_near_max_label_length() {
    // A numbered label must stay within the parser's label length limit;
    // an over-long one is rejected on the next pass, which would turn the
    // link into literal text and lose its destination.
    let text = "x".repeat(999);
    let input =
        format!("See [{text}](https://example.com/a) and [{text}](https://example.com/b).\n");
    let result = parse_and_serialize_with_source(&input);
    assert!(
        result.contains("https://example.com/b"),
        "second destination should be preserved, got:\n{}",
        result
    );
    for line in result.lines() {
        if let Some(label_len) = line.strip_prefix('[').and_then(|l| l.find("]: ")) {
            assert!(
                label_len <= 1000,
                "reference label should stay within the parser's limit, got {} bytes",
                label_len
            );
        }
    }
    let result2 = parse_and_serialize_with_source(&result);
    assert_eq!(
        result, result2,
        "a label near the length limit should be idempotent"
    );
}

#[test]
fn test_repeated_target_reuses_its_numbered_label() {
    // A target that already has a numbered label must reuse it rather than
    // being handed a fresh one when it appears again.
    let input = " -  [t](https://example.com/a)\n \
                 -  [t](https://example.com/b)\n \
                 -  [t](https://example.com/c)\n \
                 -  [t](https://example.com/b)\n";
    let result = parse_and_serialize_with_source(input);
    let count = result.matches("]: https://example.com/b").count();
    assert_eq!(
        count, 1,
        "the repeated target should keep a single definition, got:\n{}",
        result
    );
    assert_eq!(
        result.matches("[t][t 2]").count(),
        2,
        "both links to the repeated target should use the same label, got:\n{}",
        result
    );
}

#[test]
fn test_near_limit_labels_sharing_a_prefix_stay_distinct() {
    // Labels long enough to be shortened before numbering collapse to the
    // same prefix, so their numbered variants live in one namespace and must
    // still be allocated distinctly.
    let prefix = "x".repeat(997);
    let mut input = String::new();
    for i in 0..8 {
        let text = format!("{prefix}{i:03}");
        input.push_str(&format!(
            "L [{text}](https://example.com/a{i}) and [{text}](https://example.com/b{i}).\n\n"
        ));
    }
    let result = parse_and_serialize_with_source(&input);
    for i in 0..8 {
        assert!(
            result.contains(&format!("https://example.com/a{i}\n")),
            "destination a{} should be preserved, got:\n{}",
            i,
            result
        );
        assert!(
            result.contains(&format!("https://example.com/b{i}\n")),
            "destination b{} should be preserved, got:\n{}",
            i,
            result
        );
    }
    let result2 = parse_and_serialize_with_source(&result);
    assert_eq!(
        result, result2,
        "shortened labels sharing a prefix should be idempotent"
    );
}

#[test]
fn test_numbered_variant_occupied_by_another_target_is_reused() {
    // A numbered variant that is already occupied must still be found by a
    // later link to the target it holds, rather than that target being given
    // a second, redundant definition.
    let input = "See [b][foo 2], [foo](https://example.com/y), \
                 [foo](https://example.com/z) and [foo](https://example.com/x).\n\n\
                 [foo 2]: https://example.com/x\n";
    let result = parse_and_serialize_with_source(input);
    let count = result.matches("]: https://example.com/x").count();
    assert_eq!(
        count, 1,
        "the occupied variant's target should keep a single definition, got:\n{}",
        result
    );
    assert!(
        result.contains("[foo][foo 2]"),
        "the later link should reuse the existing variant, got:\n{}",
        result
    );
    let result2 = parse_and_serialize_with_source(&result);
    assert_eq!(result, result2, "should be idempotent, got:\n{}", result2);
}
