// Data for the landing page's style rule cards.  This module must stay
// a plain data module (no Solid/DOM imports): scripts/check-anchors.ts
// imports it under Node to validate `headingText` against STYLE.md.

export interface StyleRule {
  // Short card title.
  title: string;
  // One-line note under the snippet.
  note: string;
  // Raw Markdown shown in the card, exactly as Hongdown emits it.
  snippet: string;
  // Exact heading text in STYLE.md; resolved to an anchor slug at
  // build time and validated by scripts/check-anchors.ts.
  headingText: string;
}

export const STYLE_RULES: StyleRule[] = [
  {
    title: "Setext headings",
    note: "Underlined H1 and H2 make structure visible without counting # characters.",
    snippet: `Document title
==============

Section name
------------`,
    headingText: "Setext-style for top-level headings",
  },
  {
    title: "Roomy list markers",
    note: "The ` -  ` marker aligns item text to a 4-space tab stop, so continuation lines line up.",
    snippet: ` -  First item
 -  Second item with text that
    continues on the next line
     -  Nested item`,
    headingText: "Unordered list markers",
  },
  {
    title: "Reference-style links",
    note: "External URLs move out of the prose, to the end of the section they appear in.",
    snippet: `See the [documentation] for
more details.

[documentation]: https://…`,
    headingText: "Reference-style for external URLs",
  },
  {
    title: "Tilde code fences",
    note: "Four tildes never collide with the backticks inside your code.",
    snippet: `~~~~ rust
fn main() {
    println!("Hi!");
}
~~~~`,
    headingText: "Fenced code blocks with tildes",
  },
  {
    title: "80-column wrapping",
    note: "Prose wraps at eighty display columns, so it reads well in any editor, pager, or diff.",
    snippet: `Long paragraphs are wrapped at
word boundaries, while short,
intentional lines are left
exactly as you wrote them.`,
    headingText: "Wrap at 80 characters",
  },
  {
    title: "Breathing room",
    note: "Two blank lines before each major section keep the source scannable.",
    snippet: `End of the previous section.


New section
-----------`,
    headingText: "Two blank lines before sections",
  },
  {
    title: "Typographic punctuation",
    note: "Quotes curl, dashes lengthen, ellipses condense, but never inside code.",
    snippet: `"quotes"  →  “quotes”
it's...   →  it's…
Well--no  →  Well—no`,
    headingText: "Curly quotes",
  },
  {
    title: "East Asian alignment",
    note: "Wide characters count as two columns, so tables with CJK text stay aligned.",
    snippet: `| Name | 이름   |
| ---- | ------ |
| Hong | 홍민희 |`,
    headingText: "Column width",
  },
];
