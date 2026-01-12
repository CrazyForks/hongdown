//! WebAssembly bindings for Hongdown.
//!
//! This module provides JavaScript-friendly bindings for the Hongdown formatter.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::Options;
use crate::config::{DashSetting, OrderedListPad};

/// JavaScript-friendly options struct.
///
/// All fields are optional and use camelCase naming for JavaScript conventions.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct JsOptions {
    /// Line width for wrapping (default: 80).
    pub line_width: Option<usize>,

    /// Use setext-style for h1 headings (default: true).
    pub setext_h1: Option<bool>,

    /// Use setext-style for h2 headings (default: true).
    pub setext_h2: Option<bool>,

    /// Marker for unordered lists: "-", "*", or "+" (default: "-").
    pub unordered_marker: Option<String>,

    /// Leading spaces before list marker (default: 1).
    pub leading_spaces: Option<usize>,

    /// Trailing spaces after list marker (default: 2).
    pub trailing_spaces: Option<usize>,

    /// Indent width for nested items (default: 4).
    pub indent_width: Option<usize>,

    /// Marker for odd-level ordered lists (default: ".").
    pub odd_level_marker: Option<String>,

    /// Marker for even-level ordered lists (default: ")").
    pub even_level_marker: Option<String>,

    /// Padding style for ordered list numbers: "start" or "end" (default: "start").
    pub ordered_list_pad: Option<String>,

    /// Indent width for nested ordered lists (default: 4).
    pub ordered_list_indent_width: Option<usize>,

    /// Fence character: "~" or "`" (default: "~").
    pub fence_char: Option<String>,

    /// Minimum fence length (default: 4).
    pub min_fence_length: Option<usize>,

    /// Space after fence character (default: true).
    pub space_after_fence: Option<bool>,

    /// Default language for code blocks (default: "").
    pub default_language: Option<String>,

    /// Thematic break style (default: spaced dashes).
    pub thematic_break_style: Option<String>,

    /// Leading spaces for thematic breaks (default: 3).
    pub thematic_break_leading_spaces: Option<usize>,

    /// Convert straight double quotes to curly (default: true).
    pub curly_double_quotes: Option<bool>,

    /// Convert straight single quotes to curly (default: true).
    pub curly_single_quotes: Option<bool>,

    /// Convert apostrophes to curly (default: false).
    pub curly_apostrophes: Option<bool>,

    /// Convert ... to ellipsis (default: true).
    pub ellipsis: Option<bool>,

    /// En-dash setting: false to disable, or a string pattern (default: false).
    pub en_dash: Option<JsDashSetting>,

    /// Em-dash setting: false to disable, or a string pattern (default: "--").
    pub em_dash: Option<JsDashSetting>,
}

/// JavaScript-friendly dash setting.
///
/// Can be either `false` (disabled) or a string pattern.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum JsDashSetting {
    /// Disabled when false.
    Disabled(bool),
    /// Pattern to transform to dash.
    Pattern(String),
}

impl JsDashSetting {
    fn to_dash_setting(&self) -> DashSetting {
        match self {
            JsDashSetting::Disabled(false) => DashSetting::Disabled,
            JsDashSetting::Disabled(true) => DashSetting::Disabled,
            JsDashSetting::Pattern(s) => DashSetting::Pattern(s.clone()),
        }
    }
}

impl JsOptions {
    /// Convert JavaScript options to Rust Options.
    fn to_options(&self) -> Options {
        let mut opts = Options::default();

        if let Some(v) = self.line_width {
            opts.line_width = v;
        }
        if let Some(v) = self.setext_h1 {
            opts.setext_h1 = v;
        }
        if let Some(v) = self.setext_h2 {
            opts.setext_h2 = v;
        }
        if let Some(ref v) = self.unordered_marker {
            if let Some(c) = v.chars().next() {
                opts.unordered_marker = c;
            }
        }
        if let Some(v) = self.leading_spaces {
            opts.leading_spaces = v;
        }
        if let Some(v) = self.trailing_spaces {
            opts.trailing_spaces = v;
        }
        if let Some(v) = self.indent_width {
            opts.indent_width = v;
        }
        if let Some(ref v) = self.odd_level_marker {
            if let Some(c) = v.chars().next() {
                opts.odd_level_marker = c;
            }
        }
        if let Some(ref v) = self.even_level_marker {
            if let Some(c) = v.chars().next() {
                opts.even_level_marker = c;
            }
        }
        if let Some(ref v) = self.ordered_list_pad {
            opts.ordered_list_pad = match v.as_str() {
                "end" => OrderedListPad::End,
                _ => OrderedListPad::Start,
            };
        }
        if let Some(v) = self.ordered_list_indent_width {
            opts.ordered_list_indent_width = v;
        }
        if let Some(ref v) = self.fence_char {
            if let Some(c) = v.chars().next() {
                opts.fence_char = c;
            }
        }
        if let Some(v) = self.min_fence_length {
            opts.min_fence_length = v;
        }
        if let Some(v) = self.space_after_fence {
            opts.space_after_fence = v;
        }
        if let Some(ref v) = self.default_language {
            opts.default_language = v.clone();
        }
        if let Some(ref v) = self.thematic_break_style {
            opts.thematic_break_style = v.clone();
        }
        if let Some(v) = self.thematic_break_leading_spaces {
            opts.thematic_break_leading_spaces = v;
        }
        if let Some(v) = self.curly_double_quotes {
            opts.curly_double_quotes = v;
        }
        if let Some(v) = self.curly_single_quotes {
            opts.curly_single_quotes = v;
        }
        if let Some(v) = self.curly_apostrophes {
            opts.curly_apostrophes = v;
        }
        if let Some(v) = self.ellipsis {
            opts.ellipsis = v;
        }
        if let Some(ref v) = self.en_dash {
            opts.en_dash = v.to_dash_setting();
        }
        if let Some(ref v) = self.em_dash {
            opts.em_dash = v.to_dash_setting();
        }

        opts
    }
}

