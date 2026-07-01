import { createEffect, createSignal } from "solid-js";

import { FormatOptions, formatWithWarnings, Warning } from "@hongdown/wasm";

// Reactive formatting state shared by the simple (landing) and full
// (demo page) playgrounds: whenever the input or the options change,
// the output and warnings are recomputed.
export function createPlayground(initialInput: string) {
  const [input, setInput] = createSignal(initialInput);
  const [output, setOutput] = createSignal("");
  const [warnings, setWarnings] = createSignal<Warning[]>([]);
  const [options, setOptions] = createSignal<FormatOptions>({});

  createEffect(() => {
    // Read the signals synchronously so the effect tracks them.
    const source = input();
    const opts = options();
    formatWithWarnings(source, opts)
      .then((result) => {
        setOutput(result.output);
        setWarnings(result.warnings);
      })
      .catch((e) => console.error("Formatting error:", e));
  });

  const resetOptions = () => setOptions({});

  return { input, setInput, output, warnings, options, setOptions, resetOptions };
}

export type Playground = ReturnType<typeof createPlayground>;
