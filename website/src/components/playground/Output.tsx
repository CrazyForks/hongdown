import { Component, createSignal } from "solid-js";

interface OutputProps {
  value: string;
  preRef?: (el: HTMLPreElement) => void;
}

export const Output: Component<OutputProps> = (props) => {
  const [copied, setCopied] = createSignal(false);

  const copy = () => {
    navigator.clipboard.writeText(props.value).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  };

  return (
    <div class="flex-1 overflow-auto relative min-h-0">
      <div class="absolute top-3 right-3 z-10">
        <button
          class="btn-quiet py-1 px-2.5 text-xs backdrop-blur-sm bg-paper/80 dark:bg-night/80"
          onClick={copy}
        >
          <div
            class={`w-3.5 h-3.5 ${copied() ? "i-carbon-checkmark" : "i-carbon-copy"}`}
          />
          {copied() ? "Copied" : "Copy"}
        </button>
      </div>
      <pre
        ref={props.preRef}
        class="m-0 p-4 sm:p-5 font-mono text-sm leading-relaxed whitespace-pre-wrap select-text cursor-text text-ink dark:text-snow"
      >
        {props.value}
      </pre>
    </div>
  );
};
