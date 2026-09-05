// Resolve the directory/file mapping used by the Tauri bundle.
// A single source cannot target two locations: Tauri deduplicates source paths.
import { readdir, stat } from "node:fs/promises";
import path from "node:path";

export async function resourceMap(base, mapping) {
  const result = [],
    sources = new Set(),
    targets = new Set();
  async function add(source, target) {
    const info = await stat(source);
    if (info.isDirectory()) {
      for (const name of await readdir(source))
        await add(path.join(source, name), path.join(target, name));
      return;
    }
    const sourceKey = path.resolve(source).toLowerCase();
    const targetKey = target.replaceAll("\\", "/").toLowerCase();
    if (
      !targetKey ||
      path.isAbsolute(target) ||
      targetKey.split("/").includes("..")
    )
      throw new Error("Unsafe resource destination: " + target);
    if (sources.has(sourceKey))
      throw new Error("Resource source is mapped more than once: " + source);
    if (targets.has(targetKey))
      throw new Error(
        "Resource destination is mapped more than once: " + target,
      );
    sources.add(sourceKey);
    targets.add(targetKey);
    result.push({ source, target });
  }
  for (const [source, target] of Object.entries(mapping))
    await add(path.resolve(base, source), target);
  return result;
}
