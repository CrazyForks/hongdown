//! Integration tests for Hongdown formatter.

use hongdown::{LineWidth, Options, format};

/// Test that formatting is idempotent (formatting twice produces same result).
#[test]
fn test_idempotent_formatting() {
    let input = r#"# Title

This is a paragraph with some **bold** and *italic* text.

## Section

 -  List item one
 -  List item two

~~~~ rust
fn main() {}
~~~~
"#;

    let options = Options::default();
    let first_pass = format(input, &options).unwrap();
    let second_pass = format(&first_pass, &options).unwrap();

    assert_eq!(first_pass, second_pass, "Formatting should be idempotent");
}

/// Test formatting a complete document with various elements.
#[test]
fn test_complete_document() {
    let input = r#"# Document Title

This is the introduction paragraph.

## First Section

Here is some content with *emphasis* and **strong** text.

 -  First item
 -  Second item
 -  Third item

### Subsection

> This is a block quote.

~~~~ python
def hello():
    print("Hello!")
~~~~

## Second Section

Visit [Rust](https://www.rust-lang.org/) for more info.
"#;

    let options = Options::default();
    let result = format(input, &options).unwrap();

    // Verify key formatting rules
    assert!(result.contains("Document Title\n="));
    assert!(result.contains("First Section\n-"));
    assert!(result.contains("### Subsection"));
    assert!(result.contains(" -  First item"));
    assert!(result.contains("~~~~ python"));
}

/// Test that inline code is not broken across lines.
#[test]
fn test_inline_code_not_broken() {
    let input = "This is a paragraph with `some_very_long_function_name_that_should_not_be_broken()` inline code.";
    let options = Options {
        line_width: Some(LineWidth::new(40).unwrap()),
        ..Options::default()
    };
    let result = format(input, &options).unwrap();

    // The inline code should appear intact on some line
    assert!(
        result.contains("`some_very_long_function_name_that_should_not_be_broken()`"),
        "Inline code should not be broken"
    );
}

/// With the default options (`math = true`), inline math is preserved verbatim
/// and its backslashes are not escaped.
#[test]
fn test_math_enabled_preserves_inline_by_default() {
    let input = r"An inline TeX formula: $O(\text{some text})$.";
    let options = Options::default();
    let result = format(input, &options).unwrap();
    assert_eq!(result, "An inline TeX formula: $O(\\text{some text})$.\n");
}

/// With `math = false`, a `$` is treated as literal text again, so the
/// backslash inside the (no-longer-math) span is escaped — the previous
/// behaviour.
#[test]
fn test_math_disabled_treats_dollar_as_text() {
    let input = r"An inline TeX formula: $O(\text{some text})$.";
    let options = Options {
        math: false,
        ..Options::default()
    };
    let result = format(input, &options).unwrap();
    assert!(
        result.contains(r"\\text"),
        "expected the backslash to be escaped when math is disabled: {result:?}"
    );
}

/// Formatting a document with both inline and display math is idempotent.
#[test]
fn test_math_idempotent_inline_and_display() {
    let input = "Inline $a + b = c$ and a block:\n\n$$\nE = mc^2\n$$\n\nDone.";
    let options = Options::default();
    let first = format(input, &options).unwrap();
    let second = format(&first, &options).unwrap();
    assert_eq!(first, second, "math formatting should be idempotent");
    assert!(first.contains("$a + b = c$"));
    assert!(first.contains("$$\nE = mc^2\n$$"));
}

/// Test heading underline length matches heading text.
#[test]
fn test_heading_underline_length() {
    let input = "# Short";
    let options = Options::default();
    let result = format(input, &options).unwrap();

    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "Short");
    assert_eq!(lines[1], "=====");
    assert_eq!(lines[0].len(), lines[1].len());
}

/// Test ordered list numbering.
#[test]
fn test_ordered_list_numbering() {
    let input = "1. First\n2. Second\n3. Third";
    let options = Options::default();
    let result = format(input, &options).unwrap();

    // trailing_spaces=2, so "N.  " format
    assert!(result.contains("1.  First"));
    assert!(result.contains("2.  Second"));
    assert!(result.contains("3.  Third"));
}

/// Test empty input produces empty output.
#[test]
fn test_empty_input() {
    let result = format("", &Options::default()).unwrap();
    assert_eq!(result, "");
}

/// Test whitespace-only input.
#[test]
fn test_whitespace_only() {
    let result = format("   \n\n   ", &Options::default()).unwrap();
    // Should produce empty or minimal output
    assert!(result.trim().is_empty() || result.is_empty());
}

/// MDX mode: ESM `import`/`export` statements are preserved verbatim instead of
/// being mangled by punctuation transforms and Markdown escaping.
mod mdx_esm {
    use hongdown::{LineWidth, Options, format};

    fn mdx_options() -> Options {
        Options {
            mdx: true,
            ..Options::default()
        }
    }

    /// Without MDX mode (the default), an import's string literal is corrupted:
    /// its straight double quotes become curly quotes.  This pins MDX mode as
    /// strictly opt-in.
    #[test]
    fn esm_corrupted_when_mdx_disabled() {
        let input = "import { Chart } from \"./chart.js\";\n";
        let result = format(input, &Options::default()).unwrap();
        assert!(
            result.contains('\u{201c}') || result.contains('\u{201d}'),
            "expected curly quotes when MDX mode is off: {result:?}"
        );
        assert!(!result.contains("from \"./chart.js\""));
    }

    /// With MDX mode on, the import line is preserved byte-for-byte.
    #[test]
    fn esm_import_preserved_when_mdx_enabled() {
        let input = "import { Chart } from \"./chart.js\";\n";
        let result = format(input, &mdx_options()).unwrap();
        assert_eq!(result, "import { Chart } from \"./chart.js\";\n");
    }

    /// `export const … = { … }` with single quotes is preserved verbatim.
    #[test]
    fn esm_export_object_preserved() {
        let input = "export const meta = { author: 'Hong Minhee' };\n";
        let result = format(input, &mdx_options()).unwrap();
        assert_eq!(result, "export const meta = { author: 'Hong Minhee' };\n");
    }

