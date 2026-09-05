import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
const json = async (p) => JSON.parse(await readFile(p, "utf8"));
const pkg = await json("package.json");
const lock = await json("package-lock.json");
assert.equal(
  pkg.devDependencies["@tauri-apps/cli"],
  (await json("installer/nsis/upstream.json")).cliVersion,
  "Review the pinned NSIS template when updating Tauri CLI",
);
assert.equal(lock.version, pkg.version, "npm lock version");
assert.equal(
  lock.packages[""].version,
  pkg.version,
  "npm root package version",
);
assert.equal(
  (await json("src-tauri/tauri.conf.json")).version,
  pkg.version,
  "Tauri version",
);
for (const root of ["src-tauri", "installer/helper"]) {
  const manifest = await readFile(root + "/Cargo.toml", "utf8");
  const name = manifest.match(/^name = "([A-Za-z0-9_-]+)"/m)?.[1];
  assert.ok(name, "Cargo package name: " + root);
  assert.equal(
    manifest.match(/^version = "([^"]+)"/m)?.[1],
    pkg.version,
    root + " version",
  );
  const cargoLock = await readFile(root + "/Cargo.lock", "utf8");
  const entries = [
    ...cargoLock.matchAll(
      new RegExp('^name = "' + name + '"\\r?\\nversion = "([^"]+)"', "gm"),
    ),
  ];
  assert.equal(entries.length, 1, "Cargo lock package: " + root);
  assert.equal(entries[0][1], pkg.version, root + " lock version");
}
console.log(
  "Application, installer and lockfile versions match: " + pkg.version,
);
