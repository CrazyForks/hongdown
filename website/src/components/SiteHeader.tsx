import { Component, For } from "solid-js";

import { page } from "../lib/url";

interface SiteHeaderProps {
  current: "home" | "demo" | "style";
}

const NAV_ITEMS = [
  { id: "home", label: "Home", href: page("") },
  { id: "demo", label: "Demo", href: page("demo/") },
  { id: "style", label: "Style guide", href: page("style/") },
] as const;

export const SiteHeader: Component<SiteHeaderProps> = (props) => {
  return (
    <header class="sticky top-0 z-30 bg-paper/85 dark:bg-night/85 backdrop-blur-sm">
      <div class="col-80 h-14 flex items-center justify-between font-mono text-sm">
        <a
          href={page("")}
          class="no-underline text-ink dark:text-snow font-semibold tracking-tight"
        >
          {/* The wordmark is a miniature Setext heading: text over a
              double rule. */}
          <span class="wordmark">
            Hongdown
          </span>
        </a>
        <nav class="flex items-center gap-4 sm:gap-6">
          <For each={NAV_ITEMS}>
            {(item) => (
              <a
                href={item.href}
                aria-current={props.current === item.id ? "page" : undefined}
                class={`no-underline transition-colors ${
                  props.current === item.id
                    ? "text-ink dark:text-snow"
                    : "text-quiet hover:text-ink dark:hover:text-snow"
                }`}
              >
                {item.label}
              </a>
            )}
          </For>
          <a
            href="https://github.com/dahlia/hongdown"
            target="_blank"
            rel="noopener"
            aria-label="Hongdown on GitHub"
            class="text-quiet hover:text-ink dark:hover:text-snow transition-colors"
          >
            <div class="i-carbon-logo-github w-5 h-5" />
          </a>
        </nav>
      </div>
    </header>
  );
};
