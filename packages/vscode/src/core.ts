import { isAbsolute, join, normalize } from "node:path";

export type HongdownBackend = "wasm" | "cli";

export interface MinimalReplacement {
  start: number;
  end: number;
  text: string;
}

export interface CliInvocationOptions {
  cliPath: string;
  configPath?: string;
  mdx: boolean;
}

export interface CliInvocation {
  command: string;
  args: string[];
}

export function findMinimalReplacement(
  original: string,
  formatted: string,
): MinimalReplacement | null {
  if (original === formatted) return null;

  let start = 0;
  while (
    start < original.length &&
    start < formatted.length &&
    original[start] === formatted[start]
  ) {
    start += 1;
  }

  let originalEnd = original.length;
  let formattedEnd = formatted.length;
  while (
    originalEnd > start &&
    formattedEnd > start &&
    original[originalEnd - 1] === formatted[formattedEnd - 1]
  ) {
    originalEnd -= 1;
    formattedEnd -= 1;
  }

  return {
    start,
    end: originalEnd,
    text: formatted.slice(start, formattedEnd),
  };
}

export function resolveConfigPath(
  configuredPath: string,
  workspaceRoot?: string,
): string | undefined {
  const trimmed = configuredPath.trim();
  if (trimmed.length > 0) {
    return normalize(
      isAbsolute(trimmed) || !workspaceRoot
        ? trimmed
        : join(workspaceRoot, trimmed),
    );
  }
  return workspaceRoot ? join(workspaceRoot, ".hongdown.toml") : undefined;
}

export function shouldEnableMdx(languageId: string, fileName: string): boolean {
  return languageId.toLowerCase() === "mdx" || /\.mdx$/i.test(fileName);
}

export function buildCliInvocation(options: CliInvocationOptions): CliInvocation {
  const args = ["--stdin"];
  if (options.configPath) {
    args.push("--config", options.configPath);
  }
  if (options.mdx) {
    args.push("--mdx");
  }
  return {
    command: options.cliPath || "hongdown",
    args,
  };
}

export function ensureTrustedBackend(
  backend: HongdownBackend,
  isTrusted: boolean,
): HongdownBackend {
  if (backend === "cli" && !isTrusted) {
    throw new Error("The Hongdown CLI backend requires a trusted workspace.");
  }
  return backend;
}