    /// A multi-line import (brace list spanning lines) is preserved verbatim,
    /// including its internal newlines and indentation.
    #[test]
    fn esm_multiline_import_preserved() {
        let input = "import {\n  Chart,\n  Tabs,\n} from \"./ui.js\";\n";
        let result = format(input, &mdx_options()).unwrap();
        assert_eq!(result, input);
    }

    /// Surrounding prose is still formatted (wrapped at the line width) while the
    /// import passes through untouched.
    #[test]
    fn esm_prose_still_wraps() {
        let input = "import { Chart } from \"./chart.js\";\n\n\
            This is a fairly long paragraph of prose that comfortably exceeds the \
            eighty column width and therefore must be wrapped by the formatter.\n";
        let options = Options {
            line_width: Some(LineWidth::new(80).unwrap()),
            ..mdx_options()
        };
        let result = format(input, &options).unwrap();
        assert!(result.contains("import { Chart } from \"./chart.js\";"));
        // The prose paragraph must have been wrapped onto multiple lines.
        let prose = result.split("\n\n").nth(1).unwrap_or("");
        assert!(
            prose.lines().count() > 1,
            "expected the prose to wrap: {result:?}"
        );
        assert!(prose.lines().all(|line| line.chars().count() <= 80));
    }

    /// Formatting an MDX document with ESM statements twice yields identical
    /// output.
    #[test]
    fn esm_idempotent() {
        let input = "import { Chart } from \"./chart.js\";\n\n\
            export const meta = { author: 'Hong Minhee' };\n\n\
            Some prose with \"quotes\" and -- dashes.\n";
        let options = mdx_options();
        let first = format(input, &options).unwrap();
        let second = format(&first, &options).unwrap();
        assert_eq!(first, second, "MDX formatting should be idempotent");
        assert!(first.contains("import { Chart } from \"./chart.js\";"));
        assert!(first.contains("export const meta = { author: 'Hong Minhee' };"));
    }

    /// MDX mode does not change a plain Markdown document that has no MDX
    /// constructs: output equals the default-options output.
    #[test]
    fn no_change_for_plain_markdown() {
        let input = "# Title\n\nA paragraph with \"quotes\" and a [link](https://example.com/).\n";
        let with_mdx = format(input, &mdx_options()).unwrap();
        let without_mdx = format(input, &Options::default()).unwrap();
        assert_eq!(with_mdx, without_mdx);
    }
}

/// MDX mode: JSX elements and fragments that comrak would corrupt are preserved
/// verbatim, while plain prose around them is still formatted.
mod mdx_jsx {
    use hongdown::{Options, format};

    fn mdx_options() -> Options {
        Options {
            mdx: true,
            ..Options::default()
        }
    }

    /// A single-line tag with an expression attribute is corrupted without MDX
    /// mode (its string literal is curled) and preserved with it.
    #[test]
    fn single_line_expression_attribute() {
        let input = "<Chart data={{ title: \"Hello\" }} />\n";
        let off = format(input, &Options::default()).unwrap();
        assert!(
            off.contains('\u{201c}'),
            "expected corruption when off: {off:?}"
        );
        let on = format(input, &mdx_options()).unwrap();
        assert_eq!(on, input);
    }

    /// A multi-line self-closing element (open tag incomplete on line 1) loses
    /// its indentation and curls quotes without MDX mode; with it, every byte is
    /// preserved.
    #[test]
    fn multiline_self_closing_element() {
        let input =
            "<PackageManagerTabs\n  command={{\n    npm: \"npm add @scope/pkg\",\n  }}\n/>\n";
        let off = format(input, &Options::default()).unwrap();
        assert_ne!(off, input, "expected corruption when off");
        let on = format(input, &mdx_options()).unwrap();
        assert_eq!(on, input);
    }

    /// JSX fragments are preserved verbatim.
    #[test]
    fn fragment_preserved() {
        let input = "<>It's a \"fragment\"</>\n";
        let on = format(input, &mdx_options()).unwrap();
        assert_eq!(on, input);
        let off = format(input, &Options::default()).unwrap();
        assert!(
            off.contains('\u{201c}'),
            "expected corruption when off: {off:?}"
        );
    }

    /// A container whose opening tag comrak cannot parse (here a `{{…}}` object
    /// attribute, which breaks CommonMark's tag grammar) is preserved as a whole,
    /// keeping its children verbatim.
    #[test]
    fn corrupt_braced_container_preserved() {
        let input = "<Tabs value={{ id: selected }}>It's \"nested\" content</Tabs>\n";
        let off = format(input, &Options::default()).unwrap();
        assert!(
            off.contains('\u{201c}'),
            "expected corruption when off: {off:?}"
        );
        let on = format(input, &mdx_options()).unwrap();
        assert_eq!(on, input);
    }

    /// A container whose opening tag *is* valid inline HTML (a simple `{…}`
    /// attribute) is handled by comrak: the JS attribute is preserved while the
    /// Markdown children are still formatted.
    #[test]
    fn simple_braced_container_keeps_attribute_formats_children() {
        let input = "<Tabs value={selected}>It's \"nested\" content</Tabs>\n";
        let on = format(input, &mdx_options()).unwrap();
        assert!(
            on.contains("value={selected}"),
            "JS attribute preserved: {on:?}"
        );
        assert!(on.contains('\u{201c}'), "children formatted: {on:?}");
    }

    /// A plain (brace-free, single-line) tag is handled by comrak already, so MDX
    /// mode does not change it.
    #[test]
    fn plain_container_unchanged() {
        let input = "<Note type=\"tip\">Some text here.</Note>\n";
        let on = format(input, &mdx_options()).unwrap();
        let off = format(input, &Options::default()).unwrap();
        assert_eq!(on, off);
    }

