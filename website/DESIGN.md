Website design language
=======================

This document records the design decisions behind the Hongdown website so
that future changes keep a consistent look and feel.  Read it before
touching the UI; update it when a decision here changes.


Concept
-------

The website reads like a beautifully formatted plain-text document —
Hongdown's own output is the visual language.  The product's thesis
(*“Markdown should be readable as plain text, not just after rendering”*)
is not just quoted; the site itself demonstrates it:

 -  Page and section headings are *Setext headings*: real `=` and `-`
    characters under the text, clipped to the heading width.
 -  Section dividers are Hongdown's thematic break style
    (`- - - - - - …`).
 -  The philosophy list uses Hongdown's ` -  ` list marker, rendered
    literally.
 -  The header wordmark is a miniature Setext H1 (text over a double
    rule), echoed by the favicon.

When adding a new visual element, prefer a device that exists in
Hongdown's plain-text output over an invented one.


Voice and copy
--------------

 -  Confident, dry, and honest: the site says outright that this is one
    person's style, and why it is still worth adopting.
 -  Terse over salesy.  Describe what the formatter does in plain terms
    (“URLs move out of the prose”), not marketing language.
 -  Sentence case everywhere — headings, buttons, labels — matching the
    style convention Hongdown itself enforces.
 -  UI labels name what the user controls (“Line width”), never
    internals.


Design tokens
-------------

Tokens live in *uno.config.ts* (`theme.colors`, `theme.fontFamily`,
`shortcuts`) and, for hand-written CSS devices, as custom properties in
*src/styles.css* (`--hong`, `--rule-faint`, `--surface-raised`).  Keep
the two in sync.

### Color

| Token           | Light     | Dark      | Role                          |
| --------------- | --------- | --------- | ----------------------------- |
| `paper`/`night` | `#FCFCFB` | `#141210` | Page background               |
| `…-raised`      | `#F4F3F0` | `#1E1B18` | Cards, code blocks, panels    |
| `…-shade`       | `#ECEAE5` | `#282420` | Hover states, controls        |
| `ink`/`snow`    | `#221E1B` | `#E9E6E1` | Body text                     |
| `…-mute`        | `#6E6861` | `#A8A198` | Secondary text (`text-quiet`) |
| `…-faint`       | `#A39C92` | `#6E6861` | Rules, placeholders           |
| `hong`          | `#BE3450` | `#E7768C` | The accent (see below)        |

The accent is *crimson* on purpose: 홍/紅 (*hong*) means crimson, and a
dahlia — the author's handle — is a crimson flower.  It is reserved for
links, the hero/H1 Setext underline, primary buttons, toggles, and small
markers.  Everything else stays neutral; do not introduce a second
accent color.  Warnings may use amber, sparingly.

### Typography

 -  *IBM Plex Mono* — everything from the plain-text world: headings,
    raw Markdown samples, code, navigation, buttons, labels, metadata.
 -  *Source Serif 4* — prose paragraphs (the “reading first” thesis:
    book typography for a tool about readability).

There is no sans-serif role.  If text is not prose, it is mono.

### Layout

 -  The content column is `col-80` (max-width 48 rem = 768 px), which is
    exactly 80 columns of 16 px IBM Plex Mono — the style guide's line
    width, as a layout measure.
 -  *Borderless hierarchy*: separation comes from background shifts
    (`surface` vs `surface-raised`), not border lines.  Hairline rings
    (`ring-ink/8`) are acceptable only where two same-color surfaces
    would otherwise merge.
 -  Corner radii are modest: `rounded` (0.25 rem) for controls,
    `rounded-xl` (0.75 rem) for cards and panels.

### Gradient tints

Gradients exist only as *tints* of the accent — never as gradient text
and never with a second hue.  All are defined in *src/styles.css*:

 -  `.page-wash` — a radial crimson wash bleeding from the top of every
    page (5% opacity in light mode, 8% in dark), like warm light
    falling on paper.
 -  `.card-tint` — a 3.5% diagonal tint on raised cards (rule cards,
    the style page TOC).
 -  `.btn-primary` gloss — a faint white-to-transparent overlay on the
    primary button.

Keep strengths at or below these values; if a tint is clearly visible
as a gradient, it is too strong.

### The setext underline device

Implemented in *src/styles.css* (`.setext`, `.setext-1`, `.setext-2`,
and the `.style-doc h1/h2` equivalents).  The underline is real `=`/`-`
characters in a `::after` pseudo-element with
`width: 0; min-width: 100%; overflow: hidden` so it clips to the heading text
width without inflating the heading's `fit-content` box.  H1 underlines are
crimson; H2 underlines are faint.  Do not replace these with `border-bottom` —
the character texture is the point.


Dark mode
---------

System preference only (`presetUno({ dark: "media" })`); there is no
manual toggle, and none should be added.  Every change must be checked
in both schemes.  Hand-written CSS uses the custom properties from
*src/styles.css*, which flip inside one `@media (prefers-color-scheme: dark)`
block.  Shiki code blocks emit dual-theme inline styles
(`vitesse-light`/`vitesse-dark`); *styles.css* flips them with the
`--shiki-dark` variable override.


Motion
------

Restraint: hover transitions, the copy-button check mark, the rule
cards' hover lift (`.card-lift`: a 2 px rise with a soft crimson-tinged
shadow), and exactly one orchestrated moment — the hero underline
drawing itself in on page load (`.rule-draw`).  Anything animated must
respect `prefers-reduced-motion`.


Page inventory
--------------

| Page      | Entry                   | Notes                                                                           |
| --------- | ----------------------- | ------------------------------------------------------------------------------- |
| `/`       | *src/entries/index.tsx* | Landing: hero, install, philosophy, rule cards, simple playground, integrations |
| `/demo/`  | *src/entries/demo.tsx*  | Full playground, app-shell layout (fixed viewport height)                       |
| `/style/` | *src/entries/style.tsx* | STYLE.md rendered at build time                                                 |

Shared chrome is `SiteHeader`/`SiteFooter`; the playground core
(`createPlayground`, `Editor`, `Output`) is shared between the simple
and full playgrounds.

### The STYLE.md pipeline

*/style/* is generated from the repository root *STYLE.md* by
*plugins/style-doc.ts* (a Vite plugin exposing `virtual:style-doc` and
`virtual:style-toc`).  Anchors use github-slugger, so they match
GitHub's rendering of STYLE.md.  The landing page's rule cards
(*src/content/style-rules.ts*) reference STYLE.md headings *by text*;
*scripts/check-anchors.ts* (run in `pnpm check` and `mise run check:website`)
fails when a referenced heading disappears.  When STYLE.md changes, the page
updates on the next build — but check the rule cards still showcase rules that
exist.


Non-goals
---------

 -  No client-side router (three static MPA entries).
 -  No SSR or prerendering; the canonical, indexable spec is STYLE.md on
    GitHub.
 -  No CMS, no analytics, no cookie banners.
 -  No manual dark mode toggle.
 -  No `og:image` for now; if one is added later, derive it from the
    favicon's Setext-H mark.
