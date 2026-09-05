# Contributing

## Pull Request Guidelines

The use of AI is not restricted. However, all Pull Requests must meet the following requirements:

- For new features or structural changes, **open an Issue first, explain the necessity and purpose, and proceed only after sufficient discussion**.
- Follow the existing **coding conventions, project structure, architecture, and design principles**.
- Keep changes to the **minimum scope necessary** to fulfill the intended requirement.
- Changes should be **non-destructive** and must not break existing functionality or interfaces.
- Avoid unrelated **refactoring, formatting changes, file moves, or dependency changes**.
- When adding or modifying functionality, **add relevant tests where appropriate and ensure existing tests pass**.
- Keep each Pull Request focused on **a single purpose whenever possible**.
- Clearly describe the **purpose, key changes, impact, and testing method** in the Pull Request.
- Regardless of whether AI was used, the contributor must **review and understand the submitted changes and remains fully responsible for them**.

## Checks before a PR

Run **npm ci** and **npm run check** on Windows with Node.js 22.18+ and the pinned Rust toolchain. This gate needs no OCR weights, GPU, Python or prepared runtime downloads.

Keep tests close to the behavior they protect. Prefer small synthetic fixtures and observable outcomes over source-code matching, private implementation details or exact model-generated text. New adapters should follow the shared adapter contract, with model-specific cases in that adapter.

Changes to imports, native libraries or installation also need the packaging checks in [Testing](docs/development/TESTING.md). For OCR quality changes, attach the input (if shareable), settings and before/after results.
