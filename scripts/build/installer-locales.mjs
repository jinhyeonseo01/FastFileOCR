import { readFile, writeFile, mkdir } from "node:fs/promises";
const languages = { english: "en", korean: "ko", japanese: "ja" };
const keys = [
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
  "installDataSummary",
  "uninstallData",
  "uninstallDocuments",
  "uninstallUnsafe",
  "webviewStartError",
  "webviewInstallError",
  "installerAppRunning",
];
let result = "; Generated from locate/*.json. Do not edit.\n[CustomMessages]\n";
for (const [inno, lang] of Object.entries(languages)) {
  const values = JSON.parse(await readFile("locate/" + lang + ".json", "utf8"));
  for (const key of keys)
    result +=
      inno + "." + key + "=" + values[key].replace(/\r?\n/g, "%n") + "\n";
}
await mkdir(".cache/installer", { recursive: true });
await writeFile(".cache/installer/messages.iss", "\ufeff" + result);
