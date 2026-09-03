# Validation record

Date: September 3, 2026. Windows x64 development machine; NVIDIA RTX 4080 SUPER, driver 610.74. This record describes observed checks, not a general accuracy benchmark.

## Source and packaging

- TypeScript checks and the Vite production build pass. Markdown rendering is a separate lazy-loaded chunk.
- Rust checks, formatting and 21 unit tests pass.
- English, Korean and Japanese catalog keys, placeholders and source references pass validation.
- The bundle verifier checks 177 prepared files and excludes model weights. CUDA, CPU, Vulkan, PDFium and ONNX Runtime are included.
- GitHub Actions YAML parses; tag versioning, release permissions, current-version asset selection and the installer data test gate are configured.
- Model weights, prepared runtimes, build outputs and local workspaces are excluded from Git.

Rust tests cover persistence/recovery, traversal protection, partial downloads, layout coordinates and reading order, raw text preservation, HTML sanitization, table normalization, structured export, model settings, preference migration, fresh-start behavior and repeated PDF imports.

## App checks

- The running Tauri app switches between English, Korean and Japanese and persists the language and device selection.
- Clicking a selected document again deselects it. Ctrl selection, Shift range and Ctrl+A were exercised in the native app.
- The sample PDF can be imported twice in one process. This caught and fixed PDFium's duplicate-bind initialization error.
- CUDA whole-page OCR reads the sample invoice, including the title. CUDA region OCR recognizes the second sample page as 11 regions.
- The installed release executable performs CUDA region OCR and exports JSON with FastFileOCR branding, the model id, coordinates and individual region results.
- Model-specific maximum-token settings persist across reinstall alongside CUDA and Korean language settings.
- README screenshots are actual native-app captures using the public synthetic PDF. The 1050×700 Japanese workspace and settings dialog were also inspected for overflow.
- The icon is flat with a true transparent alpha channel; it is used by the app, installer and README.

Earlier runtime verification also exercised CPU and Vulkan OCR, clipboard Ctrl+V, the Tauri file-drop event, selected-page rescans and model download pause/stop/restart/resume. The physical Windows drag gesture was not automated.

## Installer data lifecycle

The installer was installed into an isolated directory. Its selected data location and Japanese initial language were read by the installed app without a test data-path override. The installed app was then changed to Korean/CUDA with independent model options; reinstalling in keep mode preserved settings byte-for-byte and retained both pages.

The executable data test harness uses the production installer procedures with simulated choices and isolated fixture folders. It verifies:

- Fresh settings are backed up before reset; models and workspaces remain.
- Silent uninstall keeps data by default.
- Choosing data removal deletes settings, models, logs, updates and selected managed workspaces.
- Unrelated files and files outside the data folder remain untouched.
- Choosing an ordinary populated parent creates a dedicated FastFileOCR child; it does not claim or erase the parent.
- An occupied, unmarked dedicated child is rejected.
- Silent validation failures do not display message boxes.

The harness caught and fixed a backup-date argument mismatch and incorrect handling of individual files during cleanup. Direct installation also caught an unsupported installer path constant. Those fixes are included in the final installer source.

Run: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/dev/test-installer-data.ps1` after an installer build. The release workflow runs the same gate.

## Scope and limitations

- No GitHub remote is configured in this checkout, so a real tag push, hosted-runner build and end-to-end GitHub update installation have not been executed. Release parsing and installer/checksum selection are covered locally.
- No clean Windows VM was used. WebView2 was already present, so installation of WebView2 on a machine without it was not exercised.
- Native wizard layout/keyboard navigation was not automated because the Windows UI helper could not start. Installer data logic and actual command-line installation were exercised separately.
- General document detection can miss stylized headings or unusual layouts; whole-page rescanning remains available. Comic-specific accuracy and reading order have not been benchmarked.
- Output follows recognized reading order and does not reproduce the exact original page layout. Whole-page responses may contain plain text rather than fully formatted tables.
- Installer outputs are unsigned. Version, file size and SHA-256 are recorded in the generated `build-info.json` beside the EXE.
