# Adding an OCR model

Implement ModelAdapter in src-tauri/src/models and register it in get() and descriptors(). No UI file should contain the model's weight filenames, prompt templates or output-normalization rules.

Each adapter owns:

- Stable id, display name, supported modes/devices and layout capability.
- Immutable model manifest (repository, revision, sizes, SHA-256).
- Runtime weights, projector, chat template, alias and context size.
- Recognition prompts and result normalization.
- Settings validation and conversion to runtime options.
- A field descriptor list for the model's independent settings UI.

The shared llama.cpp engine manages the process, transport, cancellation and GPU fallback. ModelOptions renders select, number, text and boolean descriptors. Settings.modelOptions stores each model's values under its model id, preserving independent values when switching models.

The existing Paddle adapter uses its own prompts.json and manifest.json. Older project settings without modelId migrate to PaddleOCR-VL 1.6; legacy maxTokens remains a compatibility fallback. New settings are stored per model.

An adapter for a different inference protocol should provide a separate runtime implementation behind the model boundary rather than embedding protocol logic into React. Layout detection remains optional and must be disabled for adapters that do not support it.

All user-facing descriptor labels belong in locate. Run npm run locales:check after adding keys. Preserve raw OCR output even when normalized output is incomplete.
