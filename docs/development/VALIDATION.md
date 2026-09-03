# Validation record

Date: September 3, 2026. Windows x64 development machine; NVIDIA RTX 4080 SUPER, driver 610.74. This record describes observed checks, not a general accuracy benchmark.

## Source and packaging

- TypeScript checks and the Vite production build pass. Markdown rendering is a separate lazy-loaded chunk.
- Rust checks, formatting and 22 unit tests pass.
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

## NSIS installer data lifecycle

The Tauri NSIS template compiles with English, Korean and Japanese pages. Its data operations use a standalone Rust helper; six tests cover fresh settings backups, reinstall retention, independent deletion choices, unowned folders, traversal, junction rejection and overlapping installation/data paths.

The executable harness uses the rendered production NSIS template, pages, hooks and helper. Only the payload, WebView2 bootstrap and fixture identity are substituted. In isolated directories and registry keys it verifies:

- Fresh settings backups retain downloaded models and documents.
- Installer language and data location are recorded correctly.
- Reinstalling preserves saved settings byte-for-byte.
- Default silent uninstall preserves user data.
- Settings/models and document workspaces can be deleted independently.
- Update-triggered uninstall preserves data even if cleanup flags are present.
- Unmanaged files remain untouched.
- Populated parent folders resolve to a dedicated child; occupied unowned children are rejected without a popup in silent mode.

Run npm run installer:test after building the NSIS installer. The release workflow runs the same gate. The upstream template and CLI versions are pinned together, with a checksum and checked extension anchors.

## Scope and limitations

- No GitHub remote is configured in this checkout, so a real tag push, hosted-runner build and end-to-end GitHub update installation have not been executed. Release parsing and installer/checksum selection are covered locally.
- No clean Windows VM was used. WebView2 was already present, so installation of WebView2 on a machine without it was not exercised.
- Native wizard layout/keyboard navigation was not automated because the Windows UI helper could not start. Installer data logic and actual command-line installation were exercised separately.
- General document detection can miss stylized headings or unusual layouts; whole-page rescanning remains available. Comic-specific accuracy and reading order have not been benchmarked.
- Output follows recognized reading order and does not reproduce the exact original page layout. Whole-page responses may contain plain text rather than fully formatted tables.
- Installer outputs are unsigned. Version, file size and SHA-256 are recorded in the generated `build-info.json` beside the EXE.