/// Format result with warnings.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsFormatResult {
    /// The formatted Markdown output.
    pub output: String,
    /// Warnings generated during formatting.
    pub warnings: Vec<JsWarning>,
}

/// A warning generated during formatting.
#[derive(Debug, Serialize)]
pub struct JsWarning {
    /// Line number where the warning was generated (1-indexed).
    pub line: usize,
    /// Warning message.
    pub message: String,
}

/// Format Markdown according to Hong Minhee's style conventions.
///
/// # Arguments
///
/// * `input` - Markdown source to format
/// * `options` - Optional formatting options as a JavaScript object
///
/// # Returns
///
/// The formatted Markdown string.
#[wasm_bindgen]
pub fn format(input: &str, options: JsValue) -> Result<String, JsError> {
    let js_opts: JsOptions = if options.is_undefined() || options.is_null() {
        JsOptions::default()
    } else {
        serde_wasm_bindgen::from_value(options).map_err(|e| JsError::new(&e.to_string()))?
    };

    let opts = js_opts.to_options();
    crate::format(input, &opts).map_err(|e| JsError::new(&e.to_string()))
}

/// Format Markdown and return both output and warnings.
///
/// # Arguments
///
/// * `input` - Markdown source to format
/// * `options` - Optional formatting options as a JavaScript object
///
/// # Returns
///
/// An object with `output` (formatted string) and `warnings` (array of warning objects).
#[wasm_bindgen(js_name = formatWithWarnings)]
pub fn format_with_warnings(input: &str, options: JsValue) -> Result<JsValue, JsError> {
    let js_opts: JsOptions = if options.is_undefined() || options.is_null() {
        JsOptions::default()
    } else {
        serde_wasm_bindgen::from_value(options).map_err(|e| JsError::new(&e.to_string()))?
    };

    let opts = js_opts.to_options();
    let result =
        crate::format_with_warnings(input, &opts).map_err(|e| JsError::new(&e.to_string()))?;

    let js_result = JsFormatResult {
        output: result.output,
        warnings: result
            .warnings
            .into_iter()
            .map(|w| JsWarning {
                line: w.line,
                message: w.message,
            })
            .collect(),
    };

    serde_wasm_bindgen::to_value(&js_result).map_err(|e| JsError::new(&e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_js_options_default() {
        let js_opts = JsOptions::default();
        let opts = js_opts.to_options();
        assert_eq!(opts.line_width, 80);
        assert!(opts.setext_h1);
        assert!(opts.setext_h2);
    }

    #[test]
    fn test_js_options_partial() {
        let js_opts = JsOptions {
            line_width: Some(100),
            setext_h1: Some(false),
            ..Default::default()
        };
        let opts = js_opts.to_options();
        assert_eq!(opts.line_width, 100);
        assert!(!opts.setext_h1);
        assert!(opts.setext_h2); // default
    }

    #[test]
    fn test_js_dash_setting_disabled() {
        let setting = JsDashSetting::Disabled(false);
        assert!(matches!(setting.to_dash_setting(), DashSetting::Disabled));
    }

    #[test]
    fn test_js_dash_setting_pattern() {
        let setting = JsDashSetting::Pattern("--".to_string());
        match setting.to_dash_setting() {
            DashSetting::Pattern(p) => assert_eq!(p, "--"),
            _ => panic!("Expected Pattern"),
        }
    }
}
