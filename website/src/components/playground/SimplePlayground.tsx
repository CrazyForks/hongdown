import { Component } from "solid-js";

import sampleShort from "../../sample-short.md?raw";
import { page } from "../../lib/url";
import { createPlayground } from "./createPlayground";
import { Editor } from "./Editor";
import { Output } from "./Output";

// The landing page playground: editor and output only, no options.
export const SimplePlayground: Component = () => {
  const playground = createPlayground(sampleShort);

  return (
    <div>
      <div class="rounded-xl overflow-hidden ring-1 ring-ink/8 dark:ring-snow/8">
        <div class="grid md:grid-cols-2 h-[34rem] md:h-[26rem]">
          <div class="flex flex-col min-h-0 surface">
            <div class="px-4 pt-3 pb-1 font-mono text-xs text-quiet select-none">
              input.md
            </div>
            <Editor
              value={playground.input()}
              onInput={playground.setInput}
              label="Markdown input"
            />
          </div>
          <div class="flex flex-col min-h-0 surface-raised">
            <div class="px-4 pt-3 pb-1 font-mono text-xs text-quiet select-none">
              hongdown input.md
            </div>
            <Output value={playground.output()} />
          </div>
        </div>
      </div>
      <p class="mt-4 font-mono text-sm">
        <a class="link-hong" href={page("demo/")}>
          Open the full demo with every formatting option →
        </a>
      </p>
    </div>
  );
};
