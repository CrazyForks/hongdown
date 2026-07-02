import { Component, createSignal, onCleanup } from "solid-js";

import sampleShort from "../../sample-short.md?raw";
import { page } from "../../lib/url";
import { createPlayground } from "./createPlayground";
import { Editor } from "./Editor";
import { Output } from "./Output";

// The landing page playground: editor and output only, no options.
// The output wraps at the width of its own pane (measured in mono
// columns) rather than the default 80, so the wrapped result never
// soft-wraps awkwardly in the narrow card.
export const SimplePlayground: Component = () => {
  const playground = createPlayground(sampleShort);
  const [columns, setColumns] = createSignal<number | undefined>(undefined);

  const observePane = (pre: HTMLPreElement) => {
    const measure = () => {
      const styles = getComputedStyle(pre);
      const content =
        pre.clientWidth -
        parseFloat(styles.paddingLeft) -
        parseFloat(styles.paddingRight);
      const context = document.createElement("canvas").getContext("2d");
      if (!content || !context) return;
      context.font = `${styles.fontSize} ${styles.fontFamily}`;
      const charWidth = context.measureText("0").width;
      const cols = Math.max(20, Math.floor(content / charWidth));
      if (cols !== columns()) {
        setColumns(cols);
        playground.setOptions({ lineWidth: cols });
      }
    };
    const observer = new ResizeObserver(measure);
    observer.observe(pre);
    // Re-measure once the mono webfont is in, since fallback metrics
    // differ.
    document.fonts.ready.then(measure);
    onCleanup(() => observer.disconnect());
  };

  const command = () => {
    const cols = columns();
    return cols === undefined
      ? "hongdown input.md"
      : `hongdown --line-width ${cols} input.md`;
  };

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
              {command()}
            </div>
            <Output value={playground.output()} preRef={observePane} />
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
