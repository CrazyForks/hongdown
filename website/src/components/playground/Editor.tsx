import { Component } from "solid-js";

interface EditorProps {
  value: string;
  onInput: (value: string) => void;
  label: string;
}

export const Editor: Component<EditorProps> = (props) => {
  return (
    <textarea
      class="flex-1 w-full p-4 sm:p-5 font-mono text-sm leading-relaxed resize-none focus:outline-none bg-transparent text-ink dark:text-snow placeholder:text-ink-faint dark:placeholder:text-snow-faint"
      value={props.value}
      onInput={(e) => props.onInput(e.currentTarget.value)}
      placeholder="Type or paste Markdown here…"
      spellcheck={false}
      aria-label={props.label}
    />
  );
};
