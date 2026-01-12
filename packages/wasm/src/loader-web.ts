/**
 * WASM loader for web browser environment.
 * @internal
 */

export async function loadWasmBuffer(): Promise<ArrayBuffer> {
  const wasmUrl = new URL("./hongdown_bg.wasm", import.meta.url);
  const response = await fetch(wasmUrl);
  return response.arrayBuffer();
}
