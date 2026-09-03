// Unit/behavior tests do not package or copy native runtime resources.
// Production and PDF/installer gates use the unmodified Tauri configuration.
import { spawnSync } from "node:child_process";
const config = JSON.parse(process.env.TAURI_CONFIG || "{}");
config.bundle = { ...config.bundle, resources: [] };
const result = spawnSync(
  "cargo",
  [
    "test",
    "--locked",
    "--manifest-path",
    "src-tauri/Cargo.toml",
    "--all-targets",
  ],
  {
    stdio: "inherit",
    env: { ...process.env, TAURI_CONFIG: JSON.stringify(config) },
  },
);
if (result.error) throw result.error;
process.exitCode = result.status ?? 1;
