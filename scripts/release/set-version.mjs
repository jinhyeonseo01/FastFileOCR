import { readFile, writeFile } from "node:fs/promises";
const tag = process.argv[2] || process.env.GITHUB_REF_NAME;
if (!/^v\d+\.\d+\.\d+$/.test(tag || ""))
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
let cargo = await readFile("src-tauri/Cargo.toml", "utf8");
cargo = cargo.replace(/^version = "[^"]+"/m, 'version = "' + version + '"');
await writeFile("src-tauri/Cargo.toml", cargo);
let lock = await readFile("src-tauri/Cargo.lock", "utf8");
lock = lock.replace(
  /(name = "glyph-ocr"\r?\nversion = ")[^"]+/,
  (_, prefix) => prefix + version,
);
await writeFile("src-tauri/Cargo.lock", lock);
console.log("Building FastFileOCR " + version + " from " + tag);
