import { readFile, readdir, writeFile, mkdir } from "node:fs/promises";
import { execFileSync } from "node:child_process";
import path from "node:path";
const chunks = [
  "FastFileOCR — third-party software notices\nGenerated from locked dependencies. Model and native runtime notices are separate.\n",
];
const upstreamCache = new Map();
async function append(name, version, license, root, extra, repo) {
  const names = (await readdir(root)).filter((n) =>
    /^(licen[cs]e|copying|notice)([._-]|$)/i.test(n),
  );
  if (extra && !names.includes(extra)) names.push(extra);
  const texts = [];
  for (const n of names) {
    try {
      texts.push(n + "\n" + (await readFile(path.resolve(root, n), "utf8")));
    } catch {}
  }
  if (!texts.length && repo?.includes("github.com/")) {
    let vcs = {};
    try {
      vcs = JSON.parse(
        await readFile(path.join(root, ".cargo_vcs_info.json"), "utf8"),
      );
    } catch {}
    const repository = repo.replace(/\.git\/?$/, "").replace(/\/$/, "");
    const revision = vcs.git?.sha1 || "HEAD";
    const cacheKey = repository + "/" + revision;
    if (!upstreamCache.has(cacheKey))
      upstreamCache.set(
        cacheKey,
        (async () => {
          const prefix =
            repository.replace(
              "https://github.com/",
              "https://raw.githubusercontent.com/",
            ) +
            "/" +
            revision +
            "/";
          const found = await Promise.all(
            [
              "LICENSE",
              "LICENSE.md",
              "LICENSE.txt",
              "LICENSE-MIT",
              "LICENSE-APACHE",
              "COPYING",
            ].map(async (n) => {
              try {
                const r = await fetch(prefix + n);
                return r.ok ? n + "\n" + (await r.text()) : null;
              } catch {
                return null;
              }
            }),
          );
          return found.filter(Boolean).join("\n\n");
        })(),
      );
    const text = await upstreamCache.get(cacheKey);
    if (text) texts.push(text);
  }
  if (!texts.length && /Apache-2.0/.test(license || "")) {
    const headers = [];
    for (const f of ["lib.rs", "src/lib.rs"]) {
      try {
        headers.push(
          (await readFile(path.join(root, f), "utf8"))
            .split("\n")
            .slice(0, 15)
            .filter((line) => /^\s*\/\//.test(line))
            .join("\n"),
        );
      } catch {}
    }
    texts.push(
      headers.join("\n") +
        "\n" +
        (await readFile(
          "src-tauri/resources/licenses/Transformers-Apache-2.0.txt",
          "utf8",
        )),
    );
  }
  if (!texts.length && (license || "").includes("MPL-2.0")) {
    const r = await fetch(
      "https://raw.githubusercontent.com/spdx/license-list-data/v3.27.0/text/MPL-2.0.txt",
    );
    if (!r.ok) throw new Error("Cannot obtain MPL license text");
    texts.push(
      (await r.text()) +
        "\nUnmodified source: https://crates.io/api/v1/crates/" +
        name +
        "/" +
        version +
        "/download\n",
    );
  }
  chunks.push(
    "\n" +
      "=".repeat(78) +
      "\n" +
      name +
      " " +
      version +
      "\nLicense: " +
      (license || "See source notices") +
      "\nSource: " +
      (repo || "npm registry / crates.io") +
      "\n" +
      texts.join("\n\n"),
  );
  if (!texts.length)
    throw new Error("Missing license text: " + name + " " + version);
}
const lock = JSON.parse(await readFile("package-lock.json", "utf8"));
for (const [key, pkg] of Object.entries(lock.packages)) {
  if (!key || pkg.dev) continue;
  const dir = path.resolve(key),
    p = JSON.parse(await readFile(path.join(dir, "package.json"), "utf8"));
  await append(
    p.name,
    p.version,
    p.license,
    dir,
    null,
    typeof p.repository === "string" ? p.repository : p.repository?.url,
  );
}
const metadata = JSON.parse(
  execFileSync(
    "cargo",
    [
      "metadata",
      "--locked",
      "--format-version",
      "1",
      "--filter-platform",
      "x86_64-pc-windows-msvc",
      "--manifest-path",
      "src-tauri/Cargo.toml",
    ],
    { encoding: "utf8", maxBuffer: 30 * 1024 * 1024 },
  ),
);
const used = new Set(metadata.resolve.nodes.map((n) => n.id));
for (const pkg of metadata.packages) {
  if (metadata.workspace_members.includes(pkg.id) || !used.has(pkg.id))
    continue;
  await append(
    pkg.name,
    pkg.version,
    pkg.license,
    path.dirname(pkg.manifest_path),
    pkg.license_file,
    pkg.repository,
  );
}
await mkdir("src-tauri/resources/licenses", { recursive: true });
await writeFile(
  "src-tauri/resources/licenses/THIRD-PARTY-NOTICES.txt",
  chunks.join("\n"),
  "utf8",
);
console.log("Collected complete third-party software notices.");
