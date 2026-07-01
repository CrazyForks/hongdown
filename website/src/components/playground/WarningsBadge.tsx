import { Component, For, Show } from "solid-js";

import { Warning } from "@hongdown/wasm";

interface WarningsBadgeProps {
  warnings: Warning[];
}

// Floating warnings summary for the full playground on desktop.
export const WarningsBadge: Component<WarningsBadgeProps> = (props) => {
  return (
    <Show when={props.warnings.length > 0}>
      <div class="absolute right-6 bottom-20 z-20">
        <div class="bg-paper dark:bg-night-raised rounded-lg shadow-lg p-3 max-w-sm ring-1 ring-amber-500/30">
          <div class="flex items-center gap-2 mb-2 text-amber-700 dark:text-amber-400 font-mono font-medium text-xs">
            <div class="i-carbon-warning-alt w-4 h-4" />
            Warnings ({props.warnings.length})
          </div>
          <div class="max-h-40 overflow-y-auto flex flex-col gap-1.5">
            <For each={props.warnings.slice(0, 3)}>
              {(w) => (
                <div class="text-xs text-quiet border-l-2 border-amber-500/50 pl-2">
                  Line {w.line}: {w.message}
                </div>
              )}
            </For>
            <Show when={props.warnings.length > 3}>
              <div class="text-xs text-ink-faint dark:text-snow-faint italic">
                and {props.warnings.length - 3} more…
              </div>
            </Show>
          </div>
        </div>
      </div>
    </Show>
  );
};

// Full-height warnings list for the mobile tab of the full playground.
export const WarningsList: Component<WarningsBadgeProps> = (props) => {
  return (
    <section class="flex-1 w-full flex flex-col surface p-4 min-h-0 overflow-auto">
      <h2 class="font-mono font-semibold text-sm mb-4">
        Warnings ({props.warnings.length})
      </h2>
      <Show
        when={props.warnings.length > 0}
        fallback={
          <div class="flex flex-col items-center justify-center flex-1 text-quiet">
            <div class="i-carbon-checkmark-outline w-10 h-10 mb-2 opacity-60" />
            <p class="font-mono text-sm">No warnings.</p>
          </div>
        }
      >
        <div class="flex flex-col gap-2">
          <For each={props.warnings}>
            {(w) => (
              <div class="p-3 surface-raised rounded flex gap-3">
                <div class="i-carbon-warning-alt text-amber-600 dark:text-amber-400 w-5 h-5 flex-shrink-0" />
                <div class="text-sm">
                  <div class="font-mono font-medium text-amber-700 dark:text-amber-400">
                    Line {w.line}
                  </div>
                  <div class="text-quiet">{w.message}</div>
                </div>
              </div>
            )}
          </For>
        </div>
      </Show>
    </section>
  );
};
