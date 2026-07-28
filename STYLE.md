Hong Minhee's Markdown style convention
=======================================

This document describes the Markdown style convention enforced by Hongdown.


Philosophy
----------

The core principle of this style is:

> *Markdown should be readable as plain text, not just after rendering.*

A well-formatted Markdown document should convey its structure and emphasis
clearly even when viewed in a plain text editor without any rendering.
You shouldn't need to render the document to HTML to understand its formatting
and hierarchy.

This philosophy leads to several practical implications:

 -  *Visual structure in source*: Headings, lists, and sections should be
    visually distinct in the raw text.
 -  *Consistent spacing*: Predictable whitespace patterns help readers scan
    the document structure.
 -  *Minimal escaping*: Choose delimiter styles that minimize the need for
    escape characters.
 -  *Reference-style links*: Keep prose readable by moving URLs out of the
    text flow.

This style prioritizes reading over writing.  Many rules are tedious to follow
manually—and that's intentional.  The assumption is that you'll use an
automated formatter like [Hongdown] to handle the mechanical work, freeing you
to focus on content rather than formatting details.

[Hongdown]: https://github.com/dahlia/hongdown


Headings
--------

### Setext-style for top-level headings

Use Setext-style (underlined) headings for document titles (H1) and major
sections (H2):

~~~~ markdown
Document Title
==============

Section Name
------------
~~~~

*Rationale*: Setext headings create strong visual separation in plain text.
The underline makes the heading immediately recognizable without needing to
count `#` characters.

### ATX-style for subsections

Use ATX-style (`###`, `####`, etc.) for subsections within a section:

~~~~ markdown
### Subsection

#### Sub-subsection
~~~~

*Rationale*: ATX-style is more compact for deeper nesting levels where
Setext-style would be awkward.

### Sentence case

Use sentence case for headings (capitalize only the first word and proper
nouns):

~~~~ markdown
Development commands    ← Correct
Development Commands    ← Incorrect
~~~~

When a heading ends with an explicit anchor (`{#name}`), preserve the anchor
name exactly as written.  Apply sentence case only to the visible heading text:

