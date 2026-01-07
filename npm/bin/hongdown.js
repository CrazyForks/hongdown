#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);

const PLATFORMS = {
  "darwin-arm64": "@hongdown/darwin-arm64",
  "darwin-x64": "@hongdown/darwin-x64",
  "linux-arm64": "@hongdown/linux-arm64",
  "linux-x64": "@hongdown/linux-x64",
  "win32-arm64": "@hongdown/win32-arm64",
  "win32-x64": "@hongdown/win32-x64",
};

const platformKey = `${process.platform}-${process.arch}`;
const packageName = PLATFORMS[platformKey];

if (!packageName) {
  console.error(`Unsupported platform: ${platformKey}`);
  process.exit(1);
}

try {
  const binName = process.platform === "win32" ? "hongdown.exe" : "hongdown";
  const binPath = require.resolve(`${packageName}/bin/${binName}`);
  const result = spawnSync(binPath, process.argv.slice(2), {
    stdio: "inherit",
  });
  process.exit(result.status ?? 1);
} catch (e) {
  console.error(`Failed to run hongdown: ${e.message}`);
  process.exit(1);
}
