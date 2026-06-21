import { defineConfig } from "tsdown";

export default defineConfig({
  entry: {
    extension: "src/extension.ts",
  },
  format: ["cjs"],
  platform: "node",
  target: "node20",
  dts: false,
  clean: true,
  external: ["vscode"],
});
