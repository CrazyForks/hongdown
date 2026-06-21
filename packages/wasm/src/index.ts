/**
 * Hongdown WASM Library
 *
 * A Markdown formatter that enforces Hong Minhee's Markdown style conventions.
 *
 * @example
 * ```typescript
 * import { format } from "@hongdown/wasm";
 *
 * const markdown = "# Hello\nWorld";
 * const formatted = await format(markdown);
 * ```
 *
 * @packageDocumentation
 */

import type {
  ConfigLoadResult,
  FormatOptions,
  FormatResult,
  FormatWithCodeFormatterOptions,
} from "./types.js";
// @ts-expect-error: Subpath import resolved by Node.js/bundler
import { loadWasmBuffer } from "#wasm-loader";
import init, {
  format as wasmFormat,
  loadConfigFromToml as wasmLoadConfigFromToml,
  formatWithWarnings as wasmFormatWithWarnings,
  formatWithCodeFormatter as wasmFormatWithCodeFormatter,
} from "../pkg/hongdown.js";

// Lazily initialized
let initialized = false;
let initPromise: Promise<void> | null = null;

/**
 * Ensure the WASM module is initialized.
 * @internal
 */
async function ensureInitialized(): Promise<void> {
  if (initialized) {
    return;
  }

  if (!initPromise) {
    initPromise = (async () => {
      const buffer = await loadWasmBuffer();
      await init({ module_or_path: buffer });
      initialized = true;
    })();
  }

  await initPromise;
}

/**
 * Format Markdown according to Hong Minhee's style conventions.
 *
 * This function supports formatting directives embedded in HTML comments:
 *
 * - `<!-- hongdown-disable-file -->` - Disable formatting for the entire file.
 * - `<!-- hongdown-disable-next-line -->` - Disable formatting for the next block.
 * - `<!-- hongdown-disable-next-section -->` - Disable formatting until the next
 *   section heading.
 * - `<!-- hongdown-disable -->` - Disable formatting from this point.
 * - `<!-- hongdown-enable -->` - Re-enable formatting.
 *
 * @param input - Markdown source to format
 * @param options - Formatting options (all optional)
 * @returns The formatted Markdown string
 *
 * @example
 * ```typescript
 * import { format } from "@hongdown/wasm";
 *
 * // Basic usage
 * const result = await format("# Hello\nWorld");
 *
 * // With options
 * const result = await format(markdown, {
 *   lineWidth: 100,
 *   setextH1: false,
 *   fenceChar: "`",
 * });
 * ```
 */
export async function format(
  input: string,
  options: FormatOptions = {},
): Promise<string> {
  await ensureInitialized();
  return wasmFormat(input, options);
}

/**
 * Parse a `.hongdown.toml` configuration string into JavaScript options.
 *
 * External code block formatters are reported as warnings because the WASM
 * runtime cannot execute external commands. Use the Hongdown CLI backend for
 * those settings.
 *
 * @param toml - TOML configuration source
 * @returns Formatting options and any warnings
 */
export async function loadConfigFromToml(
  toml: string,
): Promise<ConfigLoadResult> {
  await ensureInitialized();
  return wasmLoadConfigFromToml(toml) as ConfigLoadResult;
}

/**
 * Format Markdown and return both output and warnings.
 *
 * This is similar to {@link format}, but also returns any warnings generated
 * during formatting (e.g., inconsistent table column counts).
 *
 * @param input - Markdown source to format
 * @param options - Formatting options (all optional)
 * @returns Object with formatted output and any warnings
 *
 * @example
 * ```typescript
 * import { formatWithWarnings } from "@hongdown/wasm";
 *
 * const { output, warnings } = await formatWithWarnings(markdown);
 *
 * if (warnings.length > 0) {
 *   for (const warning of warnings) {
 *     console.warn(`Line ${warning.line}: ${warning.message}`);
 *   }
 * }
 * ```
 */
export async function formatWithWarnings(
  input: string,
  options: FormatOptions = {},
): Promise<FormatResult> {
  await ensureInitialized();
  return wasmFormatWithWarnings(input, options) as FormatResult;
}

/**
 * Format Markdown with an optional code formatter callback.
 *
 * This function allows you to provide a callback that formats code blocks
 * using external formatters (e.g., Prettier, ESLint).
 *
 * @param input - Markdown source to format
 * @param options - Formatting options with optional code formatter callback
 * @returns Object with formatted output and any warnings
 *
 * @example
 * ```typescript
 * import { formatWithCodeFormatter } from "@hongdown/wasm";
 * import * as prettier from "prettier";
 *
 * const { output, warnings } = await formatWithCodeFormatter(markdown, {
 *   codeFormatter: (language, code) => {
 *     if (language === "javascript" || language === "typescript") {
 *       return prettier.format(code, { parser: "babel" });
 *     }
 *     return null; // Keep original for other languages
 *   },
 * });
 * ```
 */
export async function formatWithCodeFormatter(
  input: string,
  options: FormatWithCodeFormatterOptions = {},
): Promise<FormatResult> {
  await ensureInitialized();
  const { codeFormatter, ...restOptions } = options;
  return wasmFormatWithCodeFormatter(
    input,
    restOptions,
    codeFormatter ?? null,
  ) as FormatResult;
}

// Re-export types
export type {
  CodeFormatterCallback,
  ConfigLoadResult,
  FormatOptions,
  FormatResult,
  FormatWithCodeFormatterOptions,
  Warning,
  OrderedListPad,
  DashSetting,
} from "./types.js";
