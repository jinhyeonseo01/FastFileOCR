# Architecture

FastFileOCR uses a Rust core and Tauri 2 with React. Files, document state, inference, downloads, exports and updates remain native. React handles selection, editing, settings and progress display through the command boundary.

## Scan path

Files / clipboard / drag-and-drop → page images → verified model files → model adapter → local llama.cpp → saved results.

Whole-page recognition is the default. When the user enables region detection, PP-DocLayoutV3 supplies bounding boxes and reading order, then PaddleOCR-VL recognizes each crop. Both raw and normalized output are retained. If no regions are detected, the app falls back to whole-page OCR. A detector can miss individual regions; the original page and rescan controls remain available for review.

The layout model runs through ONNX Runtime on CPU. Its graph is exported from the pinned Transformers implementation and references official Safetensors weights by byte offset. The graph contains no trained weights. Export validation compares PyTorch and ONNX Runtime outputs. Runtime installations require no Python.

llama.cpp provides CPU, Vulkan and CUDA backends. Automatic mode tries CUDA, then Vulkan, then CPU. Explicit device selection reports a startup failure instead of silently changing the selected backend. Sidecars listen only on loopback, require a per-session key, and are owned by the Rust process. A Windows Job Object cleans them up when the parent exits.

## Model boundary

See [MODELS.md](MODELS.md). The adapter owns model manifests, prompts, inference options and normalization. The registry supplies settings descriptors to React. Per-model options remain independent. The current runtime transport is llama.cpp; additional protocols belong behind the native runtime boundary.

## Persistence

The selected data root contains settings.json, models, logs, updates and default workspaces. The installer stores its location in the current-user registry. An ownership marker prevents treating an unrelated directory as application data. Versioned settings preserve supported fields on upgrade and reject unknown future schema versions.

Workspaces hold copied source files, normalized page images, thumbnails and saved results. UUID names and relative-path validation restrict document references to their workspace. Atomic writes and backups protect saved state. Interrupted pages return to the queue; completed pages can be rescanned with different options. Edits are flushed before destructive or scan operations.

## Downloads and exports

The model manifest pins repository revisions, file lengths and SHA-256 values. Partial downloads use HTTP Range with Content-Range validation. Pause, stop and restart retain .part files. Files receive their final names only after verification.

JSON preserves page settings, raw OCR, normalized text, structure and region coordinates. HTML is sanitized. Preview and exported documents cannot execute OCR-provided scripts or load remote images. Export does not attempt exact page-layout reconstruction.

The updater checks the configured repository's latest stable GitHub release. A version-matched installer and checksum asset are required. It verifies the installer after download and before launching it. The user decides when to install and how to retain data.

## Presentation

Visible text is stored in locate/en.json, ko.json and ja.json. Rust emits translation keys and structured parameters; the frontend resolves them in the selected language. Model prompts are separate from UI translations. Installer custom messages are generated from the same catalogs.

Components are separated into sidebar, document editor, lazy-loaded Markdown rendering, settings, model options and notifications. Shared styles define the sky-blue palette and responsive layout. Tooling is grouped under scripts/build, scripts/release and scripts/dev.