~~~~ markdown
Test section {#myAPI}    ← Correct
Test section {#myapi}    ← Incorrect
~~~~

*Rationale*: Sentence case is easier to read and more natural in technical
documentation.

### Explicit anchor alignment

When a heading ends with an explicit anchor (`{#name}`), right-align the
anchor to the configured line width (80 columns by default):

~~~~ markdown
Introduction                                                            {#intro}
================================================================================
~~~~

The Setext-style underline extends to cover the full heading line, including
the anchor.

If the heading body is already too wide to right-align the anchor within the
line width, or if word wrapping is disabled (`line_width = false`), a single
space is used instead:

~~~~ markdown
A very long heading that cannot be right-aligned {#long}
========================================================
~~~~

*Rationale*: Right-aligned anchors create a clean, visually consistent right
margin and make anchor identifiers easy to find when scanning a document in
plain text.

### Underline length

The underline of a Setext-style heading should match the display width of
the heading text, accounting for East Asian wide characters.

#### East Asian character width

East Asian wide characters (CJK characters) are counted as two columns when
calculating the display width.


Emphasis
--------

### Asterisks for emphasis

Use asterisks (`*`) for emphasis by default:

~~~~ markdown
This is *emphasized* text.
This is **strongly emphasized** text.
~~~~

### Underscores when content contains asterisks

When the emphasized content contains asterisk characters, use underscores
to avoid escaping:

~~~~ markdown
The file _*.txt_ matches all text files.
The pattern __**/*.md__ matches recursively.
~~~~

*Rationale*: This produces cleaner source text by avoiding backslash escapes.

### Escape all underscores in regular text

Underscores in regular text are always escaped, even in the middle of words:

~~~~ markdown
Use the CONFIG\_FILE\_NAME constant.
~~~~

*Rationale*: While CommonMark doesn't treat intraword underscores as emphasis
delimiters, escaping ensures consistent rendering across all Markdown parsers.


Backslash escapes
-----------------

### Preserve explicit escapes for ASCII punctuation

When plain text includes backslash-escaped ASCII punctuation, preserve those
escapes as written:

~~~~ markdown
Path: \[identifier\]
Literal braces: \{json\}
~~~~

*Rationale*: CommonMark allows backslash escapes for ASCII punctuation.
Preserving explicit escapes keeps formatting idempotent and avoids changing
rendered output by introducing visible backslashes.


Lists
-----

### Unordered list markers

Use ` -  ` (space, hyphen, two spaces) for unordered list items:

~~~~ markdown
 -  First item
 -  Second item
 -  Third item
~~~~

*Rationale*: The leading space creates visual indentation from the left margin.
The two trailing spaces align the text with a 4-space tab stop, making
continuation lines easy to align.

### Nested lists

Indent nested items by 4 spaces:

~~~~ markdown
 -  Parent item
     -  Child item
     -  Another child
 -  Another parent
~~~~

### Ordered list markers

Use `.` for odd nesting levels and `)` for even nesting levels:

~~~~ markdown
1.  First item
2.  Second item
    1)  Nested first
    2)  Nested second
3.  Third item
~~~~

*Rationale*: Alternating markers make the nesting level visually apparent.

### Fixed marker width

Ordered list markers maintain a fixed 4-character width.  When numbers grow
longer, trailing spaces are reduced (minimum 1 space):

~~~~ markdown
1.  First item
2.  Second item
...
9.  Ninth item
10. Tenth item
~~~~

*Rationale*: Consistent marker width keeps continuation lines aligned at
the same column regardless of item count.

### Continuation lines

Align continuation lines with the start of the item text:

~~~~ markdown
 -  This is a list item with text that continues
    on the next line with proper alignment.
~~~~

### Task lists

Task list items use checkboxes (`[ ]` for unchecked, `[x]` for checked) after
the list marker:

~~~~ markdown
 -  [ ] Unchecked task
 -  [x] Completed task
~~~~

*Rationale*: Task lists follow the same spacing rules as regular unordered
lists, keeping the document consistent.


Code
----

### Fenced code blocks with tildes

Use four tildes (`~~~~`) for fenced code blocks:

~~~~~ markdown
~~~~ rust
fn main() {
    println!("Hello, world!");
}
~~~~
~~~~~

*Rationale*: Tildes are visually distinct from the code content, which often
contains backticks for string literals or shell commands.

### Language identifiers

Always specify a language identifier for syntax highlighting.  If no specific
language applies, the identifier can be omitted:

~~~~~ markdown
~~~~ javascript
console.log("Hello");
~~~~
~~~~~

Additional metadata in the info string is preserved as written, including
HTML entities:

~~~~~ markdown
~~~~ c++ title=&quot;main.cpp&quot;
int main() {}
~~~~
~~~~~

### Inline code spans

Use backticks for inline code.  When the content contains backticks, use
multiple backticks as delimiters:

~~~~ markdown
Use the `format()` function.
The syntax is `` `code` `` with backticks.
~~~~

Preserve original spacing in code spans.  If the original source has space
padding (`` ` code ` ``), it is preserved in the output.

### Mathematical expressions

Recognize and preserve TeX/LaTeX math expressions verbatim.  Both inline math
(`$…$`) and display math (`$$…$$`) are emitted exactly as written — they are
never escaped or transformed, just like code spans:

~~~~ markdown
The complexity is $O(\text{some text})$ in the worst case.

$$
x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}
$$
~~~~

In particular, backslashes inside a formula are kept as-is (for example,
`$O(\text{x})$` is not rewritten to `$O(\\text{x})$`), and inline math
containing spaces is never broken across lines when wrapping.

Dollar-math parsing follows the same heuristics GitHub uses, so a lone `$`, a
shell prompt such as `$ command`, or a price like `$5` is treated as ordinary
text rather than math.

This behaviour is controlled by the `math` option (enabled by default); set
`math = false` to treat every `$` as literal text.

*Rationale*: Math expressions are not Markdown and must round-trip byte-for-byte
to stay syntactically valid; escaping or reflowing them would corrupt the
rendered output.


Links
-----

### Reference-style for external URLs

Convert external URLs to reference-style links, with each definition placed at
the end of the first section that uses it:

~~~~ markdown
See the [documentation] for more details.

Read the [installation guide] before proceeding.

[documentation]: https://example.com/docs
[installation guide]: https://example.com/install
~~~~

When several links share a definition, keep it in the first section that uses
it.  A link inside a footnote belongs to the section where the footnote is
referenced, regardless of where its definition appears in the source.  If body
content and one of its footnotes share a definition in the same section, place
the reference definition after the footnote definition.

*Rationale*: Reference-style links keep the prose readable by moving long URLs
out of the text flow.  Placing definitions at section end keeps related content
together.

### Angle brackets for reference destinations

Wrap a reference destination in angle brackets when the bare form would be
empty, ambiguous, or invalid.  This includes destinations containing ASCII
whitespace or control characters, destinations beginning with `<`, and
destinations containing backslashes or text that would be reparsed as a
character reference.  Also use angle brackets for unbalanced parentheses or
more than 32 levels of parenthesis nesting.

~~~~ markdown
[space]: <https://example.com/a b>
[tab]: <https://example.com/a&#9;b>
[empty]: <>
~~~~

Within such a destination, backslash-escape literal angle brackets and
backslashes.  Write ampersands as `&amp;`, and write tabs and line endings as
numeric character references.

*Rationale*: Without the angle brackets, whitespace ends the destination and an
empty destination is not recognized at all.  Keeping these destinations
angle-bracketed preserves their targets and makes formatting idempotent.

### Inline style for links containing images

Keep an inline link inline when its text contains an image, even if its
destination is external.  This includes images nested inside emphasis or strong
spans:

~~~~ markdown
[**![Build status](https://example.com/status.svg)**](https://example.com/build)
~~~~

*Rationale*: Image syntax contains brackets, which are not allowed in a
reference label.  Turning the complete link text into a label would produce an
invalid reference and leave the outer link unresolved.

### Inline style for relative URLs

Keep relative URLs and fragment links inline:

~~~~ markdown
See *[Chapter 2](./chapter2.md)* for more details.
Jump to the [installation section](#installation).
~~~~

*Rationale*: For inter-document links, the filename itself serves as a natural
identifier.  Using reference-style would create redundancy:

~~~~ markdown
See also *[Chapter 2]* for more details.

[Chapter 2]: ./chapter2.md
~~~~

The reference definition just repeats what the link text already conveys.

### Shortcut references when text matches label

When the link text matches the reference label, use shortcut reference syntax:

~~~~ markdown
See [GitHub] for the source code.

[GitHub]: https://github.com/example/repo
~~~~

### Collapsed references before brackets

When a shortcut reference would be immediately followed by text starting with
`[` (such as a footnote reference), use collapsed reference syntax `[text][]`
instead of shortcut syntax `[text]` to avoid ambiguity:

~~~~ markdown
See [GitHub][][^1] for details.

[GitHub]: https://github.com/example/repo

[^1]: Footnote text.
~~~~

*Rationale*: Without the empty brackets, `[GitHub][^1]` could be parsed as a
full reference link with label `^1`, which would break the intended link and
footnote.

### Distinct labels for distinct targets

A reference label may be shared only by links that point at the very same
target, meaning the same URL *and* the same title.  When the label a link
would otherwise take is already used for a different target, the link gets a
numbered label and full reference syntax:

~~~~ markdown
 -  Read [guide].
 -  Read [guide][guide 2].

[guide]: https://example.com/first
[guide 2]: https://example.com/second
~~~~

Labels are compared the way CommonMark resolves them: CommonMark ASCII
whitespace is trimmed from the edges, remaining whitespace runs are collapsed,
and Unicode case folding is applied.  So `[Guide]` and `[guide]` are the same
label, as are `[Straße]` and `[STRASSE]`, and one of them is renumbered unless
both point at the same target.  Non-ASCII whitespace at a label edge remains
significant.

When an inline external link is converted to reference style, runs of CommonMark
ASCII whitespace in ordinary text are collapsed to one space before its
displayed text and label are emitted.  Whitespace-sensitive inline constructs,
including code spans, math, and inline HTML, keep their displayed content; their
generated labels are normalized separately.  Significant edge whitespace is
retained in both so that the reference resolves through the same key.

For numbered label allocation, a label appearing as unresolved reference
syntax is also considered occupied.  Allocation skips it rather than adding a
definition that would silently turn the bracketed text into a link.

*Rationale*: A reference definition is document-wide, and a duplicate
definition is ignored by the parser rather than merged.  Sharing a label
between different targets would therefore silently change where one of the
links points, and formatting must never do that.


Block quotes and alerts
-----------------------

### Block quote continuation

Continue block quotes with `>` on each line:

~~~~ markdown
> This is a block quote that spans
> multiple lines of text.
~~~~

Use `>` on blank lines inside block quotes, including blank lines between
loose list items:

~~~~ markdown
>  -  First item.
>
>  -  Second item.
~~~~

### GitHub-style alerts

Use GitHub-flavored alert syntax for callouts:

~~~~ markdown
> [!NOTE]
> This is a note with additional information.

> [!WARNING]
> This action cannot be undone.
~~~~

Supported alert types: `NOTE`, `TIP`, `IMPORTANT`, `WARNING`, `CAUTION`.


Tables
------

### Pipe table formatting

Use pipe tables with proper column alignment:

~~~~ markdown
| Name    | Description                    |
| ------- | ------------------------------ |
| foo     | The foo component              |
| bar     | The bar component              |
~~~~

### Column width

Columns are padded to align pipes vertically.  East Asian wide characters
are counted as two columns for proper alignment.

### Escaped pipes in content

Pipe characters within cell content are escaped:

~~~~ markdown
| Pattern   | Meaning          |
| --------- | ---------------- |
| `a \| b`  | a or b           |
~~~~


Thematic breaks
---------------

### Dashes with spaces

Thematic breaks (horizontal rules) are rendered as a long line of spaced dashes
(37 dashes) with 3 leading spaces:

~~~~ markdown
Previous section content.

   - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

New section content.
~~~~

*Rationale*: The extended line of dashes creates a visually prominent separator
that resembles a traditional horizontal rule, making section breaks immediately
apparent when scanning the plain text source.


Line wrapping
-------------

### Wrap at 80 characters

Wrap prose at approximately 80 display columns:

~~~~ markdown
This is a paragraph that demonstrates line wrapping.  When text exceeds the
80-character limit, it wraps to the next line while preserving word boundaries.
~~~~

*Rationale*: 80 characters is a widely supported terminal width that ensures
readability across different editors and viewers.

### East Asian character width

East Asian wide characters (CJK characters) are counted as two columns when
calculating line width.

### Visible prefixes count toward width

Visible structural prefixes also count toward the line width on the first line.
This includes list markers, task-list checkboxes, block quote markers, and
definition-list markers:

~~~~ markdown
 -  This list item also wraps at the configured line width,
    including the ` -  ` prefix on the first line.
~~~~

### Preserve intentional short lines

Lines that are intentionally short in the source (well under the limit) are
preserved as-is, allowing for semantic line breaks.

### Long words

Words that exceed the line width limit are not broken and may extend beyond
80 characters.


Spacing
-------

### Blank lines between elements

Use one blank line between paragraphs, list items (in loose lists), and other
block elements.

### Two blank lines before sections

Use two blank lines before Setext-style section headings (H2):

~~~~ markdown
Previous section content.


New section
-----------
~~~~

*Rationale*: Extra spacing creates clear visual separation between major
sections in the plain text source.

### Trailing newline

Files end with exactly one trailing newline.


Punctuation
-----------

Hongdown supports SmartyPants-style punctuation transformations that convert
ASCII punctuation to their typographically correct Unicode equivalents.
These transformations are optional and configurable.

### Curly quotes

Straight double quotes (`"`) are converted to curly quotes:

~~~~ markdown
He said "hello" to her.    →    He said “hello” to her.
~~~~

Straight single quotes (`'`) used as quotation marks are converted to curly
single quotes:

~~~~ markdown
She said 'hello' to him.    →    She said ‘hello’ to him.
~~~~

*Rationale*: Curly quotes are typographically correct and improve the visual
appearance of rendered text.

### Apostrophes

By default, apostrophes in contractions remain as the ASCII character (`'`).
The Unicode name for this character is U+0027 APOSTROPHE, so using it for
apostrophes is semantically correct.  Converting to curly apostrophes can be
enabled via configuration:

~~~~ markdown
It's a beautiful day.    →    It’s a beautiful day.    (when enabled)
The '90s were great.     →    The ’90s were great.     (when enabled)
~~~~

### Ellipsis

Three consecutive periods are converted to the ellipsis character:

~~~~ markdown
Wait for it...    →    Wait for it…
~~~~

Four or more periods are preserved as-is.

### Dashes

Double hyphens are converted to em-dashes by default:

~~~~ markdown
Well--I think so.    →    Well—I think so.
~~~~

En-dashes can be enabled via configuration with a custom pattern.
Single-hyphen patterns only match when surrounded by whitespace to avoid
breaking hyphenated words:

~~~~ markdown
Pages 10 - 20    →    Pages 10 – 20    (when en_dash = "-")
~~~~

### Code preservation

Punctuation transformations are never applied inside code spans or fenced
code blocks.  This ensures that code examples remain syntactically correct:

~~~~ markdown
Use `"string"` for text.    →    Use `"string"` for text.
~~~~


Special elements
----------------

### Footnotes

Footnote definitions are placed at the end of the section where they are
referenced:

~~~~ markdown
This claim needs a citation[^1].

[^1]: Source: Example Study, 2024.
~~~~

### Definition lists

Use the extended syntax for definition lists:

~~~~ markdown
Term
:   Definition of the term.

Another term
:   Its definition.
~~~~

### Abbreviations

Abbreviation definitions are preserved at the end of the document:

~~~~ markdown
The HTML specification defines this behavior.

*[HTML]: HyperText Markup Language
~~~~


Disabled regions
----------------

Formatting can be turned off for part of a document with the HTML comment
directives listed in the *[README](./README.md)*.  What follows describes what
is preserved where they apply.

### Verbatim content

Content whose formatting is disabled is copied from the source as it stands,
including its indentation, the blank lines within it, and any reference or
footnote definition written inside it:

~~~~ markdown
<!-- hongdown-disable -->

Kept    exactly   as written, [guide] and all.

[guide]: https://example.com/guide

<!-- hongdown-enable -->
~~~~

The blank line between a directive comment and the content beside it is
normalized to one.  Everything between them is left alone.

*Rationale*: A reference definition is not a node of its own in the parsed
document, so a region rebuilt element by element loses one.  Copying the
region's source keeps whatever it holds.

### Definitions a disabled region needs

A link that is copied keeps the label the source gave it, so a definition it
depends on is kept even where nothing else refers to it, and stays where the
source put it:

~~~~ markdown
<!-- hongdown-disable -->

[![Build status][badge]][ci]

<!-- hongdown-enable -->

[badge]: https://example.com/badge.svg
[ci]: https://example.com/ci
~~~~

A label a disabled region defines belongs to that region.  Where a formatted
link would otherwise take it for a different target, that link is given a
numbered label instead, as in *[Distinct labels for distinct
targets](#distinct-labels-for-distinct-targets)*.

*Rationale*: A copied link cannot be relabelled without changing text that is
meant to be preserved, and a definition inside the region defines its label for
the whole document.


MDX
---

MDX documents interleave Markdown with JavaScript and JSX.  Those constructs are
not part of CommonMark, so formatting them as ordinary Markdown corrupts the
embedded code.  When MDX mode is enabled, the guiding principle is “first, do no
harm”: detect the JavaScript/JSX constructs and preserve them verbatim, while
still formatting the surrounding Markdown prose.

MDX mode is off by default.  It is enabled automatically for files with the
*.mdx* extension, and can be enabled explicitly for other input.

Protection applies to constructs that appear in paragraph text and that Hongdown
would otherwise corrupt.  Constructs inside headings are left to the heading
formatter (an explicit `{#id}` anchor stays a heading anchor).

### Preserve embedded JavaScript and JSX

The following constructs are preserved exactly as written — never wrapped,
escaped, or punctuation-transformed:

 -  ESM `import` and `export` statements
 -  JSX elements (`<Tabs …>…</Tabs>`, `<Chart … />`) and fragments (`<>…</>`)
 -  `{…}` expressions, including JSX comments (`{/* … */}`)

~~~~ mdx
import { Chart } from "./chart.js";

export const meta = { author: "Hong Minhee" };

<Chart data={{ title: "Sales" }} />

The total is {formatCurrency("USD", total)} for the year.
~~~~

The embedded JavaScript and JSX is preserved, not reformatted; only the
surrounding Markdown is formatted.  A JSX element whose opening tag is valid
inline HTML (for example a single `{value}` attribute) keeps its tag while its
Markdown children are still formatted; an element whose opening tag breaks the
HTML grammar (a `{{…}}` attribute, embedded quotes or spaces, or a multi-line
opener) is preserved as a whole.

### Leave ambiguous and parser-owned constructs alone

To avoid corrupting code, some constructs are deliberately left to the
underlying parser instead of being protected:

 -  Braces inside inline code (`` `{x}` ``), math (`$\frac{1}{2}$`), and
    link/image syntax are not treated as expressions
 -  Constructs inside a heading are left to the heading formatter (an explicit
    `{#id}` anchor stays a heading anchor, not an expression)
 -  Where distinguishing a regex literal from division would require a full
    JavaScript parser (a `/` immediately after `}`), or where an ESM object's
    body contains a blank line, the construct is left unprotected rather than
    risk mis-protecting it
