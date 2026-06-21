import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { join } from "node:path";

import {
  buildCliInvocation,
  ensureTrustedBackend,
  findMinimalReplacement,
  resolveConfigPath,
  shouldEnableMdx,
} from "../src/core.ts";

describe("findMinimalReplacement", () => {
  it("returns an empty edit for identical content", () => {
    assert.equal(findMinimalReplacement("same\n", "same\n"), null);
  });

  it("returns the smallest changed slice", () => {
    assert.deepEqual(findMinimalReplacement("abc def ghi", "abc XYZ ghi"), {
      start: 4,
      end: 7,
      text: "XYZ",
    });
  });
});

describe("resolveConfigPath", () => {
  it("uses an explicit workspace-relative config path", () => {
    assert.equal(
      resolveConfigPath("config/.hongdown.toml", "/workspace/project"),
      join("/workspace/project", "config/.hongdown.toml"),
    );
  });

  it("uses the workspace root .hongdown.toml by default", () => {
    assert.equal(
      resolveConfigPath("", "/workspace/project"),
      join("/workspace/project", ".hongdown.toml"),
    );
  });
});

describe("shouldEnableMdx", () => {
  it("enables MDX for mdx language and file extensions", () => {
    assert.equal(shouldEnableMdx("markdown", "/docs/post.mdx"), true);
    assert.equal(shouldEnableMdx("mdx", "/docs/post.md"), true);
  });

  it("does not enable MDX for ordinary Markdown", () => {
    assert.equal(shouldEnableMdx("markdown", "/docs/post.md"), false);
  });
});

describe("buildCliInvocation", () => {
  it("builds stdin CLI arguments with config and MDX", () => {
    assert.deepEqual(
      buildCliInvocation({
        cliPath: "hongdown",
        configPath: "/workspace/.hongdown.toml",
        mdx: true,
      }),
      {
        command: "hongdown",
        args: ["--stdin", "--config", "/workspace/.hongdown.toml", "--mdx"],
      },
    );
  });
});

describe("ensureTrustedBackend", () => {
  it("allows WASM in untrusted workspaces", () => {
    assert.equal(ensureTrustedBackend("wasm", false), "wasm");
  });

  it("rejects CLI in untrusted workspaces", () => {
    assert.throws(
      () => ensureTrustedBackend("cli", false),
      /requires a trusted workspace/i,
    );
  });
});
