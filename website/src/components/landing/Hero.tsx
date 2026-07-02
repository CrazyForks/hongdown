import { Component } from "solid-js";

export const Hero: Component = () => {
  return (
    <section class="col-80 pt-16 sm:pt-24 pb-4">
      <p
        aria-hidden="true"
        class="font-mono text-xs sm:text-sm text-ink-faint dark:text-snow-faint mb-6 select-none"
      >
        {"<!-- an opinionated Markdown formatter -->"}
      </p>
      <h1 class="setext setext-1 rule-draw font-mono font-semibold text-3xl sm:text-5xl leading-tight tracking-tight m-0">
        Markdown, readable
        <br />
        before it renders.
      </h1>
      <p class="text-body text-lg sm:text-xl mt-8 max-w-[40rem] leading-relaxed">
        Hongdown formats Markdown so the plain text reads as clearly as the
        rendered page: Setext headings, aligned lists, reference links, and
        80&#8209;column wrapping, applied automatically.
      </p>
      <p class="text-quiet font-serif mt-4 max-w-[40rem] leading-relaxed">
        It is one person's style,{" "}
        <a class="link-hong" href="https://hongminhee.org/" target="_blank" rel="noopener">
          Hong Minhee
        </a>
        's, refined over years of writing documentation for projects like{" "}
        <a class="link-hong" href="https://fedify.dev/" target="_blank" rel="noopener">Fedify</a>,{" "}
        <a class="link-hong" href="https://logtape.org/" target="_blank" rel="noopener">LogTape</a>, and{" "}
        <a class="link-hong" href="https://optique.dev/" target="_blank" rel="noopener">Optique</a>.
      </p>
      <div class="flex flex-wrap gap-3 mt-10">
        <a class="btn-primary" href="#install">Install</a>
        <a class="btn-quiet" href="#playground">Try it live</a>
      </div>
    </section>
  );
};
