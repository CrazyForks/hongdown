//! WebAssembly bindings for Hongdown.
//!
//! This module provides JavaScript-friendly bindings for the Hongdown formatter.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::Options;
use crate::config::{
    Config, DashPattern, DashSetting, FenceChar, IndentWidth, LeadingSpaces, LineWidth,
    MinFenceLength, OrderedListPad, OrderedMarker, ThematicBreakStyle, TrailingSpaces,
    UnorderedMarker,
};

/// JavaScript-friendly line width setting.
///
/// Can be either `false` (no wrapping) or a positive integer.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum JsLineWidthSetting {
    /// Disabled when `false`; `true` is ignored (keeps default).
    NoWrap(bool),
    /// Wrap at this many columns.
    Width(usize),
}

/// JavaScript-friendly options struct.
///
/// All fields are optional and use camelCase naming for JavaScript conventions.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct JsOptions {
    /// Line width for wrapping (`false` disables wrapping; default: 80).
    pub line_width: Option<JsLineWidthSetting>,

    /// Recognize and preserve TeX/LaTeX math expressions (default: true).
    pub math: Option<bool>,

    /// Enable MDX mode: preserve embedded JavaScript/JSX verbatim (default:
    /// false).
    pub mdx: Option<bool>,

    /// Use setext-style for h1 headings (default: true).
    pub setext_h1: Option<bool>,

    /// Use setext-style for h2 headings (default: true).
    pub setext_h2: Option<bool>,

    /// Convert headings to sentence case (default: false).
    pub heading_sentence_case: Option<bool>,

    /// Additional proper nouns to preserve in sentence case.
    /// These are merged with built-in proper nouns.
    pub heading_proper_nouns: Option<Vec<String>>,

    /// Words to treat as common nouns in sentence case.
    /// These are excluded from built-in proper nouns.
    pub heading_common_nouns: Option<Vec<String>>,

    /// Spacing between heading body and explicit anchor ID.
    pub heading_anchor_align: Option<i32>,

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
#[derive(Debug, Clone, Deserialize, Serialize)]
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
            JsDashSetting::Pattern(s) => DashPattern::new(s.clone())
                .map(DashSetting::Pattern)
                .unwrap_or(DashSetting::Disabled),
        }
    }
}

