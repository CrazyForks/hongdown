// SPDX-FileCopyrightText: 2025 Hong Minhee <https://hongminhee.org/>
// SPDX-License-Identifier: GPL-3.0-or-later
//! Configuration file support for Hongdown.
//!
//! This module provides functionality for loading and parsing configuration
//! files (`.hongdown.toml`) that control the formatter's behavior.

use serde::Deserialize;
use std::path::{Path, PathBuf};

/// The default configuration file name.
pub const CONFIG_FILE_NAME: &str = ".hongdown.toml";

/// Configuration for the Hongdown formatter.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    /// Maximum line width for wrapping (default: 80).
    pub line_width: usize,

    /// Heading formatting options.
    pub heading: HeadingConfig,

    /// Unordered list formatting options.
    pub list: ListConfig,

    /// Ordered list formatting options.
    pub ordered_list: OrderedListConfig,

    /// Code block formatting options.
    pub code_block: CodeBlockConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            line_width: 80,
            heading: HeadingConfig::default(),
            list: ListConfig::default(),
            ordered_list: OrderedListConfig::default(),
            code_block: CodeBlockConfig::default(),
        }
    }
}

/// Heading formatting options.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(default)]
pub struct HeadingConfig {
    /// Use `===` underline for h1 (default: true).
    pub setext_h1: bool,

    /// Use `---` underline for h2 (default: true).
    pub setext_h2: bool,
}

impl Default for HeadingConfig {
    fn default() -> Self {
        Self {
            setext_h1: true,
            setext_h2: true,
        }
    }
}

/// Unordered list formatting options.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(default)]
pub struct ListConfig {
    /// Marker character: `-`, `*`, or `+` (default: `-`).
    pub unordered_marker: char,

    /// Spaces before the marker (default: 1).
    pub leading_spaces: usize,

    /// Spaces after the marker (default: 2).
    pub trailing_spaces: usize,

    /// Indentation width for nested items (default: 4).
    pub indent_width: usize,
}

impl Default for ListConfig {
    fn default() -> Self {
        Self {
            unordered_marker: '-',
            leading_spaces: 1,
            trailing_spaces: 2,
            indent_width: 4,
        }
    }
}

/// Ordered list formatting options.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(default)]
pub struct OrderedListConfig {
    /// Marker style at odd nesting levels: `.` for `1.` (default: `.`).
    pub odd_level_marker: char,

    /// Marker style at even nesting levels: `)` for `1)` (default: `)`).
    pub even_level_marker: char,
}

impl Default for OrderedListConfig {
    fn default() -> Self {
        Self {
            odd_level_marker: '.',
            even_level_marker: ')',
        }
    }
}

/// Code block formatting options.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(default)]
pub struct CodeBlockConfig {
    /// Fence character: `~` or `` ` `` (default: `~`).
    pub fence_char: char,

    /// Minimum fence length (default: 4).
    pub min_fence_length: usize,

    /// Add space between fence and language identifier (default: true).
    pub space_after_fence: bool,
}

impl Default for CodeBlockConfig {
    fn default() -> Self {
        Self {
            fence_char: '~',
            min_fence_length: 4,
            space_after_fence: true,
        }
    }
}

impl Config {
    /// Parse a configuration from a TOML string.
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    /// Load configuration from a file.
    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| ConfigError::Io(path.to_path_buf(), e))?;
        Self::from_toml(&content).map_err(|e| ConfigError::Parse(path.to_path_buf(), e))
    }

    /// Discover and load configuration by searching up the directory tree.
    ///
    /// Starting from `start_dir`, searches for `.hongdown.toml` in each parent
    /// directory until the filesystem root is reached. Returns `None` if no
    /// configuration file is found.
    pub fn discover(start_dir: &Path) -> Result<Option<(PathBuf, Self)>, ConfigError> {
        let mut current = start_dir.to_path_buf();
        loop {
            let config_path = current.join(CONFIG_FILE_NAME);
            if config_path.exists() {
                let config = Self::from_file(&config_path)?;
                return Ok(Some((config_path, config)));
            }
            if !current.pop() {
                break;
            }
        }
        Ok(None)
    }
}

