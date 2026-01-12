import { defineConfig } from "tsdown";
import { copyFileSync, mkdirSync } from "node:fs";
import type { Plugin } from "rolldown";

// Plugin to copy WASM file to dist
function copyWasmPlugin(): Plugin {
  return {
    name: "copy-wasm",
    buildEnd() {
      mkdirSync("dist", { recursive: true });
      copyFileSync("pkg/hongdown_bg.wasm", "dist/hongdown_bg.wasm");
    },
  };
}

export default defineConfig({
  entry: {
    index: "src/index.ts",
    "loader-node": "src/loader-node.ts",
    "loader-web": "src/loader-web.ts",
  },
  format: ["esm", "cjs"],
  dts: true,
  clean: true,
  external: ["node:fs/promises", "node:path", "node:url", "#wasm-loader"],
  plugins: [copyWasmPlugin()],
});
