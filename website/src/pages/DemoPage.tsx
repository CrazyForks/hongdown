import { Component } from "solid-js";

import { SiteHeader } from "../components/SiteHeader";
import { FullPlayground } from "../components/playground/FullPlayground";

export const DemoPage: Component = () => {
  return (
    <div class="flex flex-col h-screen overflow-hidden">
      <SiteHeader current="demo" />
      <main class="flex-1 flex flex-col min-h-0 relative">
        <FullPlayground />
      </main>
    </div>
  );
};
