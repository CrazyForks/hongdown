import { Component, For } from "solid-js";

import { toc } from "virtual:style-toc";

import { STYLE_RULES } from "../../content/style-rules";
import { page } from "../../lib/url";
import { SectionHeading } from "./Section";

// Resolves a STYLE.md heading text to its anchor slug.  Throws when the
// heading no longer exists so a renamed heading fails loudly at build
// time instead of shipping a dead link.  scripts/check-anchors.ts runs
// the same validation in CI.
function anchorFor(headingText: string): string {
  const entry = toc.find((e) => e.text === headingText);
  if (entry === undefined) {
    throw new Error(
      `STYLE.md has no heading named ${JSON.stringify(headingText)}; ` +
        "update website/src/content/style-rules.ts.",
    );
  }
  return entry.slug;
}

export const StyleRuleCards: Component = () => {
  return (
    <section class="col-80">
      <SectionHeading>The style, at a glance</SectionHeading>
      <div class="grid sm:grid-cols-2 gap-4">
        <For each={STYLE_RULES}>
          {(rule) => (
            <div class="surface-raised card-tint card-lift rounded-xl p-5 flex flex-col gap-3">
              <pre class="m-0 font-mono text-xs leading-relaxed overflow-x-auto text-ink dark:text-snow">
                {rule.snippet}
              </pre>
              <h3 class="m-0 mt-auto font-mono text-sm font-semibold pt-2">
                {rule.title}
              </h3>
              <p class="m-0 font-serif text-sm text-quiet leading-relaxed">
                {rule.note}
              </p>
              <a
                class="link-hong no-underline hover:underline font-mono text-xs"
                href={page("style/", anchorFor(rule.headingText))}
              >
                details →
              </a>
            </div>
          )}
        </For>
      </div>
      <p class="mt-8 font-mono text-sm">
        <a class="link-hong" href={page("style/")}>
          Read the full style guide →
        </a>
      </p>
    </section>
  );
};
