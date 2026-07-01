# Hong Minhee's Markdown style convention

This document describes the Markdown style convention enforced by Hongdown.

## Philosophy

The core principle of this style is:

> _Markdown should be readable as plain text, not just after rendering._

A well-formatted Markdown document should convey its structure and emphasis clearly even when viewed in a plain text editor without any rendering. You shouldn't need to render the document to HTML to understand its formatting and hierarchy.

This philosophy leads to several practical implications:

- _Visual structure in source_: Headings, lists, and sections should be visually distinct in the raw text.
- _Consistent spacing_: Predictable whitespace patterns help readers scan the document structure.
- _Minimal escaping_: Choose delimiter styles that minimize the need for escape characters.
- _Reference-style links_: Keep prose readable by moving URLs out of the text flow.

This style prioritizes reading over writing. Many rules are tedious to follow manually—and that's intentional. The assumption is that you'll use an automated formatter like [Hongdown](https://github.com/dahlia/hongdown) to handle the mechanical work, freeing you to focus on content rather than formatting details.

## Headings

### Setext-style for top-level headings

Use Setext-style (underlined) headings for document titles (H1) and major sections (H2):

```markdown
# Document Title

## Section Name
```

_Rationale_: Setext headings create strong visual separation in plain text. The underline makes the heading immediately recognizable without needing to count `#` characters.

### ATX-style for subsections

Use ATX-style (`###`, `####`, etc.) for subsections within a section:

```markdown
### Subsection

#### Sub-subsection
```

_Rationale_: ATX-style is more compact for deeper nesting levels where Setext-style would be awkward.

### Sentence case

Use sentence case for headings (capitalize only the first word and proper nouns):

```markdown
Development commands ← Correct Development Commands ← Incorrect
```

_Rationale_: Sentence case is easier to read and more natural in technical documentation.

### Underline length

The underline of a Setext-style heading should match the display width of the heading text, accounting for East Asian wide characters.

#### East Asian character width

East Asian wide characters (CJK characters) are counted as two columns when calculating the display width.

## Emphasis

### Asterisks for emphasis

Use asterisks (`*`) for emphasis by default:

```markdown
This is _emphasized_ text. This is **strongly emphasized** text.
```

### Underscores when content contains asterisks

When the emphasized content contains asterisk characters, use underscores to avoid escaping:

```markdown
The file _*.txt_ matches all text files. The pattern ****/*.md** matches recursively.
```

_Rationale_: This produces cleaner source text by avoiding backslash escapes.

### Escape all underscores in regular text

Underscores in regular text are always escaped, even in the middle of words:

```markdown
Use the CONFIG\_FILE\_NAME constant.
```

_Rationale_: While CommonMark doesn't treat intraword underscores as emphasis delimiters, escaping ensures consistent rendering across all Markdown parsers.

## Lists

### Unordered list markers

Use `-` (space, hyphen, two spaces) for unordered list items:

```markdown
- First item
- Second item
- Third item
```

_Rationale_: The leading space creates visual indentation from the left margin. The two trailing spaces align the text with a 4-space tab stop, making continuation lines easy to align.

### Nested lists

Indent nested items by 4 spaces:

```markdown
- Parent item
  - Child item
  - Another child
- Another parent
```

### Ordered list markers

Use `.` for odd nesting levels and `)` for even nesting levels:

```markdown
1. First item
2. Second item
   1. Nested first
   2. Nested second
3. Third item
```

_Rationale_: Alternating markers make the nesting level visually apparent.

### Fixed marker width

Ordered list markers maintain a fixed 4-character width. When numbers grow longer, trailing spaces are reduced (minimum 1 space):

```markdown
1. First item
2. Second item ...
3. Ninth item
4. Tenth item
```

_Rationale_: Consistent marker width keeps continuation lines aligned at the same column regardless of item count.

### Continuation lines

Align continuation lines with the start of the item text:

```markdown
- This is a list item with text that continues on the next line with proper alignment.
```

### Task lists

Task list items use checkboxes (`[ ]` for unchecked, `[x]` for checked) after the list marker:

```markdown
- [ ] Unchecked task
- [x] Completed task
```

_Rationale_: Task lists follow the same spacing rules as regular unordered lists, keeping the document consistent.

## Code

### Fenced code blocks with tildes

Use four tildes (`~~~~`) for fenced code blocks:

````markdown
```rust
fn main() {
    println!("Hello, world!");
}
```
````

_Rationale_: Tildes are visually distinct from the code content, which often contains backticks for string literals or shell commands.

### Language identifiers

Always specify a language identifier for syntax highlighting. If no specific language applies, the identifier can be omitted:

````markdown
```javascript
console.log("Hello");
```
````

### Inline code spans

Use backticks for inline code. When the content contains backticks, use multiple backticks as delimiters:

```markdown
Use the `format()` function. The syntax is `` `code` `` with backticks.
```

Preserve original spacing in code spans. If the original source has space padding (`` ` code ` ``), it is preserved in the output.

## Links

### Reference-style for external URLs

Convert external URLs to reference-style links, with definitions placed at the end of the current section:

```markdown
See the [documentation] for more details.

Read the [installation guide] before proceeding.

[documentation]: https://example.com/docs
[installation guide]: https://example.com/install
```

_Rationale_: Reference-style links keep the prose readable by moving long URLs out of the text flow. Placing definitions at section end keeps related content together.

### Inline style for relative URLs

Keep relative URLs and fragment links inline:

```markdown
See _[Chapter 2](./chapter2.md)_ for more details. Jump to the [installation section](#installation).
```

_Rationale_: For inter-document links, the filename itself serves as a natural identifier. Using reference-style would create redundancy:

```markdown
See also _[Chapter 2]_ for more details.

[Chapter 2]: ./chapter2.md
```

The reference definition just repeats what the link text already conveys.

### Shortcut references when text matches label

When the link text matches the reference label, use shortcut reference syntax:

```markdown
See [GitHub] for the source code.

[GitHub]: https://github.com/example/repo
```

### Collapsed references before brackets

When a shortcut reference would be immediately followed by text starting with `[` (such as a footnote reference), use collapsed reference syntax `[text][]` instead of shortcut syntax `[text]` to avoid ambiguity:

```markdown
See [GitHub][GitHub][^1] for details.

[GitHub]: https://github.com/example/repo

[^1]: Footnote text.
```

_Rationale_: Without the empty brackets, `[GitHub][^1]` could be parsed as a full reference link with label `^1`, which would break the intended link and footnote.

## Block quotes and alerts

### Block quote continuation

Continue block quotes with `>` on each line:

```markdown
> This is a block quote that spans multiple lines of text.
```

### GitHub-style alerts

Use GitHub-flavored alert syntax for callouts:

```markdown
> [!NOTE]
> This is a note with additional information.

> [!WARNING]
> This action cannot be undone.
```

Supported alert types: `NOTE`, `TIP`, `IMPORTANT`, `WARNING`, `CAUTION`.

## Tables

### Pipe table formatting

Use pipe tables with proper column alignment:

```markdown
| Name | Description       |
| ---- | ----------------- |
| foo  | The foo component |
| bar  | The bar component |
```

### Column width

Columns are padded to align pipes vertically. East Asian wide characters are counted as two columns for proper alignment.

### Escaped pipes in content

Pipe characters within cell content are escaped:

```markdown
| Pattern  | Meaning |
| -------- | ------- |
| `a \| b` | a or b  |
```

## Thematic breaks

### Dashes with spaces

Thematic breaks (horizontal rules) are rendered as a long line of spaced dashes (37 dashes) with 3 leading spaces:

```markdown
Previous section content.

---

New section content.
```

_Rationale_: The extended line of dashes creates a visually prominent separator that resembles a traditional horizontal rule, making section breaks immediately apparent when scanning the plain text source.

## Line wrapping

### Wrap at 80 characters

Wrap prose at approximately 80 display columns:

```markdown
This is a paragraph that demonstrates line wrapping. When text exceeds the 80-character limit, it wraps to the next line while preserving word boundaries.
```

_Rationale_: 80 characters is a widely supported terminal width that ensures readability across different editors and viewers.

### East Asian character width

East Asian wide characters (CJK characters) are counted as two columns when calculating line width.

### Preserve intentional short lines

Lines that are intentionally short in the source (well under the limit) are preserved as-is, allowing for semantic line breaks.

### Long words

Words that exceed the line width limit are not broken and may extend beyond 80 characters.

## Spacing

### Blank lines between elements

Use one blank line between paragraphs, list items (in loose lists), and other block elements.

### Two blank lines before sections

Use two blank lines before Setext-style section headings (H2):

```markdown
Previous section content.

## New section
```

_Rationale_: Extra spacing creates clear visual separation between major sections in the plain text source.

### Trailing newline

Files end with exactly one trailing newline.

## Punctuation

Hongdown supports SmartyPants-style punctuation transformations that convert ASCII punctuation to their typographically correct Unicode equivalents. These transformations are optional and configurable.

### Curly quotes

Straight double quotes (`"`) are converted to curly quotes:

```markdown
He said "hello" to her. → He said “hello” to her.
```

Straight single quotes (`'`) used as quotation marks are converted to curly single quotes:

```markdown
She said 'hello' to him. → She said ‘hello’ to him.
```

_Rationale_: Curly quotes are typographically correct and improve the visual appearance of rendered text.

### Apostrophes

By default, apostrophes in contractions remain as the ASCII character (`'`). The Unicode name for this character is U+0027 APOSTROPHE, so using it for apostrophes is semantically correct. Converting to curly apostrophes can be enabled via configuration:

```markdown
It's a beautiful day. → It’s a beautiful day. (when enabled) The '90s were great. → The ’90s were great. (when enabled)
```

### Ellipsis

Three consecutive periods are converted to the ellipsis character:

```markdown
Wait for it... → Wait for it…
```

Four or more periods are preserved as-is.

### Dashes

Double hyphens are converted to em-dashes by default:

```markdown
Well--I think so. → Well—I think so.
```

En-dashes can be enabled via configuration with a custom pattern. Single-hyphen patterns only match when surrounded by whitespace to avoid breaking hyphenated words:

```markdown
Pages 10 - 20 → Pages 10 – 20 (when en_dash = "-")
```

### Code preservation

Punctuation transformations are never applied inside code spans or fenced code blocks. This ensures that code examples remain syntactically correct:

```markdown
Use `"string"` for text. → Use `"string"` for text.
```

## Special elements

### Footnotes

Footnote definitions are placed at the end of the section where they are referenced:

```markdown
This claim needs a citation[^1].

[^1]: Source: Example Study, 2024.
```

### Definition lists

Use the extended syntax for definition lists:

```markdown
Term : Definition of the term.

Another term : Its definition.
```

### Abbreviations

Abbreviation definitions are preserved at the end of the document:

```markdown
The HTML specification defines this behavior.

*[HTML]: HyperText Markup Language
```
