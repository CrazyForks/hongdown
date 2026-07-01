// Validates that every heading referenced by the landing page's style
// rule cards still exists in STYLE.md.  Run with `node
// scripts/check-anchors.ts` (Node 24 strips the types natively); wired
// into `pnpm --filter hongdown-website check`.

import { STYLE_RULES } from "../src/content/style-rules.ts";
import { renderStyleToc, STYLE_MD_PATH } from "../plugins/style-doc.ts";

const toc = await renderStyleToc();
const headings = new Set(toc.map((entry) => entry.text));

const missing = STYLE_RULES.map((rule) => rule.headingText).filter(
  (headingText) => !headings.has(headingText),
);

if (missing.length > 0) {
  console.error(
    `error: ${STYLE_MD_PATH} no longer has heading(s) referenced by ` +
      "website/src/content/style-rules.ts:",
  );
  for (const headingText of missing) {
    console.error(`  - ${JSON.stringify(headingText)}`);
  }
  console.error(
    "Update the style rule cards to match the current STYLE.md headings.",
  );
  process.exit(1);
}

console.log(
  `check-anchors: all ${STYLE_RULES.length} style rule card anchors exist ` +
    "in STYLE.md.",
);
