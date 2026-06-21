import {
  format,
  formatWithWarnings,
  formatWithCodeFormatter,
  loadConfigFromToml,
} from "@hongdown/wasm";
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

  it("respects headingSentenceCase option", async () => {
    const input = "# Getting Started With The App";
    const options: FormatOptions = { headingSentenceCase: true };
    const output = await format(input, options);
    // "Getting" stays capitalized (first word), others become lowercase
    assert.equal(output, "Getting started with the app\n============================\n");
  });

  it("respects headingProperNouns option", async () => {
    const input = "# Using MyApp With JavaScript";
    const options: FormatOptions = {
      headingSentenceCase: true,
      headingProperNouns: ["MyApp"],
    };
    const output = await format(input, options);
    assert.ok(
      output.includes("MyApp"),
      "Output should preserve custom proper noun",
    );
    assert.ok(
      output.includes("JavaScript"),
      "Output should preserve built-in proper noun",
    );
  });

  it("respects headingCommonNouns option", async () => {
    const input = "# Using React Components";
    const options: FormatOptions = {
      headingSentenceCase: true,
      headingCommonNouns: ["React"],
    };
    const output = await format(input, options);
    // "React" is a built-in proper noun, but headingCommonNouns should exclude it
    assert.ok(
      output.includes("react"),
      "Output should lowercase excluded proper noun",
    );
  });

  it("respects headingAnchorAlign option", async () => {
    const input = "# Heading {#id}";
    const output = await format(input, {
      headingAnchorAlign: 1,
    });
    assert.equal(output, "Heading {#id}\n=============\n");
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

describe("formatWithCodeFormatter", () => {
  it("returns output and warnings", async () => {
    const input = "# Hello\n\n~~~~ js\ncode\n~~~~\n";
    const result = await formatWithCodeFormatter(input);
    assert.ok("output" in result, "Result should have output property");
    assert.ok("warnings" in result, "Result should have warnings property");
    assert.ok(Array.isArray(result.warnings), "Warnings should be an array");
  });

  it("formats code blocks with callback", async () => {
    const input = "# Test\n\n~~~~ js\nhello\n~~~~\n";
    const { output } = await formatWithCodeFormatter(input, {
      codeFormatter: (language: string, code: string) => {
        if (language === "js") {
          return code.toUpperCase();
        }
        return null;
      },
    });
    assert.ok(output.includes("HELLO"), "Code should be transformed");
  });

  it("preserves original code when callback returns null", async () => {
    const input = "# Test\n\n~~~~ python\nhello\n~~~~\n";
    const { output } = await formatWithCodeFormatter(input, {
      codeFormatter: (language: string, _code: string) => {
        if (language === "js") {
          return "transformed";
        }
        return null; // Don't transform Python
      },
    });
    assert.ok(output.includes("hello"), "Original code should be preserved");
    assert.ok(
      !output.includes("transformed"),
      "Code should not be transformed",
    );
  });

  it("works without callback", async () => {
    const input = "# Test\n\n~~~~ js\ncode\n~~~~\n";
    const { output } = await formatWithCodeFormatter(input);
    assert.ok(output.includes("code"), "Code should be preserved");
  });

  it("passes correct language to callback", async () => {
    const input = "~~~~ typescript\ncode\n~~~~\n";
    let receivedLanguage = "";
    await formatWithCodeFormatter(input, {
      codeFormatter: (language: string, _code: string) => {
        receivedLanguage = language;
        return null;
      },
    });
    assert.equal(receivedLanguage, "typescript", "Language should match");
  });

  it("passes correct code to callback", async () => {
    const input = "~~~~ js\nconst x = 1;\n~~~~\n";
    let receivedCode = "";
    await formatWithCodeFormatter(input, {
      codeFormatter: (_language: string, code: string) => {
        receivedCode = code;
        return null;
      },
    });
    assert.equal(receivedCode, "const x = 1;\n", "Code should match");
  });

  it("accepts formatting options alongside callback", async () => {
    const input = "# Test\n\n~~~~ js\ncode\n~~~~\n";
    const { output } = await formatWithCodeFormatter(input, {
      setextH1: false,
      codeFormatter: (language: string, code: string) => {
        if (language === "js") {
          return code.toUpperCase();
        }
        return null;
      },
    });
    assert.ok(output.startsWith("# Test"), "H1 should be ATX style");
    assert.ok(output.includes("CODE"), "Code should be transformed");
  });
});

describe("options", () => {
  it("accepts all formatting options", async () => {
    const input = "# Test\n\n- item";
    const options: FormatOptions = {
      lineWidth: 100,
      setextH1: true,
      setextH2: true,
      headingSentenceCase: false,
      headingProperNouns: [],
      headingCommonNouns: [],
      headingAnchorAlign: 0,
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

describe("loadConfigFromToml", () => {
  it("converts TOML config to formatting options", async () => {
    const { options, warnings } = await loadConfigFromToml(`
line_width = 100
math = false
mdx = true

[heading]
setext_h1 = false
anchor_align = 1

[code_block]
fence_char = "\`"
default_language = "text"

[punctuation]
en_dash = "--"
em_dash = false
`);

    assert.deepEqual(warnings, []);
    assert.equal(options.lineWidth, 100);
    assert.equal(options.math, false);
    assert.equal(options.mdx, true);
    assert.equal(options.setextH1, false);
    assert.equal(options.headingAnchorAlign, 1);
    assert.equal(options.fenceChar, "`");
    assert.equal(options.defaultLanguage, "text");
    assert.equal(options.enDash, "--");
    assert.equal(options.emDash, false);
  });

  it("warns about external code formatters in WASM config", async () => {
    const { options, warnings } = await loadConfigFromToml(`
[code_block.formatters]
javascript = ["prettier", "--parser", "babel"]
`);

    assert.equal(options.fenceChar, "~");
    assert.equal(warnings.length, 1);
    assert.match(warnings[0], /external code formatters/i);
  });
});
