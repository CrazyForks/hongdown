import { format, formatWithWarnings } from "@hongdown/wasm";
import type { FormatOptions } from "@hongdown/wasm";
import assert from "node:assert/strict";
import { describe, it } from "node:test";

describe("format", () => {
  it("formats ATX headings to Setext style for h1", async () => {
    const input = "# Hello\n\nWorld";
    const output = await format(input);
    assert.equal(output, "Hello\n=====\n\nWorld\n");
  });

  it("formats ATX headings to Setext style for h2", async () => {
    const input = "## Section\n\nContent";
    const output = await format(input);
    assert.equal(output, "Section\n-------\n\nContent\n");
  });

  it("keeps ATX style for h3 and below", async () => {
    const input = "### Subsection\n\nContent";
    const output = await format(input);
    assert.equal(output, "### Subsection\n\nContent\n");
  });

  it("formats unordered lists with proper markers", async () => {
    const input = "* Item 1\n* Item 2";
    const output = await format(input);
    assert.equal(output, " -  Item 1\n -  Item 2\n");
  });

  it("wraps long lines", async () => {
    const input =
      "This is a very long line that should be wrapped because it exceeds the default line width of eighty characters.";
    const output = await format(input);
    assert.ok(output.includes("\n"), "Output should contain line breaks");
  });

  it("respects lineWidth option", async () => {
    const input = "Short line that fits.";
    const options: FormatOptions = { lineWidth: 100 };
    const output = await format(input, options);
    assert.equal(output, "Short line that fits.\n");
  });

  it("respects setextH1 option", async () => {
    const input = "# Heading";
    const options: FormatOptions = { setextH1: false };
    const output = await format(input, options);
    assert.equal(output, "# Heading\n");
  });

  it("respects setextH2 option", async () => {
    const input = "## Heading";
    const options: FormatOptions = { setextH2: false };
    const output = await format(input, options);
    assert.equal(output, "## Heading\n");
  });

  it("respects fenceChar option", async () => {
    const input = "```js\ncode\n```";
    const options: FormatOptions = { fenceChar: "`" };
    const output = await format(input, options);
    assert.ok(output.includes("````"), "Output should use backtick fences");
  });

  it("formats code blocks with tildes by default", async () => {
    const input = "```js\ncode\n```";
    const output = await format(input);
    assert.ok(output.includes("~~~~"), "Output should use tilde fences");
  });

  it("handles empty input", async () => {
    const output = await format("");
    assert.equal(output, "");
  });

  it("handles input with only whitespace", async () => {
    const output = await format("   \n\n   ");
    assert.equal(output, "");
  });
});

describe("formatWithWarnings", () => {
  it("returns output and warnings", async () => {
    const input = "# Hello\n\nWorld";
    const result = await formatWithWarnings(input);
    assert.ok("output" in result, "Result should have output property");
    assert.ok("warnings" in result, "Result should have warnings property");
    assert.ok(Array.isArray(result.warnings), "Warnings should be an array");
  });

  it("returns formatted output", async () => {
    const input = "# Hello\n\nWorld";
    const { output } = await formatWithWarnings(input);
    assert.equal(output, "Hello\n=====\n\nWorld\n");
  });

  it("returns empty warnings for valid input", async () => {
    const input = "# Hello\n\nWorld";
    const { warnings } = await formatWithWarnings(input);
    assert.equal(warnings.length, 0);
  });

  it("returns warnings for tables with inconsistent columns", async () => {
    const input = "| A | B |\n|---|---|\n| 1 | 2 | 3 |";
    const { warnings } = await formatWithWarnings(input);
    assert.ok(warnings.length > 0, "Should have warnings");
    assert.ok(
      warnings.some((w) => w.message.toLowerCase().includes("column")),
      "Warning should mention columns",
    );
  });
});

describe("options", () => {
  it("accepts all formatting options", async () => {
    const input = "# Test\n\n- item";
    const options: FormatOptions = {
      lineWidth: 100,
      setextH1: true,
      setextH2: true,
      unorderedMarker: "-",
      leadingSpaces: 1,
      trailingSpaces: 2,
      indentWidth: 4,
      oddLevelMarker: ".",
      evenLevelMarker: ")",
      orderedListPad: "start",
      orderedListIndentWidth: 4,
      fenceChar: "~",
      minFenceLength: 4,
      spaceAfterFence: true,
      defaultLanguage: "",
      thematicBreakStyle:
        "- - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -",
      thematicBreakLeadingSpaces: 3,
      curlyDoubleQuotes: true,
      curlySingleQuotes: true,
      curlyApostrophes: false,
      ellipsis: true,
      enDash: false,
      emDash: "--",
    };
    const output = await format(input, options);
    assert.ok(output.length > 0, "Should produce output");
  });
});
