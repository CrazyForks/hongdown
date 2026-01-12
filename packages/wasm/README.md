@hongdown/wasm
==============

A WebAssembly-based Markdown formatter library that enforces
[Hong Minhee's Markdown style conventions].

This package provides the same formatting as the [Hongdown CLI], but as a
JavaScript/TypeScript library that works in Node.js, Bun, Deno, and web
browsers.

[Hong Minhee's Markdown style conventions]: https://github.com/dahlia/hongdown/blob/main/STYLE.md
[Hongdown CLI]: https://www.npmjs.com/package/hongdown


Installation
------------

~~~~ bash
npm install @hongdown/wasm
~~~~


Usage
-----

~~~~ typescript
import { format, formatWithWarnings } from "@hongdown/wasm";

// Basic usage
const markdown = "# Hello\nWorld";
const formatted = await format(markdown);

// With options
const result = await format(markdown, {
  lineWidth: 100,
  setextH1: false,
  fenceChar: "`",
});

// Get warnings along with formatted output
const { output, warnings } = await formatWithWarnings(markdown);
if (warnings.length > 0) {
  for (const warning of warnings) {
    console.warn(`Line ${warning.line}: ${warning.message}`);
  }
}
~~~~


Options
-------

All options are optional.  See the [TypeScript type definitions] for the
complete list of available options.

[TypeScript type definitions]: https://github.com/dahlia/hongdown/blob/main/packages/wasm/src/types.ts


License
-------

Distributed under the [GPL-3.0-or-later].

[GPL-3.0-or-later]: https://www.gnu.org/licenses/gpl-3.0.html
