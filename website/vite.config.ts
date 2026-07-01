import path from "node:path";
import { fileURLToPath } from "node:url";

import UnoCSS from "unocss/vite";
import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

import styleDoc from "./plugins/style-doc";

const rootDir = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  plugins: [
    UnoCSS(),
    solid(),
    styleDoc(),
  ],
  base: process.env.GITHUB_ACTIONS ? "/hongdown/" : "/",
  // Three static pages, no client-side router; unknown paths 404 in dev,
  // matching GitHub Pages behavior.
  appType: "mpa",
  build: {
    target: "esnext",
    rollupOptions: {
      input: {
        index: path.resolve(rootDir, "index.html"),
        demo: path.resolve(rootDir, "demo/index.html"),
        style: path.resolve(rootDir, "style/index.html"),
      },
    },
  },
});