    /// Inside a plain container, a braced child element is still protected
    /// individually while the surrounding prose is formatted: the child's string
    /// literal keeps straight quotes, but the prose quotes are curled.
    #[test]
    fn braced_child_in_plain_container() {
        let input = "<Note>It's \"great\" and <Badge label={\"wow\"} /> indeed.</Note>\n";
        let on = format(input, &mdx_options()).unwrap();
        assert!(
            on.contains("label={\"wow\"}"),
            "child should keep straight quotes: {on:?}"
        );
        assert!(on.contains('\u{201c}'), "prose quotes should curl: {on:?}");
    }

    /// An inline JSX element in prose is preserved while the surrounding prose is
    /// still punctuation-transformed.
    #[test]
    fn inline_jsx_in_prose() {
        let input = "He said \"hi\" then <Badge count={n} /> appeared.\n";
        let on = format(input, &mdx_options()).unwrap();
        assert!(on.contains("<Badge count={n} />"));
        assert!(
            on.contains('\u{201c}'),
            "prose quotes should still curl: {on:?}"
        );
    }

    /// Formatting an MDX document with JSX is idempotent.
    #[test]
    fn jsx_idempotent() {
        let input = "<Chart data={{ title: \"Hello\" }} />\n\n\
            A paragraph with \"quotes\" and a <Badge count={n} /> inline element.\n";
        let options = mdx_options();
        let first = format(input, &options).unwrap();
        let second = format(&first, &options).unwrap();
        assert_eq!(first, second);
    }

    /// Comparison operators and stray `<` in prose are not mistaken for JSX.
    #[test]
    fn comparison_not_treated_as_jsx() {
        let input = "If a < b and c > d then proceed.\n";
        let on = format(input, &mdx_options()).unwrap();
        let off = format(input, &Options::default()).unwrap();
        assert_eq!(on, off);
    }
}

/// MDX mode: bare `{…}` expressions are preserved verbatim, including their
/// comments and string literals, while surrounding prose is still formatted.
mod mdx_expressions {
    use hongdown::{Options, format};

    fn mdx_options() -> Options {
        Options {
            mdx: true,
            ..Options::default()
        }
    }

    /// A bare flow expression `{/* … */}` is corrupted without MDX mode (escaped
    /// `*`, curled quotes) and preserved with it.
    #[test]
    fn flow_comment_expression() {
        let input = "{/* a JSX comment with \"quotes\" */}\n";
        let off = format(input, &Options::default()).unwrap();
        assert_ne!(off, input, "expected corruption when off");
        let on = format(input, &mdx_options()).unwrap();
        assert_eq!(on, input);
    }

    /// An inline expression in prose is preserved while the prose is formatted.
    #[test]
    fn inline_expression_in_prose() {
        let input = "Total: {count} items in the \"cart\".\n";
        let on = format(input, &mdx_options()).unwrap();
        assert!(on.contains("{count}"));
        assert!(on.contains('\u{201c}'), "prose quotes should curl: {on:?}");
    }

    /// An expression's own string literal keeps straight quotes.
    #[test]
    fn expression_string_preserved() {
        let input = "Price is {formatCurrency(\"USD\", 5)} today.\n";
        let off = format(input, &Options::default()).unwrap();
        assert!(off.contains('\u{201c}'), "expected corruption when off");
        let on = format(input, &mdx_options()).unwrap();
        assert_eq!(on, input);
    }

    /// A `{#id}` heading anchor is not treated as an expression; the heading is
    /// formatted normally.
    #[test]
    fn heading_anchor_not_protected() {
        let input = "## Heading title {#my-id}\n";
        let on = format(input, &mdx_options()).unwrap();
        let off = format(input, &Options::default()).unwrap();
        assert_eq!(on, off);
    }

    /// Braces inside inline code and math are not mistaken for expressions.
    #[test]
    fn braces_in_code_and_math_untouched() {
        let code = "Use `{x}` inline.\n";
        let math = "The value $\\frac{1}{2}$ is a half.\n";
        for input in [code, math] {
            let on = format(input, &mdx_options()).unwrap();
            let off = format(input, &Options::default()).unwrap();
            assert_eq!(
                on, off,
                "code/math braces should be left to comrak: {input:?}"
            );
        }
    }

    /// Braces inside Markdown link/image syntax (text or destination) are owned
    /// by comrak, so MDX mode leaves them alone and stays idempotent — a `{…}`
    /// in link text must not desync from its reference definition.
    #[test]
    fn braces_in_links_untouched() {
        let cases = [
            "See [user](https://example.com/{id}) here.\n",
            "Link [{label}](https://example.com/) text.\n",
            "Image ![alt](https://example.com/{file}.png) here.\n",
        ];
        for input in cases {
            let on = format(input, &mdx_options()).unwrap();
            let off = format(input, &Options::default()).unwrap();
            assert_eq!(
                on, off,
                "link/image braces should be left to comrak: {input:?}"
            );
            assert_eq!(
                on,
                format(&on, &mdx_options()).unwrap(),
                "should be idempotent: {input:?}"
            );
        }
    }

    /// A real expression alongside a link is still protected.
    #[test]
    fn expression_next_to_link_still_protected() {
        let input = "Mixed [{label}](u) and a real {expr} here.\n";
        let on = format(input, &mdx_options()).unwrap();
        assert!(on.contains("{expr}"));
        assert!(on.contains("[{label}](u)"));
        assert_eq!(on, format(&on, &mdx_options()).unwrap());
    }

    /// An expression containing a regex literal whose character class holds a
    /// `}` is preserved as a whole (the regex brace must not end the expression).
    #[test]
    fn expression_with_regex_brace() {
        let input = "Result: {value.replace(/[}]/g, \"x\")} done.\n";
        let off = format(input, &Options::default()).unwrap();
        assert!(
            off.contains('\u{201c}'),
            "expected corruption when off: {off:?}"
        );
        let on = format(input, &mdx_options()).unwrap();
        assert_eq!(on, input);
    }

    /// Division inside an expression is not mistaken for a regex literal.
    #[test]
    fn expression_with_division_is_idempotent() {
        let input = "Ratio: {a / b} and {c / d}.\n";
        let on = format(input, &mdx_options()).unwrap();
        assert!(on.contains("{a / b}"));
        assert_eq!(on, format(&on, &mdx_options()).unwrap());
    }
}

