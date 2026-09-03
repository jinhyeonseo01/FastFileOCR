import { readFile, writeFile, mkdir } from "node:fs/promises";
const languages = { ENGLISH: "en", KOREAN: "ko", JAPANESE: "ja" };
const keys = [
  "installWindowsRequired",
  "installDataTitle",
  "installDataDescription",
  "installDataHint",
  "installModeTitle",
  "installModeDescription",
  "installModeHint",
  "installKeep",
  "installFresh",
  "installUnsafe",
  "installWriteError",
  "installResetConfirm",
  "installBrowse",
  "installDataResolved",
  "uninstallDataTitle",
  "uninstallDataHint",
  "uninstallData",
  "uninstallDocuments",
  "uninstallUnsafe",
];
const escape = (value) =>
  value
    .replaceAll("$", () => "$")
    .replaceAll('"', '$\\"')
    .replace(/\r?\n/g, "$\\r$\\n");
let result = "; Generated from locate/*.json. Do not edit.\n";
for (const [nsis, lang] of Object.entries(languages)) {
  const values = JSON.parse(await readFile("locate/" + lang + ".json", "utf8"));
  for (const key of keys) {
    if (!values[key])
      throw new Error("Missing installer translation: " + lang + "/" + key);
    result +=
      "LangString " +
      key +
      " ${LANG_" +
      nsis +
      '} "' +
      escape(values[key]) +
      '"\n';
  }
}
await mkdir(".cache/installer", { recursive: true });
await writeFile(".cache/installer/messages.nsh", "\ufeff" + result);
