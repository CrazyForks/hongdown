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

import type { FormatOptions, FormatResult } from "./types.js";
// @ts-expect-error: Subpath import resolved by Node.js/bundler
import { loadWasmBuffer } from "#wasm-loader";
import init, { format as wasmFormat, formatWithWarnings as wasmFormatWithWarnings } from "../pkg/hongdown.js";

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

// Re-export types
export type {
  FormatOptions,
  FormatResult,
  Warning,
  OrderedListPad,
  DashSetting,
} from "./types.js";
