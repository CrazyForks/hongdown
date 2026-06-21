import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import type {
  ConfigLoadResult,
  FormatOptions,
  FormatResult,
} from "@hongdown/wasm";

export interface HongdownWasmApi {
  formatWithWarnings(
    input: string,
    options?: FormatOptions,
  ): Promise<FormatResult>;
  loadConfigFromToml(toml: string): Promise<ConfigLoadResult>;
}

let wasmApi: Promise<HongdownWasmApi> | undefined;
const extensionDir = dirname(fileURLToPath(import.meta.url));

export async function loadHongdownWasm(): Promise<HongdownWasmApi> {
  wasmApi ??= import(
    pathToFileURL(
      join(extensionDir, "vendor/@hongdown/wasm/dist/index.mjs"),
    ).href
  ) as Promise<HongdownWasmApi>;
  return wasmApi;
}
