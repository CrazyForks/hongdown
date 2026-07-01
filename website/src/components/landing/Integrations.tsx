import { Component, For } from "solid-js";

import { SectionHeading } from "./Section";

const EDITORS = [
  {
    name: "VS Code",
    href: "https://marketplace.visualstudio.com/items?itemName=hongminhee.hongdown",
  },
  { name: "Zed", href: "https://github.com/dahlia/hongdown#zed" },
  { name: "Neovim", href: "https://github.com/dahlia/hongdown#neovim" },
  { name: "Helix", href: "https://github.com/dahlia/hongdown#helix" },
];

export const Integrations: Component = () => {
  return (
    <section class="col-80">
      <SectionHeading>In your editor</SectionHeading>
      <p class="font-serif leading-relaxed max-w-[40rem]">
        Format on save in{" "}
        <For each={EDITORS}>
          {(editor, index) => (
            <>
              <a class="link-hong" href={editor.href} target="_blank" rel="noopener">
                {editor.name}
              </a>
              {index() < EDITORS.length - 2
                ? ", "
                : index() === EDITORS.length - 2
                  ? ", or "
                  : ""}
            </>
          )}
        </For>
        , or run <code class="font-mono text-sm surface-raised px-1.5 py-0.5 rounded">hongdown -w</code>{" "}
        anywhere a terminal runs.
      </p>
      <p class="font-serif text-quiet mt-4 max-w-[40rem] leading-relaxed">
        Embedding it in a tool? Use the{" "}
        <a class="link-hong" href="https://crates.io/crates/hongdown" target="_blank" rel="noopener">
          hongdown
        </a>{" "}
        Rust crate or the{" "}
        <a class="link-hong" href="https://www.npmjs.com/package/@hongdown/wasm" target="_blank" rel="noopener">
          @hongdown/wasm
        </a>{" "}
        package for JavaScript and TypeScript.
      </p>
    </section>
  );
};
