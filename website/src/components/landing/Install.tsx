import { Component, For, createSignal } from "solid-js";

import { INSTALL_METHODS, RELEASES_URL } from "../../content/install";
import { SectionHeading } from "./Section";

const CommandRow: Component<{ name: string; command: string }> = (props) => {
  const [copied, setCopied] = createSignal(false);

  const copy = () => {
    navigator.clipboard.writeText(props.command).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  };

  return (
    <div class="flex flex-col sm:flex-row sm:items-start gap-1 sm:gap-4">
      <div class="font-mono text-sm text-quiet sm:w-16 sm:pt-2.5 shrink-0">
        {props.name}
      </div>
      <div class="flex-1 flex items-start gap-2 surface-raised rounded px-3 py-2 min-w-0">
        <pre class="m-0 flex-1 font-mono text-sm leading-relaxed overflow-x-auto">
          {props.command}
        </pre>
        <button
          class="shrink-0 p-1.5 rounded text-quiet hover:text-ink dark:hover:text-snow hover:bg-paper-shade dark:hover:bg-night-shade cursor-pointer transition-colors bg-transparent"
          onClick={copy}
          aria-label={`Copy ${props.name} install command`}
        >
          <div
            class={`w-4 h-4 ${copied() ? "i-carbon-checkmark" : "i-carbon-copy"}`}
          />
        </button>
      </div>
    </div>
  );
};

export const Install: Component = () => {
  return (
    <section class="col-80">
      <SectionHeading id="install">Installation</SectionHeading>
      <div class="flex flex-col gap-3">
        <For each={INSTALL_METHODS}>
          {(method) => <CommandRow name={method.name} command={method.command} />}
        </For>
      </div>
      <p class="font-serif text-quiet mt-6">
        Or grab a pre-built binary for Linux, macOS, or Windows from the{" "}
        <a class="link-hong" href={RELEASES_URL} target="_blank" rel="noopener">
          GitHub releases
        </a>
        .
      </p>
    </section>
  );
};
