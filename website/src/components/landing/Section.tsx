import { Component, JSX } from "solid-js";

interface SectionHeadingProps {
  id?: string;
  children: JSX.Element;
}

// Landing section headings are Setext H2s, like Hongdown would emit.
export const SectionHeading: Component<SectionHeadingProps> = (props) => {
  return (
    <h2
      id={props.id}
      class="setext setext-2 font-mono font-semibold text-xl sm:text-2xl m-0 mb-8 scroll-mt-20"
    >
      {props.children}
    </h2>
  );
};

// Hongdown's own thematic break, used as the divider between landing
// sections.
export const ThematicBreak: Component = () => {
  return (
    <div
      aria-hidden="true"
      class="col-80 py-12 font-mono text-sm text-ink-faint dark:text-snow-faint overflow-hidden whitespace-nowrap select-none"
    >
      {"   - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -"}
    </div>
  );
};