/// MDX mode end to end: the full reproduction from the issue round-trips
/// byte-for-byte and is idempotent.
mod mdx_document {
    use hongdown::{Options, format};

    #[test]
    fn issue_reproduction_preserved_and_idempotent() {
        let input = "import { Chart } from \"./chart.js\";\n\n\
            export const meta = { author: 'Hong Minhee' };\n\n\
            <PackageManagerTabs\n  command={{\n    npm: \"npm add @scope/pkg\",\n    \
            deno: \"deno add jsr:@scope/pkg\",\n  }}\n/>\n\n\
            {/* a JSX comment with \"quotes\" */}\n";
        let options = Options {
            mdx: true,
            ..Options::default()
        };
        let result = format(input, &options).unwrap();
        assert_eq!(result, input, "MDX constructs should be preserved verbatim");
        let again = format(&result, &options).unwrap();
        assert_eq!(result, again, "MDX formatting should be idempotent");
    }

    /// Prose interleaved with MDX constructs is still formatted while the
    /// constructs are preserved.
    #[test]
    fn prose_between_constructs_is_formatted() {
        let input = "import { x } from \"y\";\n\n\
            This paragraph has \"smart quotes\" and needs no wrapping.\n\n\
            <Chart data={x} />\n";
        let options = Options {
            mdx: true,
            ..Options::default()
        };
        let result = format(input, &options).unwrap();
        assert!(result.contains("import { x } from \"y\";"));
        assert!(result.contains("<Chart data={x} />"));
        assert!(
            result.contains('\u{201c}'),
            "prose quotes should curl: {result:?}"
        );
        assert_eq!(result, format(&result, &options).unwrap());
    }
}

mod cli_tests {
    use std::io::Write;
    use std::process::{Command, Stdio};

    /// Helper function to run hongdown CLI with given args and stdin input.
    fn run_hongdown(args: &[&str], stdin_input: Option<&str>) -> (String, String, i32) {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_hongdown"));
        cmd.args(args);

        if stdin_input.is_some() {
            cmd.stdin(Stdio::piped());
        }
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().expect("Failed to spawn hongdown");

        if let Some(input) = stdin_input {
            let mut stdin = child.stdin.take().expect("Failed to get stdin");
            // Ignore broken pipe errors - the process may have exited early
            // (e.g., due to argument validation failure) before reading stdin
            let _ = stdin.write_all(input.as_bytes());
        }

        let output = child.wait_with_output().expect("Failed to wait for output");
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        (stdout, stderr, exit_code)
    }

    /// Test --diff flag shows no output when input is already formatted.
    #[test]
    fn test_diff_no_changes() {
        let formatted_input = "Title\n=====\n\nA paragraph.\n";
        let (stdout, _stderr, exit_code) =
            run_hongdown(&["--diff", "--stdin"], Some(formatted_input));

        // No diff output when already formatted
        assert!(stdout.is_empty(), "No diff expected for formatted input");
        assert_eq!(exit_code, 0);
    }

    /// Test --diff flag shows unified diff when input needs formatting.
    #[test]
    fn test_diff_with_changes() {
        let unformatted_input = "# Title\n\nA paragraph.";
        let (stdout, _stderr, exit_code) =
            run_hongdown(&["--diff", "--stdin"], Some(unformatted_input));

        // Should show diff output
        assert!(stdout.contains("---"), "Diff should contain --- header");
        assert!(stdout.contains("+++"), "Diff should contain +++ header");
        assert!(stdout.contains("-# Title"), "Diff should show removed line");
        assert!(stdout.contains("+Title"), "Diff should show added line");
        assert!(
            stdout.contains("+====="),
            "Diff should show added underline"
        );
        assert_eq!(exit_code, 0);
    }

    /// Test --diff with file input.
    #[test]
    fn test_diff_with_file() {
        use std::fs;
        use tempfile::NamedTempFile;

        // Create a temporary file with unformatted content
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        writeln!(temp_file, "# Test Heading").expect("Failed to write to temp file");
        writeln!(temp_file).expect("Failed to write to temp file");
        writeln!(temp_file, "A paragraph.").expect("Failed to write to temp file");

        let file_path = temp_file.path().to_str().unwrap();
        let (stdout, _stderr, exit_code) = run_hongdown(&["--diff", file_path], None);

        // Should show diff with filename in header
        assert!(stdout.contains("---"), "Diff should contain --- header");
        assert!(stdout.contains("+++"), "Diff should contain +++ header");
        assert_eq!(exit_code, 0);

        // File should not be modified
        let content = fs::read_to_string(temp_file.path()).expect("Failed to read temp file");
        assert!(
            content.contains("# Test Heading"),
            "File should not be modified"
        );
    }

    /// Test --diff and --check are mutually exclusive.
    #[test]
    fn test_diff_check_mutually_exclusive() {
        let (_stdout, stderr, exit_code) =
            run_hongdown(&["--diff", "--check", "--stdin"], Some("# Test"));

        // Should fail with error about conflicting options
        assert_ne!(exit_code, 0);
        assert!(
            stderr.contains("cannot be used with") || stderr.contains("conflict"),
            "Should report conflicting options"
        );
    }

    /// Test --diff and --write are mutually exclusive.
    #[test]
    fn test_diff_write_mutually_exclusive() {
        let (_stdout, stderr, exit_code) =
            run_hongdown(&["--diff", "--write", "--stdin"], Some("# Test"));

        // Should fail with error about conflicting options
        assert_ne!(exit_code, 0);
        assert!(
            stderr.contains("cannot be used with") || stderr.contains("conflict"),
            "Should report conflicting options"
        );
    }

