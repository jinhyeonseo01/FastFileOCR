import { readFile, readdir } from "node:fs/promises";
import assert from "node:assert/strict";
const en = JSON.parse(await readFile("locate/en.json", "utf8"));
for (const lang of ["ko", "ja"]) {
  const values = JSON.parse(await readFile("locate/" + lang + ".json", "utf8"));
  assert.deepEqual(
    Object.keys(values).sort(),
    Object.keys(en).sort(),
    "Locale keys: " + lang,
  );
  for (const key of Object.keys(en)) {
    assert.ok(values[key].trim(), "Empty translation: " + lang + "/" + key);
    const params = (s) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
    assert.deepEqual(
      params(values[key]),
      params(en[key]),
      "Parameters: " + lang + "/" + key,
    );
  }
}
async function sourceFiles(root) {
  const out = [];
  for (const f of await readdir(root, { withFileTypes: true })) {
    const p = root + "/" + f.name;
    if (f.isDirectory()) out.push(...(await sourceFiles(p)));
    else if (/\.(ts|tsx|rs)$/.test(p)) out.push(p);
  }
  return out;
}
for (const path of [
  ...(await sourceFiles("src")),
  ...(await sourceFiles("src-tauri/src")),
]) {
  const source = await readFile(path, "utf8");
  for (const m of source.matchAll(
    /(?:\bt\(|i18n::(?:text|f|translated)\()\s*["']([\w.]+)["']/g,
  ))
    assert.ok(m[1] in en, path + ": " + m[1]);
}
console.log(
  "Locale keys, interpolation parameters and source references verified.",
);
