import { cp, mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const packageRoot = resolve(here, "..");
const wasmRoot = resolve(packageRoot, "../wasm");
const sourceDist = resolve(wasmRoot, "dist");
const targetRoot = resolve(packageRoot, "dist/vendor/@hongdown/wasm");

async function main() {
  const sourcePackage = JSON.parse(
    await readFile(resolve(wasmRoot, "package.json"), "utf8"),
  );

  await mkdir(targetRoot, { recursive: true });
  await cp(sourceDist, resolve(targetRoot, "dist"), {
    recursive: true,
    force: true,
  });

  await writeFile(
    resolve(targetRoot, "package.json"),
    JSON.stringify(
      {
        type: "module",
        imports: sourcePackage.imports,
      },
      null,
      2,
    ) + "\n",
  );
}

await main();