    /// Test --write reports which files were changed.
    #[test]
    fn test_write_reports_changed_files() {
        use std::fs;
        use tempfile::TempDir;

        // Create a temporary directory with test files
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create a file that needs formatting
        let unformatted_path = temp_dir.path().join("unformatted.md");
        fs::write(&unformatted_path, "# Needs Formatting\n\nA paragraph.")
            .expect("Failed to write unformatted file");

        // Create a file that is already formatted
        // Note: heading uses single word to avoid sentence case transformation
        let formatted_path = temp_dir.path().join("formatted.md");
        fs::write(&formatted_path, "Formatted\n=========\n\nA paragraph.\n")
            .expect("Failed to write formatted file");

        let (stdout, _stderr, exit_code) = run_hongdown(
            &[
                "--write",
                unformatted_path.to_str().unwrap(),
                formatted_path.to_str().unwrap(),
            ],
            None,
        );

        assert_eq!(exit_code, 0);

        // Should report the unformatted file as changed
        assert!(
            stdout.contains("unformatted.md"),
            "Should report the changed file: got stdout: {}",
            stdout
        );

        // Should NOT report the already formatted file (check for the exact filename)
        // Note: "unformatted.md" contains "formatted.md" as a substring, so we need to
        // check that "formatted.md" only appears as part of "unformatted.md"
        let stdout_without_unformatted = stdout.replace("unformatted.md", "");
        assert!(
            !stdout_without_unformatted.contains("formatted.md"),
            "Should not report unchanged file: got stdout: {}",
            stdout
        );
    }

    /// Test --write does not report files that are unchanged.
    #[test]
    fn test_write_silent_on_unchanged_files() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create a file that is already formatted
        let formatted_path = temp_dir.path().join("already_formatted.md");
        fs::write(&formatted_path, "Title\n=====\n\nA paragraph.\n")
            .expect("Failed to write formatted file");

        let (stdout, _stderr, exit_code) =
            run_hongdown(&["--write", formatted_path.to_str().unwrap()], None);

        assert_eq!(exit_code, 0);

        // Should not report any files since nothing changed
        assert!(
            stdout.is_empty(),
            "Should not report unchanged files: got stdout: {}",
            stdout
        );
    }

    /// Test that running hongdown without files and without --stdin fails.
    #[test]
    fn test_no_input_error() {
        use tempfile::TempDir;

        // Create a temporary directory without .hongdown.toml
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_hongdown"));
        cmd.current_dir(temp_dir.path());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let output = cmd.output().expect("Failed to run hongdown");
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().unwrap_or(-1);

        // Should fail with error about missing input
        assert_ne!(exit_code, 0, "Should exit with error code");
        assert!(
            stderr.contains("no input files") || stderr.contains("No input"),
            "Error message should mention missing input files: got stderr: {}",
            stderr
        );
    }

    /// Test that --stdin explicitly allows stdin input.
    #[test]
    fn test_stdin_flag_works() {
        let input = "# Test\n\nParagraph.";
        let (stdout, _stderr, exit_code) = run_hongdown(&["--stdin"], Some(input));

        assert_eq!(exit_code, 0);
        assert!(stdout.contains("Test\n===="));
    }

    /// Test that `-` explicitly allows stdin input.
    #[test]
    fn test_dash_for_stdin() {
        let input = "# Test\n\nParagraph.";
        let (stdout, _stderr, exit_code) = run_hongdown(&["-"], Some(input));

        assert_eq!(exit_code, 0);
        assert!(stdout.contains("Test\n===="));
    }

    /// Test that passing a directory as an argument recursively finds .md files.
    #[test]
    fn test_directory_argument_finds_md_files() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create some already-formatted .md files (using setext headings)
        fs::write(
            temp_dir.path().join("README.md"),
            "README\n======\n\nContent.\n",
        )
        .expect("Failed to write README.md");
        fs::write(
            temp_dir.path().join("CHANGELOG.md"),
            "Changelog\n=========\n\nChanges.\n",
        )
        .expect("Failed to write CHANGELOG.md");

        // Create a subdirectory with more .md files
        let docs_dir = temp_dir.path().join("docs");
        fs::create_dir(&docs_dir).expect("Failed to create docs dir");
        fs::write(
            docs_dir.join("guide.md"),
            "Guide\n=====\n\nGuide content.\n",
        )
        .expect("Failed to write guide.md");

        // Create a .markdown file (should also be found)
        fs::write(
            docs_dir.join("reference.markdown"),
            "Reference\n=========\n\nReference content.\n",
        )
        .expect("Failed to write reference.markdown");

        // Create a non-.md file that should be ignored
        fs::write(temp_dir.path().join("main.rs"), "fn main() {}")
            .expect("Failed to write main.rs");

        let (stdout, _stderr, exit_code) =
            run_hongdown(&["--check", temp_dir.path().to_str().unwrap()], None);

        // All .md files are already formatted, so --check should succeed
        assert_eq!(exit_code, 0, "All .md files should be formatted");
        assert!(stdout.is_empty(), "No output expected when all files pass");
    }

    /// Test that --write with directory argument formats all .md files.
    #[test]
    fn test_directory_argument_write_mode() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create an unformatted .md file
        fs::write(temp_dir.path().join("test.md"), "# Test\n\nParagraph.")
            .expect("Failed to write test.md");

        // Create a subdirectory with an unformatted file
        let sub_dir = temp_dir.path().join("sub");
        fs::create_dir(&sub_dir).expect("Failed to create sub dir");
        fs::write(sub_dir.join("nested.md"), "# Nested\n\nContent.")
            .expect("Failed to write nested.md");

        let (stdout, _stderr, exit_code) =
            run_hongdown(&["--write", temp_dir.path().to_str().unwrap()], None);

        assert_eq!(exit_code, 0);

        // Both files should be reported as changed
        assert!(
            stdout.contains("test.md"),
            "Should report test.md as changed"
        );
        assert!(
            stdout.contains("nested.md"),
            "Should report nested.md as changed"
        );

        // Verify files were actually formatted
        let test_content =
            fs::read_to_string(temp_dir.path().join("test.md")).expect("Failed to read test.md");
        assert!(
            test_content.contains("Test\n===="),
            "test.md should be formatted"
        );

        let nested_content =
            fs::read_to_string(sub_dir.join("nested.md")).expect("Failed to read nested.md");
        assert!(
            nested_content.contains("Nested\n======"),
            "nested.md should be formatted"
        );
    }

    /// Test that directory argument with --check fails when files need formatting.
    #[test]
    fn test_directory_argument_check_fails_on_unformatted() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create an unformatted .md file
        fs::write(temp_dir.path().join("unformatted.md"), "# Title\n\nText.")
            .expect("Failed to write unformatted.md");

        let (_stdout, stderr, exit_code) =
            run_hongdown(&["--check", temp_dir.path().to_str().unwrap()], None);

        assert_ne!(exit_code, 0, "Should fail when files need formatting");
        assert!(
            stderr.contains("not formatted"),
            "Should report unformatted file"
        );
    }

    /// Test that empty directory produces no error.
    #[test]
    fn test_directory_argument_empty_dir() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let (stdout, stderr, exit_code) =
            run_hongdown(&["--check", temp_dir.path().to_str().unwrap()], None);

        // Empty directory should succeed (nothing to check)
        assert_eq!(
            exit_code, 0,
            "Empty directory should not fail: stderr={}",
            stderr
        );
        assert!(stdout.is_empty());
    }

    /// Test mixing directory and file arguments.
    #[test]
    fn test_mixed_directory_and_file_arguments() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create a directory with an already-formatted file
        let sub_dir = temp_dir.path().join("docs");
        fs::create_dir(&sub_dir).expect("Failed to create docs dir");
        fs::write(sub_dir.join("doc.md"), "Doc\n===\n\nContent.\n")
            .expect("Failed to write doc.md");

        // Create a standalone already-formatted file
        let standalone = temp_dir.path().join("standalone.md");
        fs::write(&standalone, "Standalone\n==========\n\nText.\n")
            .expect("Failed to write standalone.md");

        let (stdout, _stderr, exit_code) = run_hongdown(
            &[
                "--check",
                sub_dir.to_str().unwrap(),
                standalone.to_str().unwrap(),
            ],
            None,
        );

        assert_eq!(exit_code, 0, "All files should pass check");
        assert!(stdout.is_empty());
    }
}

