import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { resourceMap } from "./resource-map.mjs";

test("bundles reject a source shared by a directory and a root DLL mapping", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "ffocr-resources-"));
  try {
    await mkdir(path.join(root, "runtime"));
    await writeFile(path.join(root, "runtime", "crt.dll"), "crt");
    await assert.rejects(
      resourceMap(root, { runtime: "runtime", "runtime/crt.dll": "crt.dll" }),
      /mapped more than once/,
    );
    await writeFile(path.join(root, "app-crt.dll"), "crt");
    const files = await resourceMap(root, {
      runtime: "runtime",
      "app-crt.dll": "crt.dll",
    });
    assert.deepEqual(files.map((f) => f.target.replaceAll("\\", "/")).sort(), [
      "crt.dll",
      "runtime/crt.dll",
    ]);
    await assert.rejects(
      resourceMap(root, { "app-crt.dll": "../outside.dll" }),
      /Unsafe/,
    );
    await assert.rejects(
      resourceMap(root, {
        "app-crt.dll": "CRT.dll",
        "runtime/crt.dll": "crt.dll",
      }),
      /destination/,
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
