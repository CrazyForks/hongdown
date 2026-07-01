import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { Marked } from "marked";
import { getHeadingList, gfmHeadingId } from "marked-gfm-heading-id";
import markedShiki from "marked-shiki";
import { codeToHtml } from "shiki";
import type { Plugin } from "vite";

// STYLE.md at the repository root is the single source of truth for the
// /style/ page.  It is rendered at build time; editing STYLE.md reloads
// the page during development.
export const STYLE_MD_PATH = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../../STYLE.md",
);

const SHIKI_THEMES = {
  light: "vitesse-light",
  dark: "vitesse-dark",
} as const;

export interface TocEntry {
  level: number;
  text: string;
  slug: string;
}

export interface StyleDoc {
  html: string;
  toc: TocEntry[];
}

export async function renderStyleDoc(): Promise<StyleDoc> {
  const source = fs.readFileSync(STYLE_MD_PATH, "utf8");
  const marked = new Marked();
  // gfmHeadingId uses github-slugger, so anchors match GitHub's own
  // rendering of STYLE.md.
  marked.use(gfmHeadingId());
  marked.use(
    markedShiki({
      async highlight(code, lang) {
        const language = (lang || "text").toLowerCase();
        try {
          return await codeToHtml(code, {
            lang: language,
            themes: SHIKI_THEMES,
            defaultColor: "light",
          });
        } catch {
          // Unknown language: render as plain text.
          return await codeToHtml(code, {
            lang: "text",
            themes: SHIKI_THEMES,
            defaultColor: "light",
          });
        }
      },
    }),
  );
  const html = await marked.parse(source, { async: true });
  const toc = getHeadingList().map((heading) => ({
    level: heading.level,
    text: heading.text,
    slug: heading.id,
  }));
  return { html, toc };
}

// Heading list only, without syntax highlighting.  Kept separate so the
// landing page (which only needs anchors) doesn't bundle the whole
// rendered document.
export async function renderStyleToc(): Promise<TocEntry[]> {
  const source = fs.readFileSync(STYLE_MD_PATH, "utf8");
  const marked = new Marked();
  marked.use(gfmHeadingId());
  await marked.parse(source, { async: true });
  return getHeadingList().map((heading) => ({
    level: heading.level,
    text: heading.text,
    slug: heading.id,
  }));
}

export default function styleDoc(): Plugin {
  const docId = "virtual:style-doc";
  const tocId = "virtual:style-toc";
  const resolvedDocId = "\0" + docId;
  const resolvedTocId = "\0" + tocId;
  return {
    name: "hongdown-style-doc",
    resolveId(id) {
      if (id === docId) return resolvedDocId;
      if (id === tocId) return resolvedTocId;
    },
    async load(id) {
      if (id === resolvedDocId) {
        this.addWatchFile(STYLE_MD_PATH);
        const { html, toc } = await renderStyleDoc();
        return (
          `export const html = ${JSON.stringify(html)};\n` +
          `export const toc = ${JSON.stringify(toc)};\n`
        );
      }
      if (id === resolvedTocId) {
        this.addWatchFile(STYLE_MD_PATH);
        const toc = await renderStyleToc();
        return `export const toc = ${JSON.stringify(toc)};\n`;
      }
    },
    handleHotUpdate(ctx) {
      if (path.normalize(ctx.file) === STYLE_MD_PATH) {
        for (const id of [resolvedDocId, resolvedTocId]) {
          const mod = ctx.server.moduleGraph.getModuleById(id);
          if (mod) ctx.server.reloadModule(mod);
        }
      }
    },
  };
}