/// Test proper nouns directive in sentence case.
#[test]
fn test_sentence_case_proper_nouns_directive() {
    let input = r#"<!-- hongdown-proper-nouns: Swift, Go -->

# Using Swift And Go Programming

Some content.
"#;

    let options = Options {
        heading_sentence_case: true,
        ..Options::default()
    };
    let result = format(input, &options).unwrap();

    // Swift and Go should be preserved as proper nouns
    assert!(
        result.contains("Using Swift and Go programming"),
        "Swift and Go should be preserved as proper nouns via directive"
    );
}

/// Test common nouns directive in sentence case.
#[test]
fn test_sentence_case_common_nouns_directive() {
    let input = r#"<!-- hongdown-common-nouns: Python, JavaScript -->

# Learning Python And JavaScript Programming

Some content.
"#;

    let options = Options {
        heading_sentence_case: true,
        ..Options::default()
    };
    let result = format(input, &options).unwrap();

    // Python and JavaScript should NOT be preserved (treated as common nouns)
    assert!(
        result.contains("Learning python and javascript programming"),
        "Python and JavaScript should be lowercased via common-nouns directive"
    );
}

/// Test both directives together.
#[test]
fn test_sentence_case_both_directives() {
    let input = r#"<!-- hongdown-proper-nouns: Swift, Go -->
<!-- hongdown-common-nouns: Python -->

# Using Swift, Go, And Python

Some content.
"#;

    let options = Options {
        heading_sentence_case: true,
        ..Options::default()
    };
    let result = format(input, &options).unwrap();

    // Swift and Go preserved, Python lowercased
    assert!(
        result.contains("Using Swift, Go, and python"),
        "Swift and Go should be proper nouns, Python should be common noun"
    );
}

// ============================================================================
// Code block formatter integration tests
// ============================================================================

#[cfg(not(target_arch = "wasm32"))]
mod code_formatter_tests {
    use hongdown::{CodeFormatter, Options, format, format_with_warnings};
    use std::collections::HashMap;

    /// Test code formatter with a real external command (cat).
    #[test]
    fn test_code_formatter_integration() {
        let mut formatters = HashMap::new();
        formatters.insert(
            "text".to_string(),
            CodeFormatter {
                command: vec!["cat".to_string()],
                timeout_secs: 5,
            },
        );

        let options = Options {
            code_formatters: formatters,
            ..Options::default()
        };

        let input = "~~~~ text\nhello world\n~~~~\n";
        let result = format(input, &options).unwrap();
        assert_eq!(result, "~~~~ text\nhello world\n~~~~\n");
    }

    /// Test code formatter transformation with tr command.
    #[test]
    fn test_code_formatter_transforms() {
        let mut formatters = HashMap::new();
        formatters.insert(
            "upper".to_string(),
            CodeFormatter {
                command: vec!["tr".to_string(), "a-z".to_string(), "A-Z".to_string()],
                timeout_secs: 5,
            },
        );

        let options = Options {
            code_formatters: formatters,
            ..Options::default()
        };

        let input = "~~~~ upper\nhello world\n~~~~\n";
        let result = format(input, &options).unwrap();
        assert_eq!(result, "~~~~ upper\nHELLO WORLD\n~~~~\n");
    }

    /// Test code formatter failure preserves original content and emits warning.
    #[test]
    fn test_code_formatter_failure_warning() {
        let mut formatters = HashMap::new();
        formatters.insert(
            "fail".to_string(),
            CodeFormatter {
                command: vec!["false".to_string()],
                timeout_secs: 5,
            },
        );

        let options = Options {
            code_formatters: formatters,
            ..Options::default()
        };

        let input = "~~~~ fail\noriginal content\n~~~~\n";
        let result = format_with_warnings(input, &options).unwrap();

        // Original content should be preserved
        assert_eq!(result.output, "~~~~ fail\noriginal content\n~~~~\n");

        // Warning should be emitted
        assert!(!result.warnings.is_empty());
        assert!(result.warnings[0].message.contains("failed"));
    }

