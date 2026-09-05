# Testing

## Required checks

| Layer              | Command / workflow                        | Requires                                    | Protects                                                                                           |
| ------------------ | ----------------------------------------- | ------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| PR and main        | npm run check / Checks                    | Windows C++ tools, Node 22.18+, pinned Rust | Formatting, types/build, locale placeholders, versions, Rust behavior, selection, resource map     |
| Prepared libraries | npm run test:resources                    | resources:prepare                           | Repeated PDF imports in one process                                                                |
| Packaging          | npm run installer; npm run installer:test | Prepared libraries and NSIS build           | Actual resource destinations and hashes, CPU executable loading, settings/model/document retention |
| Real inference     | smoke example below                       | Model weights and selected engine           | Multimodal inference and image quality on a real runtime                                           |

Checks runs for pull requests (including forks), main pushes and merge groups with read-only permissions and no secrets. It neither publishes nor downloads weights. A first-time fork contributor may need the maintainer's workflow approval under GitHub repository settings. Use **Source and behavior** as a required branch check if branch protection is enabled.

The tag release workflow runs the source gate, PDF test and packaging gate before publishing. **Packaging validation** offers the same packaging checks through workflow_dispatch on a selected branch, without a tag or release. Run it when touching native resources, Tauri configuration, installer code or import libraries. GPU downloads are not part of these gates.

The test:rust runner disables Tauri resource copying only for that test process. The production configuration, PDF test and installer checks still require the full resource bundle. This lets a fresh source checkout run behavior tests without fabricating empty DLLs.

## What to keep, add and remove

- Keep behavior tests for persistence/recovery, user-data ownership/deletion, resume/hash verification, cancellation, output sanitization, tables, export, layout coordinates and model settings. These protect real failures.
- Keep unit tests beside their Rust module or frontend domain. Build-tool tests sit beside the script. No new test framework or machine-specific model cache is required.
- Add a focused regression for meaningful behavior fixes, including failure and recovery where data could be lost. Fixtures must describe external input independently: never generate test inputs from the validator's own required-file list.
- Test common adapter contracts through the model registry; keep Paddle-specific normalization and fixture expectations in its module. A new model should not have to produce an old model's exact prose.
- Keep expensive/external tests explicit. The PDF test is ignored by default, named in test:resources and required by packaging/release. Document prerequisites for new ignored tests and add their invocation to the appropriate gate; do not silently skip missing dependencies there.
- Remove tests only when the protected behavior is intentionally removed or another test covers the same risk. Explain the replacement in the PR. Avoid checks for static strings, private helper ordering, exact screenshot pixel positions or implementation-shaped fixtures.
- Avoid duplicate compilation/check commands. npm run check is the shared local and CI source gate. Installer helper unit tests run there; installer:test exercises executable integration.

## GPU and image regressions

The previous installer harness omitted almost all payload files. It could pass when Tauri mapped one MSVC source DLL to only one of two required destinations. It now installs the production resource payload into isolated directories, checks each destination against its source hash and starts the bundled CPU engine with a minimal PATH. App identity and the main executable are substituted for isolation; WebView2 installation is skipped.

A resource-map test reproduces overlapping source mappings. Runtime-cache tests cover missing DLL diagnostics, completed archive retention after failure, successful retry, corruption and repair. Image tests verify pixels and alpha compositing; crop tests verify context, scaling and page boundaries. Multimodal tests verify weights plus projector for registered models, CPU/GPU flags and byte-preserving PNG/JPEG request payloads.

Batch-removal tests verify saved list changes, shared-source retention and rejection of mixed valid/unknown IDs before mutation. Preference tests verify that legacy files and IPC input cannot change the build-defined update repository while language and update-check preferences are preserved.

The former dated VALIDATION.md mixed one-machine observations with repeatable gates. This document replaces it; historical test counts and hardware timings are not release criteria.

## Manual inference and quality comparisons

Prepare libraries, then run from the repository root:

    cargo run --locked --manifest-path src-tauri/Cargo.toml --example smoke -- --prepare-runtime vulkan

Choose cuda to exercise its two-archive installation. Optional DATA_DIR and CANCEL_AFTER_BYTES arguments use an isolated cache and test restart/resume. FASTFILEOCR_SMOKE_RESOURCES can point at an installed app's resources directory to test DLL staging against its actual payload.

    cargo run --locked --manifest-path src-tauri/Cargo.toml --example smoke -- docs/assets/sample-invoice.pdf cpu text

The example performs whole-page OCR by default; append --layout for the same region path used by the app. FASTFILEOCR_SMOKE_MODELS can reuse an existing model cache. Compare region detection on/off on the same input, keeping mode, instructions, device and token limit fixed. For small outlined Korean text, include scene-text images as well as normal documents. Existing JPEG scan pages require reimport to recover edges from the source.

Record input provenance, runtime/model revision, settings, raw text, region boxes and warnings. Treat exact model text as an evaluation result, not a deterministic PR assertion. PNG plus padding does not establish an accuracy gain without representative comparisons.

## Limits of automation

These checks do not replace a clean-Windows-VM check for first-time WebView2 installation, interactive wizard navigation, clipboard gestures or real GPU driver compatibility. DLL/hash validation uses production NSIS resource lines in an isolated harness.

Workflow behavior: [GitHub pull_request documentation](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#pull_request).

## Dependency updates

Dependabot checks npm, both Cargo packages and GitHub Actions weekly. Minor/patch version updates are grouped by ecosystem; major upgrades and the pinned Tauri CLI remain separate for review. No auto-merge is configured. Tauri CLI changes must also update/review the NSIS template pin and pass packaging validation.

Enable Dependabot alerts and security updates in the repository security settings if they are not already enabled. The YAML config schedules version updates; it does not itself enable those repository settings. Downloaded llama.cpp, CUDA, PDFium and ONNX binaries and model revisions are outside package-manager coverage and still need maintainer review.

[Dependabot configuration](https://docs.github.com/en/code-security/reference/supply-chain-security/dependabot-options-reference) · [Security updates](https://docs.github.com/en/code-security/concepts/supply-chain-security/dependabot-security-updates).
