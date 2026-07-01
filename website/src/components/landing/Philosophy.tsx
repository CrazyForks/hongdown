import { Component, For } from "solid-js";

import { SectionHeading } from "./Section";

const TENETS = [
  {
    term: "Visual structure in source",
    detail: "Headings, lists, and sections are distinct in the raw text.",
  },
  {
    term: "Consistent spacing",
    detail: "Predictable whitespace lets you scan a document's structure.",
  },
  {
    term: "Minimal escaping",
    detail: "Delimiters are chosen to avoid backslashes in the first place.",
  },
  {
    term: "Reference-style links",
    detail: "URLs live at the end of the section, not in the middle of a sentence.",
  },
];

export const Philosophy: Component = () => {
  return (
    <section class="col-80">
      <SectionHeading>Philosophy</SectionHeading>
      <blockquote class="m-0 border-l-3 border-hong dark:border-hong-bright pl-5 font-serif italic text-xl sm:text-2xl leading-relaxed max-w-[38rem]">
        Markdown should be readable as plain text, not just after rendering.
      </blockquote>
      <ul class="list-none m-0 mt-10 p-0 flex flex-col gap-4">
        <For each={TENETS}>
          {(tenet) => (
            <li class="flex gap-1 items-baseline">
              {/* Hongdown's own list marker. */}
              <span
                aria-hidden="true"
                class="font-mono text-hong dark:text-hong-bright select-none whitespace-pre shrink-0"
              >
                {" -  "}
              </span>
              <p class="m-0 font-serif leading-relaxed">
                <strong class="font-semibold">{tenet.term}</strong>
                {" — "}
                <span class="text-quiet">{tenet.detail}</span>
              </p>
            </li>
          )}
        </For>
      </ul>
      <p class="font-serif text-quiet mt-8 max-w-[40rem] leading-relaxed">
        The style prioritizes reading over writing. Many of its rules are
        tedious to follow by hand — that's intentional. The formatter does the
        tedious part, so you can spend your attention on what the document
        says.
      </p>
    </section>
  );
};
