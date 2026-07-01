import { Component, For } from "solid-js";

import { html, toc } from "virtual:style-doc";

import { SiteFooter } from "../components/SiteFooter";
import { SiteHeader } from "../components/SiteHeader";

// The style guide, rendered at build time from STYLE.md at the
// repository root.  Edit STYLE.md, not this page, to change the rules.
export const StylePage: Component = () => {
  const sections = toc.filter((entry) => entry.level === 2);

  return (
    <div class="min-h-screen flex flex-col page-wash">
      <SiteHeader current="style" />
      <main class="flex-1 col-80 pt-10 pb-16 w-full">
        <nav
          aria-label="Table of contents"
          class="mb-12 surface-raised card-tint rounded-xl p-5"
        >
          <h2 class="font-mono text-xs tracking-wide text-quiet mb-3 mt-0">
            Contents
          </h2>
          <ul class="m-0 p-0 list-none columns-2 gap-8 font-mono text-sm">
            <For each={sections}>
              {(entry) => (
                <li class="mb-1.5 break-inside-avoid">
                  <a class="link-hong" href={`#${entry.slug}`}>
                    {entry.text}
                  </a>
                </li>
              )}
            </For>
          </ul>
        </nav>
        <article class="style-doc text-body" innerHTML={html} />
      </main>
      <SiteFooter />
    </div>
  );
};