    /// Test multiple code blocks with different languages.
    #[test]
    fn test_multiple_code_blocks() {
        let mut formatters = HashMap::new();
        formatters.insert(
            "upper".to_string(),
            CodeFormatter {
                command: vec!["tr".to_string(), "a-z".to_string(), "A-Z".to_string()],
                timeout_secs: 5,
            },
        );
        // No formatter for "rust" - should preserve original

        let options = Options {
            code_formatters: formatters,
            ..Options::default()
        };

        let input = r#"First block:

~~~~ upper
hello
~~~~

Second block:

~~~~ rust
fn main() {}
~~~~
"#;
        let result = format(input, &options).unwrap();

        // First block should be transformed
        assert!(result.contains("HELLO"));
        // Second block should be unchanged
        assert!(result.contains("fn main() {}"));
    }

    /// Test code formatter with default_language.
    #[test]
    fn test_code_formatter_with_default_language() {
        let mut formatters = HashMap::new();
        formatters.insert(
            "text".to_string(),
            CodeFormatter {
                command: vec!["tr".to_string(), "a-z".to_string(), "A-Z".to_string()],
                timeout_secs: 5,
            },
        );

        let options = Options {
            default_language: "text".to_string(),
            code_formatters: formatters,
            ..Options::default()
        };

        // Code block without language should use default and apply formatter
        let input = "~~~~\nhello\n~~~~\n";
        let result = format(input, &options).unwrap();
        assert_eq!(result, "~~~~ text\nHELLO\n~~~~\n");
    }
}

// ============================================================================
// Cascading configuration integration tests
// ============================================================================

#[cfg(not(target_arch = "wasm32"))]
mod cascading_config_integration_tests {
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use tempfile::TempDir;

    /// Helper to create a config file with given content.
    fn create_config(dir: &Path, content: &str) {
        let config_path = dir.join(".hongdown.toml");
        fs::write(&config_path, content).unwrap();
    }

    /// Helper to create a test markdown file.
    fn create_markdown_file(dir: &Path, filename: &str, content: &str) {
        let file_path = dir.join(filename);
        fs::write(&file_path, content).unwrap();
    }