/// Errors that can occur when loading configuration.
#[derive(Debug)]
pub enum ConfigError {
    /// I/O error reading the configuration file.
    Io(PathBuf, std::io::Error),
    /// Error parsing the TOML configuration.
    Parse(PathBuf, toml::de::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(path, err) => {
                write!(f, "failed to read {}: {}", path.display(), err)
            }
            ConfigError::Parse(path, err) => {
                write!(f, "failed to parse {}: {}", path.display(), err)
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::Io(_, err) => Some(err),
            ConfigError::Parse(_, err) => Some(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.line_width, 80);
        assert!(config.heading.setext_h1);
        assert!(config.heading.setext_h2);
        assert_eq!(config.list.unordered_marker, '-');
        assert_eq!(config.list.leading_spaces, 1);
        assert_eq!(config.list.trailing_spaces, 2);
        assert_eq!(config.list.indent_width, 4);
        assert_eq!(config.ordered_list.odd_level_marker, '.');
        assert_eq!(config.ordered_list.even_level_marker, ')');
        assert_eq!(config.code_block.fence_char, '~');
        assert_eq!(config.code_block.min_fence_length, 4);
        assert!(config.code_block.space_after_fence);
    }

    #[test]
    fn test_parse_empty_toml() {
        let config = Config::from_toml("").unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn test_parse_line_width() {
        let config = Config::from_toml("line_width = 100").unwrap();
        assert_eq!(config.line_width, 100);
    }

    #[test]
    fn test_parse_heading_config() {
        let config = Config::from_toml(
            r#"
[heading]
setext_h1 = false
setext_h2 = false
"#,
        )
        .unwrap();
        assert!(!config.heading.setext_h1);
        assert!(!config.heading.setext_h2);
    }

    #[test]
    fn test_parse_list_config() {
        let config = Config::from_toml(
            r#"
[list]
unordered_marker = "*"
leading_spaces = 0
trailing_spaces = 1
indent_width = 2
"#,
        )
        .unwrap();
        assert_eq!(config.list.unordered_marker, '*');
        assert_eq!(config.list.leading_spaces, 0);
        assert_eq!(config.list.trailing_spaces, 1);
        assert_eq!(config.list.indent_width, 2);
    }

    #[test]
    fn test_parse_ordered_list_config() {
        let config = Config::from_toml(
            r#"
[ordered_list]
odd_level_marker = ")"
even_level_marker = "."
"#,
        )
        .unwrap();
        assert_eq!(config.ordered_list.odd_level_marker, ')');
        assert_eq!(config.ordered_list.even_level_marker, '.');
    }

    #[test]
    fn test_parse_code_block_config() {
        let config = Config::from_toml(
            r#"
[code_block]
fence_char = "`"
min_fence_length = 3
space_after_fence = false
"#,
        )
        .unwrap();
        assert_eq!(config.code_block.fence_char, '`');
        assert_eq!(config.code_block.min_fence_length, 3);
        assert!(!config.code_block.space_after_fence);
    }

    #[test]
    fn test_parse_full_config() {
        let config = Config::from_toml(
            r#"
line_width = 80

[heading]
setext_h1 = true
setext_h2 = true

[list]
unordered_marker = "-"
leading_spaces = 1
trailing_spaces = 2
indent_width = 4

[ordered_list]
odd_level_marker = "."
even_level_marker = ")"

[code_block]
fence_char = "~"
min_fence_length = 4
space_after_fence = true
"#,
        )
        .unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn test_parse_invalid_toml() {
        let result = Config::from_toml("line_width = \"not a number\"");
        assert!(result.is_err());
    }

    #[test]
    fn test_discover_no_config() {
        let temp_dir = std::env::temp_dir().join("hongdown_test_no_config");
        let _ = std::fs::create_dir_all(&temp_dir);
        let result = Config::discover(&temp_dir).unwrap();
        assert!(result.is_none());
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_discover_config_in_current_dir() {
        let temp_dir = std::env::temp_dir().join("hongdown_test_current");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();
        let config_path = temp_dir.join(CONFIG_FILE_NAME);
        std::fs::write(&config_path, "line_width = 120").unwrap();

        let result = Config::discover(&temp_dir).unwrap();
        assert!(result.is_some());
        let (path, config) = result.unwrap();
        assert_eq!(path, config_path);
        assert_eq!(config.line_width, 120);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_discover_config_in_parent_dir() {
        let temp_dir = std::env::temp_dir().join("hongdown_test_parent");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let sub_dir = temp_dir.join("subdir").join("nested");
        std::fs::create_dir_all(&sub_dir).unwrap();
        let config_path = temp_dir.join(CONFIG_FILE_NAME);
        std::fs::write(&config_path, "line_width = 90").unwrap();

        let result = Config::discover(&sub_dir).unwrap();
        assert!(result.is_some());
        let (path, config) = result.unwrap();
        assert_eq!(path, config_path);
        assert_eq!(config.line_width, 90);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
