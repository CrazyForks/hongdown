//! Text escaping and formatting utilities for Markdown serialization.

/// Normalize whitespace in text: convert newlines and multiple spaces to single space.
pub fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Escape special Markdown characters in text content.
/// Characters that could be misinterpreted as Markdown syntax need escaping.
pub fn escape_text(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();

    for (i, &ch) in chars.iter().enumerate() {
        match ch {
            // Asterisk always needs escaping (can create emphasis anywhere)
            '*' => {
                result.push('\\');
                result.push(ch);
            }
            // Underscore always needs escaping for safety
            // While CommonMark doesn't create emphasis for intraword underscores,
            // escaping ensures consistent behavior across all Markdown parsers
            '_' => {
                result.push('\\');
                result.push(ch);
            }
            // Square brackets - only escape if they could form a link
            // A '[' at the end can't start a link, ']' at the start can't close one
            // Adjacent brackets like '[[' or ']]' also don't need escaping
            '[' => {
                let next_is_bracket = i + 1 < chars.len() && chars[i + 1] == '[';
                let at_end = i + 1 >= chars.len();
                if next_is_bracket || at_end {
                    result.push(ch);
                } else {
                    result.push('\\');
                    result.push(ch);
                }
            }
            ']' => {
                let prev_is_bracket = i > 0 && chars[i - 1] == ']';
                let at_start = i == 0;
                let at_end = i + 1 >= chars.len();
                // A ']' can only close a link if followed by '(' or '['
                // At end of text or followed by other chars, it's just text
                let next_could_continue_link =
                    i + 1 < chars.len() && (chars[i + 1] == '(' || chars[i + 1] == '[');
                if prev_is_bracket || at_start || at_end || !next_could_continue_link {
                    result.push(ch);
                } else {
                    result.push('\\');
                    result.push(ch);
                }
            }
            // Backslash itself needs escaping
            '\\' => {
                result.push('\\');
                result.push(ch);
            }
            // Backtick could start code spans
            '`' => {
                result.push('\\');
                result.push(ch);
            }
            // Other characters pass through unchanged
            _ => result.push(ch),
        }
    }
    result
}

/// Format a code span with the appropriate number of backticks.
/// According to CommonMark spec, if the content contains N consecutive backticks,
/// the delimiter must use at least N+1 backticks. Spaces are added if the content
/// starts or ends with a backtick or space (and content is not all spaces).
pub fn format_code_span(content: &str) -> String {
    // Find the maximum number of consecutive backticks in the content
    let mut max_consecutive = 0;
    let mut current_consecutive = 0;
    for ch in content.chars() {
        if ch == '`' {
            current_consecutive += 1;
            max_consecutive = max_consecutive.max(current_consecutive);
        } else {
            current_consecutive = 0;
        }
    }

    // Use max_consecutive + 1 backticks as delimiters
    let delimiter_count = if max_consecutive > 0 {
        max_consecutive + 1
    } else {
        1
    };
    let delimiter: String = "`".repeat(delimiter_count);

    // Determine if we need space padding.
    // Per CommonMark: space padding is needed when content begins or ends with
    // a backtick character. This is because backticks adjacent to the delimiter
    // would be ambiguous. Spaces at the start/end do NOT require padding -
    // they are preserved as-is.
    let needs_space = if content.is_empty() {
        false
    } else {
        let first = content.chars().next().unwrap();
        let last = content.chars().last().unwrap();
        first == '`' || last == '`'
    };

    if needs_space {
        format!("{} {} {}", delimiter, content, delimiter)
    } else {
        format!("{}{}{}", delimiter, content, delimiter)
    }
}

/// Check if a string is a valid code span (starts and ends with matching backticks).
/// This is used to validate source extraction results, since comrak may provide
/// incorrect sourcepos for code spans containing escaped pipe characters in tables.
pub fn is_valid_code_span(source: &str) -> bool {
    if source.is_empty() {
        return false;
    }

    // Count leading backticks
    let leading_backticks = source.chars().take_while(|&c| c == '`').count();
    if leading_backticks == 0 {
        return false;
    }

    // Count trailing backticks
    let trailing_backticks = source.chars().rev().take_while(|&c| c == '`').count();

    // A valid code span has matching backtick counts at both ends
    leading_backticks == trailing_backticks && leading_backticks <= source.len() / 2
}

/// Escape pipe characters in table cell content.
/// Pipes must be escaped to prevent being interpreted as cell boundaries.
pub fn escape_table_cell(content: &str) -> String {
    // Escape unescaped pipe characters
    // We need to be careful not to double-escape already escaped pipes
    let mut result = String::with_capacity(content.len());
    let chars: Vec<char> = content.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            // Already escaped character - preserve both
            result.push(chars[i]);
            result.push(chars[i + 1]);
            i += 2;
        } else if chars[i] == '|' {
            // Unescaped pipe - escape it
            result.push('\\');
            result.push('|');
            i += 1;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_code_span() {
        // Valid code spans
        assert!(is_valid_code_span("`foo`"));
        assert!(is_valid_code_span("`string \\| number`"));
        assert!(is_valid_code_span("``foo``"));
        assert!(is_valid_code_span("`` foo ` bar ``"));

        // Invalid code spans (missing closing backtick)
        assert!(!is_valid_code_span("`foo"));
        assert!(!is_valid_code_span("`string \\| number"));
        assert!(!is_valid_code_span("``foo`"));

        // Edge cases
        assert!(!is_valid_code_span(""));
        assert!(!is_valid_code_span("foo"));
        assert!(!is_valid_code_span("foo`"));
    }
}
