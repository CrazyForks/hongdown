import { Component, Show, createSignal, onCleanup, onMount } from "solid-js";

import sampleMarkdown from "../../sample.md?raw";
import { OptionsPanel } from "../Options";
import { TabBar } from "../TabBar";
import { createPlayground } from "./createPlayground";
import { Editor } from "./Editor";
import { Output } from "./Output";
import { WarningsBadge, WarningsList } from "./WarningsBadge";

// The demo page playground: side-by-side panes on desktop, tabs on
// mobile, and the complete options panel docked at the bottom.
export const FullPlayground: Component = () => {
  const playground = createPlayground(sampleMarkdown);
  const [activeTab, setActiveTab] = createSignal("editor");
  const [isMobile, setIsMobile] = createSignal(false);

  onMount(() => {
    const checkMobile = () => setIsMobile(window.innerWidth < 1024);
    checkMobile();
    window.addEventListener("resize", checkMobile);
    onCleanup(() => window.removeEventListener("resize", checkMobile));
  });

  return (
    <div class="flex-1 flex flex-col min-h-0 relative">
      <Show when={isMobile()}>
        <TabBar
          tabs={[
            { id: "editor", label: "Editor" },
            { id: "output", label: "Output" },
            { id: "warnings", label: "Warnings", count: playground.warnings().length },
          ]}
          activeTab={activeTab()}
          onTabChange={setActiveTab}
        />
      </Show>

      <div
        class={`flex-1 flex min-h-0 overflow-hidden ${
          isMobile() ? "flex-col" : "flex-row"
        }`}
      >
        <Show when={!isMobile() || activeTab() === "editor"}>
          <section class="flex-1 w-full min-h-0 flex flex-col surface">
            <Editor
              value={playground.input()}
              onInput={playground.setInput}
              label="Markdown input"
            />
          </section>
        </Show>

        <Show when={!isMobile() || activeTab() === "output"}>
          <section class="flex-1 w-full min-h-0 flex flex-col surface-raised">
            <Output value={playground.output()} />
          </section>
        </Show>

        <Show when={isMobile() && activeTab() === "warnings"}>
          <WarningsList warnings={playground.warnings()} />
        </Show>
      </div>

      <Show when={!isMobile()}>
        <WarningsBadge warnings={playground.warnings()} />
      </Show>

      <OptionsPanel
        options={playground.options()}
        setOptions={playground.setOptions}
        resetOptions={playground.resetOptions}
      />
    </div>
  );
};
