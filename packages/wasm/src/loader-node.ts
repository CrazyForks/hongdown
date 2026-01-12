/**
 * WASM loader for Node.js environment.
 * @internal
 */

import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));

export async function loadWasmBuffer(): Promise<ArrayBuffer> {
  const wasmPath = resolve(__dirname, "hongdown_bg.wasm");
  const buffer = await readFile(wasmPath);
  return buffer.buffer.slice(
    buffer.byteOffset,
    buffer.byteOffset + buffer.byteLength,
  );
}