impl JsOptions {
    /// Convert JavaScript options to Rust Options.
    fn to_options(&self) -> Options {
        let mut opts = Options::default();

        match &self.line_width {
            Some(JsLineWidthSetting::NoWrap(false)) => opts.line_width = None,
            Some(JsLineWidthSetting::NoWrap(true)) => {}
            Some(JsLineWidthSetting::Width(v)) => {
                if let Ok(lw) = LineWidth::new(*v) {
                    opts.line_width = Some(lw);
                }
            }
            None => {}
        }
        if let Some(v) = self.math {
            opts.math = v;
        }
        if let Some(v) = self.mdx {
            opts.mdx = v;
        }
        if let Some(v) = self.setext_h1 {
            opts.setext_h1 = v;
        }
        if let Some(v) = self.setext_h2 {
            opts.setext_h2 = v;
        }
        if let Some(v) = self.heading_sentence_case {
            opts.heading_sentence_case = v;
        }
        if let Some(ref v) = self.heading_proper_nouns {
            opts.heading_proper_nouns = v.clone();
        }
        if let Some(ref v) = self.heading_common_nouns {
            opts.heading_common_nouns = v.clone();
        }
        if let Some(v) = self.heading_anchor_align {
            opts.heading_anchor_align = v;
        }
        if let Some(ref v) = self.unordered_marker {
            opts.unordered_marker = match v.as_str() {
                "*" => UnorderedMarker::Asterisk,
                "+" => UnorderedMarker::Plus,
                _ => UnorderedMarker::Hyphen,
            };
        }
        if let Some(v) = self.leading_spaces {
            if let Ok(leading) = LeadingSpaces::new(v) {
                opts.leading_spaces = leading;
            }
        }
        if let Some(v) = self.trailing_spaces {
            if let Ok(trailing) = TrailingSpaces::new(v) {
                opts.trailing_spaces = trailing;
            }
        }
        if let Some(v) = self.indent_width {
            if let Ok(width) = IndentWidth::new(v) {
                opts.indent_width = width;
            }
        }
        if let Some(ref v) = self.odd_level_marker {
            opts.odd_level_marker = match v.as_str() {
                ")" => OrderedMarker::Parenthesis,
                _ => OrderedMarker::Period,
            };
        }
        if let Some(ref v) = self.even_level_marker {
            opts.even_level_marker = match v.as_str() {
                "." => OrderedMarker::Period,
                _ => OrderedMarker::Parenthesis,
            };
        }
        if let Some(ref v) = self.ordered_list_pad {
            opts.ordered_list_pad = match v.as_str() {
                "end" => OrderedListPad::End,
                _ => OrderedListPad::Start,
            };
        }
        if let Some(v) = self.ordered_list_indent_width {
            if let Ok(width) = IndentWidth::new(v) {
                opts.ordered_list_indent_width = width;
            }
        }
        if let Some(ref v) = self.fence_char {
            opts.fence_char = match v.as_str() {
                "`" => FenceChar::Backtick,
                _ => FenceChar::Tilde,
            };
        }
        if let Some(v) = self.min_fence_length {
            if let Ok(min_len) = MinFenceLength::new(v) {
                opts.min_fence_length = min_len;
            }
        }
        if let Some(v) = self.space_after_fence {
            opts.space_after_fence = v;
        }
        if let Some(ref v) = self.default_language {
            opts.default_language = v.clone();
        }
        if let Some(ref v) = self.thematic_break_style {
            if let Ok(style) = ThematicBreakStyle::new(v.clone()) {
                opts.thematic_break_style = style;
            }
        }
        if let Some(v) = self.thematic_break_leading_spaces {
            if let Ok(leading) = LeadingSpaces::new(v) {
                opts.thematic_break_leading_spaces = leading;
            }
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

impl From<&Config> for JsOptions {
    fn from(config: &Config) -> Self {
        Self {
            line_width: Some(match config.line_width {
                Some(width) => JsLineWidthSetting::Width(width.get()),
                None => JsLineWidthSetting::NoWrap(false),
            }),
            math: Some(config.math),
            mdx: Some(config.mdx),
            setext_h1: Some(config.heading.setext_h1),
            setext_h2: Some(config.heading.setext_h2),
            heading_sentence_case: Some(config.heading.sentence_case),
            heading_proper_nouns: Some(config.heading.proper_nouns.clone()),
            heading_common_nouns: Some(config.heading.common_nouns.clone()),
            heading_anchor_align: Some(config.heading.anchor_align),
            unordered_marker: Some(config.unordered_list.unordered_marker.as_char().to_string()),
            leading_spaces: Some(config.unordered_list.leading_spaces.get()),
            trailing_spaces: Some(config.unordered_list.trailing_spaces.get()),
            indent_width: Some(config.unordered_list.indent_width.get()),
            odd_level_marker: Some(config.ordered_list.odd_level_marker.as_char().to_string()),
            even_level_marker: Some(config.ordered_list.even_level_marker.as_char().to_string()),
            ordered_list_pad: Some(match config.ordered_list.pad {
                OrderedListPad::Start => "start".to_string(),
                OrderedListPad::End => "end".to_string(),
            }),
            ordered_list_indent_width: Some(config.ordered_list.indent_width.get()),
            fence_char: Some(config.code_block.fence_char.as_char().to_string()),
            min_fence_length: Some(config.code_block.min_fence_length.get()),
            space_after_fence: Some(config.code_block.space_after_fence),
            default_language: Some(config.code_block.default_language.clone()),
            thematic_break_style: Some(config.thematic_break.style.as_str().to_string()),
            thematic_break_leading_spaces: Some(config.thematic_break.leading_spaces.get()),
            curly_double_quotes: Some(config.punctuation.curly_double_quotes),
            curly_single_quotes: Some(config.punctuation.curly_single_quotes),
            curly_apostrophes: Some(config.punctuation.curly_apostrophes),
            ellipsis: Some(config.punctuation.ellipsis),
            en_dash: Some(dash_setting_to_js(&config.punctuation.en_dash)),
            em_dash: Some(dash_setting_to_js(&config.punctuation.em_dash)),
        }
    }
}

fn dash_setting_to_js(setting: &DashSetting) -> JsDashSetting {
    match setting {
        DashSetting::Disabled => JsDashSetting::Disabled(false),
        DashSetting::Pattern(pattern) => JsDashSetting::Pattern(pattern.as_str().to_string()),
    }
}

/// Result of loading a TOML configuration for JavaScript callers.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsConfigResult {
    /// Formatting options equivalent to the TOML configuration.
    pub options: JsOptions,
    /// Warnings about configuration entries unavailable in the WASM runtime.
    pub warnings: Vec<String>,
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

/// Parse a `.hongdown.toml` configuration string into JavaScript options.
///
/// External code block formatters are reported as warnings because the WASM
/// runtime cannot execute external commands. Use the CLI formatter backend for
/// those settings.
#[wasm_bindgen(js_name = loadConfigFromToml)]
pub fn load_config_from_toml(toml: &str) -> Result<JsValue, JsError> {
    let config = Config::from_toml(toml).map_err(|e| JsError::new(&e.to_string()))?;
    let mut warnings = Vec::new();

    if !config.code_block.formatters.is_empty() {
        warnings.push(
            "External code formatters in code_block.formatters are ignored by the WASM formatter. Use the Hongdown CLI backend to run them.".to_string(),
        );
    }

    let result = JsConfigResult {
        options: JsOptions::from(&config),
        warnings,
    };
    serde_wasm_bindgen::to_value(&result).map_err(|e| JsError::new(&e.to_string()))
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

/// Format Markdown with an optional code formatter callback.
///
/// # Arguments
///
/// * `input` - Markdown source to format
/// * `options` - Optional formatting options as a JavaScript object
/// * `code_formatter` - Optional JavaScript callback function `(language: string, code: string) => string | null`
///   that formats code blocks. Return the formatted code, or null/undefined to keep the original.
///
/// # Returns
///
/// An object with `output` (formatted string) and `warnings` (array of warning objects).
#[wasm_bindgen(js_name = formatWithCodeFormatter)]
pub fn format_with_code_formatter(
    input: &str,
    options: JsValue,
    code_formatter: Option<js_sys::Function>,
) -> Result<JsValue, JsError> {
    use comrak::{Arena, parse_document};

    let js_opts: JsOptions = if options.is_undefined() || options.is_null() {
        JsOptions::default()
    } else {
        serde_wasm_bindgen::from_value(options).map_err(|e| JsError::new(&e.to_string()))?
    };

    let opts = js_opts.to_options();

    if input.is_empty() {
        let js_result = JsFormatResult {
            output: String::new(),
            warnings: Vec::new(),
        };
        return serde_wasm_bindgen::to_value(&js_result).map_err(|e| JsError::new(&e.to_string()));
    }

    let arena = Arena::new();
    let comrak_options = crate::comrak_options(&opts);

    // In MDX mode, protect embedded JavaScript/JSX before parsing and restore it
    // afterwards (see [`crate::mdx`]).
    let protection = if opts.mdx {
        crate::mdx::protect(input, &comrak_options)
    } else {
        None
    };
    let source: &str = protection.as_ref().map_or(input, |p| p.source.as_str());

    let root = parse_document(&arena, source, &comrak_options);

    // Create callback closure if provided
    let callback: crate::serializer::CodeFormatterCallback = code_formatter.map(|func| {
        Box::new(move |language: &str, code: &str| -> Option<String> {
            let this = JsValue::null();
            let lang_js = JsValue::from_str(language);
            let code_js = JsValue::from_str(code);

            match func.call2(&this, &lang_js, &code_js) {
                Ok(result) => {
                    if result.is_null() || result.is_undefined() {
                        None
                    } else {
                        result.as_string()
                    }
                }
                Err(_) => None,
            }
        }) as Box<dyn Fn(&str, &str) -> Option<String>>
    });

    let replacements = protection
        .as_ref()
        .map_or_else(Vec::new, |p| p.reference_label_replacements());
    let result = crate::serializer::serialize_with_code_formatter(
        root,
        &opts,
        Some(source),
        callback,
        replacements,
    );

    let output = match &protection {
        Some(p) => p.restore(&result.output),
        None => result.output,
    };

    let js_result = JsFormatResult {
        output,
        warnings: result
            .warnings
            .into_iter()
            .map(|w| JsWarning {
                // Map protected-source line numbers back to the original.
                line: protection
                    .as_ref()
                    .map_or(w.line, |p| p.original_line(w.line)),
                message: protection
                    .as_ref()
                    .map_or(w.message.clone(), |p| p.restore(&w.message)),
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
        assert_eq!(opts.line_width.unwrap().get(), 80);
        assert!(opts.setext_h1);
        assert!(opts.setext_h2);
    }

    #[test]
    fn test_js_options_partial() {
        let js_opts = JsOptions {
            line_width: Some(JsLineWidthSetting::Width(100)),
            setext_h1: Some(false),
            ..Default::default()
        };
        let opts = js_opts.to_options();
        assert_eq!(opts.line_width.unwrap().get(), 100);
        assert!(!opts.setext_h1);
        assert!(opts.setext_h2); // default
    }

    #[test]
    fn test_js_options_no_wrap() {
        let js_opts = JsOptions {
            line_width: Some(JsLineWidthSetting::NoWrap(false)),
            ..Default::default()
        };
        let opts = js_opts.to_options();
        assert!(opts.line_width.is_none());
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
            DashSetting::Pattern(p) => assert_eq!(p.as_str(), "--"),
            _ => panic!("Expected Pattern"),
        }
    }

    #[test]
    fn test_js_options_heading_sentence_case() {
        let js_opts = JsOptions {
            heading_sentence_case: Some(true),
            ..Default::default()
        };
        let opts = js_opts.to_options();
        assert!(opts.heading_sentence_case);
    }

    #[test]
    fn test_js_options_heading_proper_nouns() {
        let js_opts = JsOptions {
            heading_proper_nouns: Some(vec!["MyApp".to_string(), "OpenAI".to_string()]),
            ..Default::default()
        };
        let opts = js_opts.to_options();
        assert_eq!(opts.heading_proper_nouns, vec!["MyApp", "OpenAI"]);
    }

    #[test]
    fn test_js_options_heading_common_nouns() {
        let js_opts = JsOptions {
            heading_common_nouns: Some(vec!["react".to_string()]),
            ..Default::default()
        };
        let opts = js_opts.to_options();
        assert_eq!(opts.heading_common_nouns, vec!["react"]);
    }

    #[test]
    fn test_js_options_heading_all() {
        let js_opts = JsOptions {
            heading_sentence_case: Some(true),
            heading_proper_nouns: Some(vec!["Fedify".to_string()]),
            heading_common_nouns: Some(vec!["api".to_string()]),
            ..Default::default()
        };
        let opts = js_opts.to_options();
        assert!(opts.heading_sentence_case);
        assert_eq!(opts.heading_proper_nouns, vec!["Fedify"]);
        assert_eq!(opts.heading_common_nouns, vec!["api"]);
    }
}