    /// Helper to run hongdown on a file and return the output.
    fn run_hongdown(markdown_path: &Path) -> String {
        let output = Command::new(env!("CARGO_BIN_EXE_hongdown"))
            .arg(markdown_path)
            .current_dir(markdown_path.parent().unwrap())
            .output()
            .expect("Failed to execute hongdown");

        assert!(
            output.status.success(),
            "hongdown failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        String::from_utf8(output.stdout).unwrap()
    }

    /// Test that project config is used when present.
    #[test]
    fn test_project_config_priority() {
        let temp_dir = TempDir::new().unwrap();

        // Create project config with line_width = 40
        create_config(
            temp_dir.path(),
            r#"
line_width = 40
"#,
        );

        // Create a markdown file with long line
        create_markdown_file(
            temp_dir.path(),
            "test.md",
            "This is a very long line that should definitely be wrapped at 40 characters if the config is applied correctly.",
        );

        let markdown_path = temp_dir.path().join("test.md");
        let result = run_hongdown(&markdown_path);

        // Verify line wrapping occurred (multiple lines in output)
        let lines: Vec<&str> = result.lines().collect();
        assert!(
            lines.len() > 1,
            "Line should be wrapped with line_width = 40"
        );
        for line in &lines {
            assert!(
                line.len() <= 42, // Allow slight overflow for words
                "Line should not exceed ~40 characters: {}",
                line
            );
        }
    }

    /// Test that nearest project config is used (parent configs ignored).
    #[test]
    fn test_nearest_project_config_used() {
        let temp_dir = TempDir::new().unwrap();
        let parent = temp_dir.path();
        let child = parent.join("project");
        fs::create_dir(&child).unwrap();

        // Parent config: line_width = 40
        create_config(parent, "line_width = 40");

        // Child config: line_width = 120 (nearest config takes precedence)
        create_config(&child, "line_width = 120");

        // Create a markdown file with long line in child directory
        create_markdown_file(
            &child,
            "test.md",
            "This is a very long line that should not be wrapped because the nearest config has line_width = 120.",
        );

        let markdown_path = child.join("test.md");
        let result = run_hongdown(&markdown_path);

        // Verify line was NOT wrapped (nearest config should be used)
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(
            lines.len(),
            1,
            "Line should not be wrapped with line_width = 120 from nearest config"
        );
    }

    /// Test that explicit --config bypasses cascading.
    #[test]
    fn test_explicit_config_bypasses_cascading() {
        let temp_dir = TempDir::new().unwrap();

        // Create project config with line_width = 40
        create_config(temp_dir.path(), "line_width = 40");

        // Create separate explicit config with line_width = 120
        let explicit_config_path = temp_dir.path().join("custom.toml");
        fs::write(&explicit_config_path, "line_width = 120").unwrap();

        // Create markdown file with long line
        create_markdown_file(
            temp_dir.path(),
            "test.md",
            "This is a very long line that should not be wrapped because we're using explicit config with line_width = 120.",
        );

        let markdown_path = temp_dir.path().join("test.md");

        // Run with explicit config
        let output = Command::new(env!("CARGO_BIN_EXE_hongdown"))
            .arg("--config")
            .arg(&explicit_config_path)
            .arg(&markdown_path)
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to execute hongdown");

        assert!(
            output.status.success(),
            "hongdown failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let result = String::from_utf8(output.stdout).unwrap();

        // Verify line was NOT wrapped (explicit config should override project config)
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(
            lines.len(),
            1,
            "Line should not be wrapped with explicit config line_width = 120"
        );
    }

    /// Test that config is discovered from parent directories.
    #[test]
    fn test_config_discovered_from_parent() {
        let temp_dir = TempDir::new().unwrap();
        let parent = temp_dir.path();
        let child = parent.join("subdir");
        fs::create_dir(&child).unwrap();

        // Only parent has config with line_width = 40
        create_config(parent, "line_width = 40");

        // Create markdown file in child directory (no config there)
        create_markdown_file(
            &child,
            "test.md",
            "This is a very long line that should be wrapped because config is discovered from parent directory.",
        );

        let markdown_path = child.join("test.md");
        let result = run_hongdown(&markdown_path);

        // Verify line was wrapped (parent config should be found)
        let lines: Vec<&str> = result.lines().collect();
        assert!(
            lines.len() > 1,
            "Line should be wrapped using parent directory config"
        );
    }

    /// Test end-to-end formatting with cascading configs.
    #[test]
    fn test_cascading_config_formatting() {
        let temp_dir = TempDir::new().unwrap();

        // Create config with specific formatting options
        create_config(
            temp_dir.path(),
            r#"
line_width = 80

[heading]
setext_h1 = true
setext_h2 = false

[unordered_list]
unordered_marker = "*"
"#,
        );

        // Create markdown file with various elements
        create_markdown_file(
            temp_dir.path(),
            "test.md",
            r#"# Main Title

## Subsection

 -  First item
 -  Second item
"#,
        );

        let markdown_path = temp_dir.path().join("test.md");
        let result = run_hongdown(&markdown_path);

        // Verify formatting rules from config are applied
        assert!(result.contains("Main Title\n="), "H1 should use setext");
        assert!(result.contains("## Subsection"), "H2 should use ATX");
        assert!(
            result.contains(" *  First item"),
            "List should use * marker"
        );
        assert!(
            result.contains(" *  Second item"),
            "List should use * marker"
        );
    }

    /// Test that sentence case preserves trailing explicit anchor names.
    #[test]
    fn test_sentence_case_preserves_explicit_anchor_name() {
        let temp_dir = TempDir::new().unwrap();

        create_config(
            temp_dir.path(),
            r#"
[heading]
sentence_case = true
"#,
        );

        create_markdown_file(temp_dir.path(), "test.md", "## Test Section {#myAPI}\n");

        let markdown_path = temp_dir.path().join("test.md");
        let result = run_hongdown(&markdown_path);

        // With default anchor_align = 0, anchor is right-aligned to line_width (80).
        // "Test section" (12) + 60 spaces + "{#myAPI}" (8) = 80 chars; underline = 80 '-'
        assert_eq!(
            result,
            "Test section                                                            {#myAPI}\n\
             --------------------------------------------------------------------------------\n"
        );
    }

    /// anchor_align = 3: exactly 3 spaces between heading body and anchor.
    #[test]
    fn test_anchor_align_positive() {
        let temp_dir = TempDir::new().unwrap();

        create_config(
            temp_dir.path(),
            r#"
[heading]
anchor_align = 3
"#,
        );

        create_markdown_file(temp_dir.path(), "test.md", "## Section {#section-1}\n");

        let markdown_path = temp_dir.path().join("test.md");
        let result = run_hongdown(&markdown_path);

        // "Section" (7) + 3 spaces + "{#section-1}" (12) = 22 chars; underline = 22 '-'
        assert_eq!(result, "Section   {#section-1}\n----------------------\n");
    }

    /// anchor_align = 0 with line_width = 80: heading line is exactly 80 chars.
    #[test]
    fn test_anchor_align_zero_right_aligns() {
        let temp_dir = TempDir::new().unwrap();

        create_config(
            temp_dir.path(),
            r#"
line_width = 80

[heading]
anchor_align = 0
"#,
        );

        create_markdown_file(temp_dir.path(), "test.md", "# Title {#title}\n");

        let markdown_path = temp_dir.path().join("test.md");
        let result = run_hongdown(&markdown_path);

        // "Title" (5) + spaces + "{#title}" (8) = 80 chars; underline = 80 '='
        let first_line = result.lines().next().unwrap();
        assert_eq!(
            first_line.len(),
            80,
            "heading line should be exactly 80 chars"
        );
        assert!(first_line.ends_with("{#title}"));
    }

    /// anchor_align = -5 with line_width = 80: heading line is 75 chars.
    #[test]
    fn test_anchor_align_negative_shorter() {
        let temp_dir = TempDir::new().unwrap();

        create_config(
            temp_dir.path(),
            r#"
line_width = 80

[heading]
anchor_align = -5
"#,
        );

        create_markdown_file(temp_dir.path(), "test.md", "## Section {#s}\n");

        let markdown_path = temp_dir.path().join("test.md");
        let result = run_hongdown(&markdown_path);

        // "Section" (7) + spaces + "{#s}" (4) = 75 chars; underline = 75 '-'
        let first_line = result.lines().next().unwrap();
        assert_eq!(
            first_line.len(),
            75,
            "heading line should be exactly 75 chars"
        );
        assert!(first_line.ends_with("{#s}"));
    }

    /// anchor_align has no effect on headings without an explicit anchor.
    #[test]
    fn test_anchor_align_no_effect_without_anchor() {
        let temp_dir = TempDir::new().unwrap();

        create_config(
            temp_dir.path(),
            r#"
[heading]
anchor_align = 10
"#,
        );

        create_markdown_file(temp_dir.path(), "test.md", "# Plain Heading\n");

        let markdown_path = temp_dir.path().join("test.md");
        let result = run_hongdown(&markdown_path);

        assert_eq!(result, "Plain Heading\n=============\n");
    }

    /// anchor_align = 0 with line_width = false: falls back to 1 space.
    #[test]
    fn test_anchor_align_fallback_no_line_width() {
        let temp_dir = TempDir::new().unwrap();

        create_config(
            temp_dir.path(),
            r#"
line_width = false

[heading]
anchor_align = 0
"#,
        );

        create_markdown_file(temp_dir.path(), "test.md", "# Title {#t}\n");

        let markdown_path = temp_dir.path().join("test.md");
        let result = run_hongdown(&markdown_path);

        // line_width = false → fall back to 1 space
        assert_eq!(result, "Title {#t}\n==========\n");
    }
}
