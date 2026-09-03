<p align="center">
  <img src="docs/assets/icon.png" width="112" alt="FastFileOCR icon">
</p>

<h1 align="center">FastFileOCR</h1>
<p align="center"><strong>Simple / Fast / Accurate</strong></p>

**FastFileOCR** is a Windows desktop OCR app with a **Rust** core, powered by **PaddleOCR-VL 1.6**. Turn scanned documents, photos and screenshots into editable text, with processing on your own device.

<p align="center">
  <img src="docs/assets/workspace.png" alt="FastFileOCR recognizing a sample invoice">
</p>

## What you can do

- Add PDFs and images by file selection, drag and drop, or **Ctrl+V**.
- Scan a whole page, or enable **Detect regions before OCR** to retain positions and reading order.
- Recognize documents, plain text, tables, formulas and comics.
- Add custom instructions and rescan any page as often as you need.
- Select multiple pages with Ctrl, Shift or Ctrl+A. Click a selected page again to deselect it.
- Review and edit results, then export **TXT, Markdown, HTML or JSON**.

## Get started

1. Download the Windows x64 installer from [Releases](../../releases/latest).
2. Choose your language and a folder for settings, models and workspaces.
3. Open the app, add your files, choose a recognition mode, and scan.

You can try the included [sample document](docs/assets/sample-invoice.pdf).

The first scan downloads approximately **1.82 GB** of OCR models from Hugging Face. Region detection adds approximately **133 MB**. Downloads can be paused, stopped and resumed, including after restarting the app. Once the models are ready, OCR runs offline. Your documents and results are not uploaded.

## Regions and document structure

Enable region detection to inspect text areas, tables and headings directly on the original page. JSON exports include region coordinates, reading order and individual OCR results.

<img src="docs/assets/regions.png" alt="Region detection and structured OCR results">

Region detection can miss stylized titles or unusual layouts; use whole-page OCR when text is missing. Text and HTML exports follow the reading order; they do not reproduce the original page layout exactly. Comic mode uses recognition instructions and a general document detector, so difficult speech bubbles and reading directions may need another pass.

## Make it yours

Choose **English, 한국어 or 日本語** in Settings. English is the default; the language selected during installation is used on the first launch.

Select **Automatic, CPU, Vulkan or NVIDIA CUDA**, choose the OCR model, and adjust its options and instructions. CUDA requires a compatible NVIDIA GPU and driver.

<img src="docs/assets/settings.png" alt="Recognition settings with model selection and device options">

Updates can be checked from Settings, with an optional notification at startup. Installing an update lets you keep existing settings or start fresh with a settings backup. Downloaded models and documents are retained. Uninstallation asks separately whether to remove settings and models, and whether to remove workspaces stored inside the app's data folder.

## Supported files

**PDF, PNG, JPG/JPEG, WEBP and BMP** on Windows 10/11 x64. Export Word or PowerPoint documents to PDF before importing. Encrypted PDFs must be unlocked first.

CPU processing is available without a dedicated graphics card. The installer includes the required runtimes; you do not need Rust, Python, Node.js or LM Studio.

## License

Original project code and documentation are licensed under **[CC BY 4.0](LICENSE)**. See [NOTICE.txt](NOTICE.txt) for attribution. Third-party models and libraries retain their own licenses.

[Build and development guide](docs/development/BUILDING.md)
