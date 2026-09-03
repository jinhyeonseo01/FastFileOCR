import { readFile, writeFile } from "node:fs/promises";
const tag = process.argv[2] || process.env.GITHUB_REF_NAME;
if (!/^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(tag || ""))
  throw new Error("Expected a release tag such as v1.0.0");
const version = tag.slice(1);
for (const file of [
  "package.json",
  "package-lock.json",
  "src-tauri/tauri.conf.json",
]) {
  const value = JSON.parse(await readFile(file, "utf8"));
  value.version = version;
  if (file === "package-lock.json") value.packages[""].version = version;
  await writeFile(file, JSON.stringify(value, null, 2) + "\n");
}
for (const directory of ["src-tauri", "installer/helper"]) {
  const manifest = directory + "/Cargo.toml";
  let cargo = await readFile(manifest, "utf8");
  const name = cargo.match(/^name = "([A-Za-z0-9_-]+)"/m)?.[1];
  if (!name) throw new Error("Missing package name: " + manifest);
  cargo = cargo.replace(/^version = "[^"]+"/m, 'version = "' + version + '"');
  await writeFile(manifest, cargo);
  const file = directory + "/Cargo.lock";
  let lock = await readFile(file, "utf8");
  const pattern = new RegExp(
    '(name = "' + name + '"\\r?\\nversion = ")[^"]+',
    "g",
  );
  if ([...lock.matchAll(pattern)].length !== 1)
    throw new Error("Expected one root package in " + file);
  lock = lock.replace(pattern, (_, prefix) => prefix + version);
  await writeFile(file, lock);
}
console.log("Building FastFileOCR " + version + " from " + tag);
