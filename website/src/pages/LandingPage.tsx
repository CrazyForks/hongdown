import { Component } from "solid-js";

import { SiteFooter } from "../components/SiteFooter";
import { SiteHeader } from "../components/SiteHeader";
import { Hero } from "../components/landing/Hero";
import { Install } from "../components/landing/Install";
import { Integrations } from "../components/landing/Integrations";
import { Philosophy } from "../components/landing/Philosophy";
import { SectionHeading, ThematicBreak } from "../components/landing/Section";
import { StyleRuleCards } from "../components/landing/StyleRuleCards";
import { SimplePlayground } from "../components/playground/SimplePlayground";

export const LandingPage: Component = () => {
  return (
    <div class="min-h-screen">
      <SiteHeader current="home" />
      <main>
        <Hero />
        <ThematicBreak />
        <Install />
        <ThematicBreak />
        <Philosophy />
        <ThematicBreak />
        <StyleRuleCards />
        <ThematicBreak />
        <section class="col-80">
          <SectionHeading id="playground">Try it</SectionHeading>
          <SimplePlayground />
        </section>
        <ThematicBreak />
        <Integrations />
      </main>
      <div class="h-8" />
      <SiteFooter />
    </div>
  );
};
