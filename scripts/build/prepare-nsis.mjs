import { readFile, writeFile, mkdir, copyFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import path from "node:path";
const root = path.resolve(import.meta.dirname, "../..");
process.chdir(root);
const pin = JSON.parse(await readFile("installer/nsis/upstream.json", "utf8"));
const cli = JSON.parse(
  await readFile("node_modules/@tauri-apps/cli/package.json", "utf8"),
);
if (cli.version !== pin.cliVersion)
  throw new Error(
    "Review NSIS template extensions before updating the pinned Tauri CLI.",
  );
await mkdir(".cache/installer", { recursive: true });
const cache = ".cache/installer/upstream.nsi";
let upstream = await readFile(cache).catch(() => null);
const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");
if (!upstream || hash(upstream) !== pin.sha256) {
  const response = await fetch(pin.url);
  if (!response.ok)
    throw new Error("Could not fetch the pinned Tauri NSIS template.");
  upstream = Buffer.from(await response.arrayBuffer());
  if (hash(upstream) !== pin.sha256)
    throw new Error("Tauri NSIS template checksum mismatch.");
  await writeFile(cache, upstream);
}
let template = upstream.toString("utf8").replace(/\r\n/g, "\n");
function replaceOnce(old, next) {
  if (template.split(old).length !== 2)
    throw new Error("NSIS extension anchor changed: " + old.slice(0, 80));
  template = template.replace(old, () => next);
}
// Keep upstream application installation, WebView2 handling and version management.
// Only insert our pages and replace upstream's broad app-data cleanup.
const hook =
  '{{#if installer_hooks}}\n!include "{{installer_hooks}}"\n{{/if}}\n';
replaceOnce(hook, "");
replaceOnce(
  "; Installer pages, must be ordered as they appear",
  hook + "\n; Installer pages, must be ordered as they appear",
);
replaceOnce(
  "!insertmacro MUI_PAGE_DIRECTORY",
  "!insertmacro MUI_PAGE_DIRECTORY\nPage custom FfoDataPage FfoDataLeave\nPage custom FfoModePage FfoModeLeave",
);
const unstart = template.indexOf("; Uninstaller Pages\n");
const unend = template.indexOf("; 2. Uninstalling Page\n");
if (unstart < 0 || unend <= unstart)
  throw new Error("Uninstall page anchor changed.");
template =
  template.slice(0, unstart) +
  "; Uninstaller Pages\n!define MUI_PAGE_CUSTOMFUNCTION_PRE un.SkipIfPassive\n!insertmacro MUI_UNPAGE_CONFIRM\nUninstPage custom un.FfoDataPage un.FfoDataLeave\n\n" +
  template.slice(unend);
const cleanupStart = template.indexOf(
  "  ; Delete app data if the checkbox is selected\n",
);
const cleanupEnd = template.indexOf("  !ifmacrodef NSIS_HOOK_POSTUNINSTALL\n");
if (cleanupStart < 0 || cleanupEnd <= cleanupStart)
  throw new Error("Uninstall cleanup anchor changed.");
template =
  template.slice(0, cleanupStart) +
  "  ; Data cleanup is handled by the checked, opt-in Rust helper.\n" +
  template.slice(cleanupEnd);

replaceOnce(
  '  !if "${DISPLAYLANGUAGESELECTOR}" == "true"\n    !insertmacro MUI_LANGDLL_DISPLAY\n  !endif',
  '  Call FfoSelectLanguage\n  !if "${DISPLAYLANGUAGESELECTOR}" == "true"\n    ${If} $FfoExplicitLanguage != 1\n      !insertmacro MUI_LANGDLL_DISPLAY\n    ${EndIf}\n  !endif',
);
replaceOnce(
  'StrCpy $INSTDIR "$LOCALAPPDATA\\${PRODUCTNAME}"',
  'StrCpy $INSTDIR "$LOCALAPPDATA\\Programs\\${PRODUCTNAME}"',
);
replaceOnce(
  "FunctionEnd\n\n\nSection EarlyChecks",
  "  Call FfoInit\nFunctionEnd\n\n\nSection EarlyChecks",
);
replaceOnce(
  '{{#each language_files}}\n  !include "{{this}}"\n{{/each}}',
  '{{#each language_files}}\n  !include "{{this}}"\n{{/each}}\n!include "' +
    path.join(root, ".cache/installer/messages.nsh") +
    '"',
);
// An uninstall initiated by a reinstall must never offer to delete user data.
replaceOnce(
  '      ${IfThen} $UpdateMode = 1 ${|} StrCpy $R1 "$R1 /UPDATE" ${|} ; append /UPDATE',
  '      StrCpy $R1 "$R1 /UPDATE" ; retain user data during reinstall',
);
// Do not modify user data until the application has exited.
for (const name of ["PREINSTALL", "PREUNINSTALL"]) {
  const block =
    "  !ifmacrodef NSIS_HOOK_" +
    name +
    "\n    !insertmacro NSIS_HOOK_" +
    name +
    "\n  !endif\n\n";
  replaceOnce(
    block +
      '  !insertmacro CheckIfAppIsRunning "${MAINBINARYNAME}.exe" "${PRODUCTNAME}"',
    '  !insertmacro CheckIfAppIsRunning "${MAINBINARYNAME}.exe" "${PRODUCTNAME}"\n\n' +
      block.trimEnd(),
  );
}
await writeFile(".cache/installer/installer.nsi", template);
await import("./installer-locales.mjs");
if (!process.argv.includes("--template-only")) {
  // The setup helper must run before application runtimes have been installed.
  execFileSync(
    "cargo",
    [
      "build",
      "--locked",
      "--release",
      "--manifest-path",
      "installer/helper/Cargo.toml",
      "--target-dir",
      ".cache/installer-helper-target",
    ],
    {
      stdio: "inherit",
      env: { ...process.env, RUSTFLAGS: "-C target-feature=+crt-static" },
    },
  );
  await mkdir("src-tauri/resources/installer", { recursive: true });
  await copyFile(
    ".cache/installer-helper-target/release/fastfileocr-setup-helper.exe",
    "src-tauri/resources/installer/fastfileocr-setup-helper.exe",
  );
}
console.log(
  "Prepared Tauri NSIS template, localized pages and Rust data helper.",
);
