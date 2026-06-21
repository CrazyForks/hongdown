import { spawn } from "node:child_process";
import { access, readFile } from "node:fs/promises";
import { dirname } from "node:path";

import * as vscode from "vscode";
import type { FormatOptions, Warning } from "@hongdown/wasm";

import {
  buildCliInvocation,
  ensureTrustedBackend,
  findMinimalReplacement,
  resolveConfigPath,
  shouldEnableMdx,
  type HongdownBackend,
} from "./core.js";
import { loadHongdownWasm } from "./wasm.js";

const outputChannel = vscode.window.createOutputChannel("Hongdown");
const warnedConfigPaths = new Set<string>();

interface FormatterSettings {
  backend: HongdownBackend;
  cliPath: string;
  configuredConfigPath: string;
}

interface ResolvedConfigPath {
  path?: string;
  explicit: boolean;
}

export function activate(context: vscode.ExtensionContext): void {
  const selector: vscode.DocumentSelector = [
    { language: "markdown", scheme: "file" },
    { language: "mdx", scheme: "file" },
    { pattern: "**/*.md", scheme: "file" },
    { pattern: "**/*.markdown", scheme: "file" },
    { pattern: "**/*.mdx", scheme: "file" },
  ];

  context.subscriptions.push(
    vscode.languages.registerDocumentFormattingEditProvider(
      selector,
      new HongdownDocumentFormatter(),
    ),
    outputChannel,
  );
}

export function deactivate(): void {
  // Nothing to dispose. VS Code disposes registered subscriptions.
}

class HongdownDocumentFormatter implements vscode.DocumentFormattingEditProvider {
  async provideDocumentFormattingEdits(
    document: vscode.TextDocument,
    _options: vscode.FormattingOptions,
    token: vscode.CancellationToken,
  ): Promise<vscode.TextEdit[]> {
    if (token.isCancellationRequested) return [];

    const settings = getFormatterSettings(document);
    const workspaceFolder = vscode.workspace.getWorkspaceFolder(document.uri);
    const workspaceRoot = workspaceFolder?.uri.fsPath;
    const documentDir = dirname(document.uri.fsPath);
    const configRoot = workspaceRoot ?? documentDir;
    const configPath = await getConfigPath(settings, configRoot);
    const mdx = shouldEnableMdx(document.languageId, document.fileName);
    const input = document.getText();

    const backend = ensureTrustedBackend(
      settings.backend,
      vscode.workspace.isTrusted,
    );
    const formatted =
      backend === "cli"
        ? await formatWithCli(input, {
            cliPath: settings.cliPath,
            configPath: configPath.path,
            mdx,
            cwd: workspaceRoot ?? documentDir,
            token,
          })
        : await formatWithWasm(input, configPath.path, mdx);

    if (token.isCancellationRequested) return [];

    const replacement = findMinimalReplacement(input, formatted);
    if (!replacement) return [];

    return [
      vscode.TextEdit.replace(
        new vscode.Range(
          document.positionAt(replacement.start),
          document.positionAt(replacement.end),
        ),
        replacement.text,
      ),
    ];
  }
}

function getFormatterSettings(document: vscode.TextDocument): FormatterSettings {
  const config = vscode.workspace.getConfiguration("hongdown", document.uri);
  return {
    backend: config.get<HongdownBackend>("backend", "wasm"),
    cliPath: config.get<string>("cli.path", "hongdown") || "hongdown",
    configuredConfigPath: config.get<string>("config.path", ""),
  };
}

async function getConfigPath(
  settings: FormatterSettings,
  workspaceRoot: string | undefined,
): Promise<ResolvedConfigPath> {
  const explicit = settings.configuredConfigPath.trim().length > 0;
  const path = resolveConfigPath(settings.configuredConfigPath, workspaceRoot);
  if (!path) return { explicit };

  try {
    await access(path);
    return { path, explicit };
  } catch (error) {
    if (explicit) {
      throw new Error(`Hongdown config file was not found: ${path}`);
    }
    return { explicit };
  }
}

async function formatWithWasm(
  input: string,
  configPath: string | undefined,
  mdx: boolean,
): Promise<string> {
  const hongdown = await loadHongdownWasm();
  let options: FormatOptions = {};

  if (configPath) {
    const configSource = await readFile(configPath, "utf8");
    const loaded = await hongdown.loadConfigFromToml(configSource);
    options = loaded.options;
    for (const warning of loaded.warnings) {
      logConfigWarning(configPath, warning);
    }
  }

  if (mdx) {
    options = { ...options, mdx: true };
  }

  const result = await hongdown.formatWithWarnings(input, options);
  logFormatWarnings(configPath ?? "<document>", result.warnings);
  return result.output;
}

async function formatWithCli(
  input: string,
  options: {
    cliPath: string;
    configPath?: string;
    mdx: boolean;
    cwd: string;
    token: vscode.CancellationToken;
  },
): Promise<string> {
  const invocation = buildCliInvocation(options);
  return new Promise((resolve, reject) => {
    let settled = false;
    const child = spawn(invocation.command, invocation.args, {
      cwd: options.cwd,
      shell: false,
      windowsHide: true,
    });
    const cancellation = options.token.onCancellationRequested(() => {
      child.kill();
    });

    const cleanup = (): boolean => {
      if (settled) return false;
      settled = true;
      cancellation.dispose();
      return true;
    };
    const resolveOnce = (value: string): void => {
      if (cleanup()) resolve(value);
    };
    const rejectOnce = (error: unknown): void => {
      if (cleanup()) reject(error);
    };

    let stdout = "";
    let stderr = "";

    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.on("error", (error) => {
      rejectOnce(error);
    });
    child.on("close", (code) => {
      if (options.token.isCancellationRequested) {
        resolveOnce("");
        return;
      }
      if (code === 0) {
        if (stderr.trim().length > 0) {
          outputChannel.appendLine(stderr.trimEnd());
        }
        resolveOnce(stdout);
      } else {
        rejectOnce(
          new Error(
            `Hongdown CLI failed with exit code ${code ?? "unknown"}.\n${stderr.trimEnd()}`,
          ),
        );
      }
    });

    child.stdin.end(input);
  });
}

function logConfigWarning(configPath: string, warning: string): void {
  const key = `${configPath}\0${warning}`;
  if (warnedConfigPaths.has(key)) return;
  warnedConfigPaths.add(key);
  outputChannel.appendLine(`${configPath}: warning: ${warning}`);
}

function logFormatWarnings(source: string, warnings: Warning[]): void {
  for (const warning of warnings) {
    outputChannel.appendLine(
      `${source}:${warning.line}: warning: ${warning.message}`,
    );
  }
}
