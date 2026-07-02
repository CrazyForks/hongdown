import { Component } from "solid-js";

// The footer divider is Hongdown's own thematic break style.
const THEMATIC_BREAK =
  "- - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -";

export const SiteFooter: Component = () => {
  return (
    <footer class="col-80 pb-12 pt-4 font-mono text-xs text-quiet">
      <p aria-hidden="true" class="overflow-hidden whitespace-nowrap select-none mb-8">
        {"   " + THEMATIC_BREAK}
      </p>
      <p class="mb-3 max-w-prose">
        <i>Hongdown</i> is <i>Hong</i> + <i>Markdown</i>. It also sounds like
        the Korean <span lang="ko" class="whitespace-nowrap">홍답다</span>, “befitting of Hong.”
      </p>
      <p class="flex flex-wrap gap-x-4 gap-y-1">
        <a class="link-hong" href="https://github.com/dahlia/hongdown" target="_blank" rel="noopener">
          GitHub
        </a>
        <a class="link-hong" href="https://crates.io/crates/hongdown" target="_blank" rel="noopener">
          crates.io
        </a>
        <a class="link-hong" href="https://www.npmjs.com/package/hongdown" target="_blank" rel="noopener">
          npm
        </a>
        <a class="link-hong" href="https://www.npmjs.com/package/@hongdown/wasm" target="_blank" rel="noopener">
          @hongdown/wasm
        </a>
      </p>
      <p class="mt-3">
        GPL-3.0-or-later · Made by{" "}
        <a class="link-hong" href="https://hongminhee.org/" target="_blank" rel="noopener">
          Hong Minhee
        </a>
      </p>
    </footer>
  );
};
