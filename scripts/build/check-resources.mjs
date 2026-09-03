import { createReadStream } from "node:fs";
import { readFile, readdir, stat } from "node:fs/promises";
import { createHash } from "node:crypto";
import path from "node:path";
const root = path.resolve("src-tauri/resources");
async function rejectModelWeights(directory) {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const target = path.join(directory, entry.name);
    if (entry.isDirectory()) await rejectModelWeights(target);
    else if (/\.(gguf|safetensors|part)$/i.test(entry.name))
      throw new Error(
        "Model weights must not be bundled: " + path.relative(root, target),
      );
  }
}
try {
  await rejectModelWeights(root);
  const manifest = JSON.parse(
    (await readFile(path.join(root, "bundle-manifest.json"), "utf8")).replace(
      /^\uFEFF/,
      "",
    ),
  );
  for (const required of [
    "runtime/cpu/llama-server.exe",
    "runtime/cuda/llama-server.exe",
    "runtime/cuda/ggml-cuda.dll",
    "runtime/vulkan/llama-server.exe",
    "runtime/pdfium/pdfium.dll",
    "chat-template.jinja",
    "licenses/PaddleOCR-Apache-2.0.txt",
    "licenses/llama.cpp-MIT.txt",
    "licenses/PDFium.txt",
    "layout/layout.onnx",
    "layout/labels.json",
    "runtime/onnxruntime/onnxruntime.dll",
    "runtime/msvc/vcruntime140.dll",
    "runtime/msvc/vcruntime140_1.dll",
    "runtime/cpu/msvcp140.dll",
    "runtime/vulkan/msvcp140.dll",
    "licenses/LLVM-OpenMP.txt",
    "licenses/THIRD-PARTY-NOTICES.txt",
  ]) {
    if (!manifest.files.some((f) => f.path === required))
      throw new Error("Missing bundle asset: " + required);
  }
  for (const file of manifest.files) {
    if (/\.(gguf|safetensors|part)$/i.test(file.path))
      throw new Error("Model weights must not be bundled: " + file.path);
    const target = path.resolve(root, file.path);
    if (!target.startsWith(root + path.sep))
      throw new Error("Invalid resource path");
    if ((await stat(target)).size !== file.bytes)
      throw new Error("Size mismatch: " + file.path);
    const hash = createHash("sha256");
    for await (const chunk of createReadStream(target)) hash.update(chunk);
    if (hash.digest("hex") !== file.sha256)
      throw new Error("Hash mismatch: " + file.path);
  }
  console.log(
    "Verified " +
      manifest.files.length +
      " bundled files (" +
      manifest.llama +
      ").",
  );
} catch (error) {
  console.error(error.message + "\nRun npm run resources:prepare first.");
  process.exitCode = 1;
}
