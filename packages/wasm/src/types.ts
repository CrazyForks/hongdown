/**
 * Padding style for ordered list numbers.
 *
 * - `"start"`: Pad before the number (default): `  1.`, `  2.`, ..., ` 10.`
 * - `"end"`: Pad after the number: `1. `, `2. `, ..., `10.`
 */
export type OrderedListPad = "start" | "end";

/**
 * Dash transformation setting.
 *
 * - `false`: Disabled
 * - `string`: Pattern to transform to the dash character
 */
export type DashSetting = false | string;

/**
 * Formatting options for the Hongdown formatter.
 *
 * All options are optional. When not specified, default values are used.
 */
export interface FormatOptions {
  /**
   * Line width for wrapping.
   * @default 80
   */
  lineWidth?: number;

  /**
   * Use setext-style (underlined) for h1 headings.
   * @default true
   */
  setextH1?: boolean;

  /**
   * Use setext-style (underlined) for h2 headings.
   * @default true
   */
  setextH2?: boolean;

  /**
   * Convert headings to sentence case.
   * When enabled, headings like "Getting Started With HONGDOWN" become
   * "Getting started with Hongdown".
   * @default false
   */
  headingSentenceCase?: boolean;

  /**
   * Additional proper nouns to preserve in sentence case.
   * These are merged with built-in proper nouns (like "GitHub", "JavaScript").
   * @example ["MyApp", "OpenAI"]
   * @default []
   */
  headingProperNouns?: string[];

  /**
   * Words to treat as common nouns in sentence case.
   * These are excluded from built-in proper nouns.
   * @example ["react"]
   * @default []
   */
  headingCommonNouns?: string[];

  /**
   * Marker character for unordered lists: `"-"`, `"*"`, or `"+"`.
   * @default "-"
   */
  unorderedMarker?: string;

  /**
   * Number of leading spaces before the list marker.
   * @default 1
   */
  leadingSpaces?: number;

  /**
   * Number of trailing spaces after the list marker.
   * @default 2
   */
  trailingSpaces?: number;

  /**
   * Indentation width for nested list items.
   * @default 4
   */
  indentWidth?: number;

  /**
   * Marker style for ordered lists at odd nesting levels.
   * Use `"."` for `1.` or `")"` for `1)`.
   * @default "."
   */
  oddLevelMarker?: string;

  /**
   * Marker style for ordered lists at even nesting levels.
   * Use `"."` for `1.` or `")"` for `1)`.
   * @default ")"
   */
  evenLevelMarker?: string;

  /**
   * Padding style for ordered list numbers.
   * @default "start"
   */
  orderedListPad?: OrderedListPad;

  /**
   * Indentation width for nested ordered list items.
   * @default 4
   */
  orderedListIndentWidth?: number;

  /**
   * Fence character for code blocks: `"~"` or `` "`" ``.
   * @default "~"
   */
  fenceChar?: string;

  /**
   * Minimum fence length for code blocks.
   * @default 4
   */
  minFenceLength?: number;

  /**
   * Add space between fence and language identifier.
   * @default true
   */
  spaceAfterFence?: boolean;

  /**
   * Default language identifier for code blocks without one.
   * When empty, code blocks without a language identifier remain without one.
   * @default ""
   */
  defaultLanguage?: string;

  /**
   * The style string for thematic breaks.
   * @default "- - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -"
   */
  thematicBreakStyle?: string;

  /**
   * Number of leading spaces before thematic breaks (0-3).
   * @default 3
   */
  thematicBreakLeadingSpaces?: number;

  /**
   * Convert straight double quotes to curly quotes.
   * `"text"` becomes `"text"` (U+201C and U+201D).
   * @default true
   */
  curlyDoubleQuotes?: boolean;

  /**
   * Convert straight single quotes to curly quotes.
   * `'text'` becomes `'text'` (U+2018 and U+2019).
   * @default true
   */
  curlySingleQuotes?: boolean;

  /**
   * Convert straight apostrophes to curly apostrophes.
   * `it's` becomes `it's` (U+2019).
   * @default false
   */
  curlyApostrophes?: boolean;

  /**
   * Convert three dots to ellipsis character.
   * `...` becomes `…` (U+2026).
   * @default true
   */
  ellipsis?: boolean;

  /**
   * Convert a pattern to en-dash.
   * Set to a string like `"--"` to enable.
   * The pattern is replaced with `–` (U+2013).
   * @default false
   */
  enDash?: DashSetting;

  /**
   * Convert a pattern to em-dash.
   * Set to `false` to disable, or a string like `"---"` for a different pattern.
   * The pattern is replaced with `—` (U+2014).
   * @default "--"
   */
  emDash?: DashSetting;
}

/**
 * A warning generated during formatting.
 */
export interface Warning {
  /**
   * Line number where the warning was generated (1-indexed).
   */
  line: number;

  /**
   * Warning message.
   */
  message: string;
}

/**
 * Result of formatting with warnings.
 */
export interface FormatResult {
  /**
   * The formatted Markdown output.
   */
  output: string;

  /**
   * Warnings generated during formatting.
   */
  warnings: Warning[];
}
