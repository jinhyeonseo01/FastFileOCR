# Visual assets

The FastFileOCR icon was generated with Image Gen and refined with the imagegen skill on September 3, 2026. The final design uses a flat sky-blue scan frame, a white document and a forward arrow. Its transparent alpha channel was verified before integration.

Final edit brief: “Fix only the background. Remove the checkerboard pattern completely and use real transparent alpha. Keep the flat sky-blue scanner brackets, white document, three blue text strokes and forward arrow. No background rectangle, shadow, gradient or 3D effects.”

The generated PNG was converted into application sizes using `tauri icon`. The distributed copies are `docs/assets/icon.png`, `public/icon.png`, and `src-tauri/icons/icon.png` / `icon.ico`. Intermediate files are not part of the repository.

The README screenshots are captures of the running Windows Tauri application, using the synthetic two-page `docs/assets/sample-invoice.pdf`. Recreate the sample with `node scripts/dev/make-fixtures.mjs`. It contains no personal documents. Screenshot content reflects actual OCR results.
