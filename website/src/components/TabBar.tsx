import { Component, For, Show } from "solid-js";

interface TabBarProps {
  tabs: { id: string; label: string; count?: number }[];
  activeTab: string;
  onTabChange: (id: string) => void;
}

export const TabBar: Component<TabBarProps> = (props) => {
  return (
    <div class="flex surface-raised overflow-x-auto">
      <For each={props.tabs}>
        {(tab) => (
          <button
            class={`px-4 py-3 font-mono text-sm transition-colors whitespace-nowrap flex items-center gap-2 cursor-pointer ${
              props.activeTab === tab.id
                ? "surface text-hong dark:text-hong-bright"
                : "bg-transparent text-quiet hover:text-ink dark:hover:text-snow"
            }`}
            onClick={() => props.onTabChange(tab.id)}
          >
            {tab.label}
            <Show when={tab.count !== undefined && tab.count > 0}>
              <span class="bg-amber-100 dark:bg-amber-900/50 text-amber-700 dark:text-amber-400 text-[10px] px-1.5 py-0.5 rounded-full font-semibold">
                {tab.count}
              </span>
            </Show>
          </button>
        )}
      </For>
    </div>
  );
};
