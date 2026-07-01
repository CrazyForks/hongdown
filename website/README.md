Hongdown website
================

The website for [Hongdown], deployed to GitHub Pages at
<https://dahlia.github.io/hongdown/>.  It consists of three pages:

 -  `/` — landing page: philosophy, installation, style rule highlights,
    and a simple live playground.
 -  `/demo/` — the full playground with every formatting option.
 -  `/style/` — the complete style guide, rendered at build time from
    *[STYLE.md](../STYLE.md)* at the repository root.

[Hongdown]: https://github.com/dahlia/hongdown


Getting started
---------------

1.  Install dependencies (from the repository root):

    ~~~~ bash
    pnpm install
    ~~~~

2.  Start the development server:

    ~~~~ bash
    pnpm --filter hongdown-website dev
    ~~~~

3.  Open <http://localhost:5173> in your browser.

To create a production build (output in *dist/*):

~~~~ bash
pnpm --filter hongdown-website build
~~~~

To type-check and validate the style guide anchors:

~~~~ bash
pnpm --filter hongdown-website check
~~~~


How the style guide page works
------------------------------

*/style/* is not hand-written: *plugins/style-doc.ts* is a Vite plugin
that reads the repository root *STYLE.md*, renders it with [marked] and
highlights code with [Shiki] at build time, and exposes the result as
the `virtual:style-doc` module.  Editing *STYLE.md* hot-reloads the page
during development and updates the deployed page on the next build.

Heading anchors are generated with github-slugger, so they match
GitHub's own rendering of *STYLE.md*.  The landing page's style rule
cards (*src/content/style-rules.ts*) link to those anchors by heading
text; *scripts/check-anchors.ts* fails the `check` script when a
referenced heading no longer exists.

[marked]: https://marked.js.org/
[Shiki]: https://shiki.style/


Design
------

See *[DESIGN.md](./DESIGN.md)* for the design language: tokens,
typography, the Setext underline device, dark mode rules, and non-goals.
Read it before changing the UI.


Technical details
-----------------

 -  *Framework*: [Solid.js] (three MPA entries, no client-side router)
 -  *Styling*: [UnoCSS]
 -  *Bundler*: [Vite]
 -  *Core*: [@hongdown/wasm]

[Solid.js]: https://www.solidjs.com/
[UnoCSS]: https://unocss.dev/
[Vite]: https://vitejs.dev/
[@hongdown/wasm]: https://www.npmjs.com/package/@hongdown/wasm
