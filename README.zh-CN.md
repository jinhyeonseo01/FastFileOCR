<p align="center">
  <a href="README.md">English</a> ·
  <a href="README.ko.md">한국어</a> ·
  <a href="README.ja.md">日本語</a> ·
  <a href="README.zh-CN.md">简体中文</a>
</p>

<p align="center">
  <img src="docs/assets/icon.png" width="112" alt="FastFileOCR">
</p>

<h1 align="center">FastFileOCR</h1>
<p align="center"><strong>Simple / Fast / Accurate</strong></p>

**FastFileOCR** 是一款Windows桌面OCR应用，可将扫描文档、照片和电脑截图转换为可编辑文本。OCR处理在您的电脑上完成。

|              | 支持内容                                                                                                                                 |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------- |
| OCR模型      | **[PaddleOCR-VL 1.6](https://huggingface.co/PaddlePaddle/PaddleOCR-VL-1.6-GGUF)**，使用 **PP-DocLayoutV3** 进行区域检测                  |
| 核心开发语言 | **Rust**                                                                                                                                 |
| 文字识别语言 | 中文、英语、韩语、日语等 — **[模型支持109种语言](https://www.paddleocr.ai/main/en/version3.x/algorithm/PaddleOCR-VL/PaddleOCR-VL.html)** |
| 应用界面语言 | **English、한국어、日本語**                                                                                                              |
| 输入文件     | **PDF、PNG、JPG/JPEG、WEBP、BMP**，以及剪贴板图片                                                                                        |
| 导出格式     | **TXT、Markdown、HTML、JSON**                                                                                                            |
| 支持平台     | **Windows 10/11 x64**                                                                                                                    |
| 运行设备     | **自动、CPU、Vulkan、NVIDIA CUDA**                                                                                                       |

Word和PowerPoint文件请先导出为PDF再导入。加密的PDF需要先解除保护。

<p align="center">
  <img src="docs/assets/workspace.png" alt="FastFileOCR识别示例发票的界面">
</p>

## 主要功能

- 通过选择文件、拖放或 **Ctrl+V** 添加PDF和图片。
- 在OCR前检测区域，保留位置和阅读顺序。**默认已勾选**；取消勾选即可切换为整页OCR。
- 支持文档、纯文本、表格、公式和漫画识别模式。
- 可输入自定义指令，并根据需要多次重新扫描同一页面。
- 使用Ctrl、Shift或Ctrl+A选择多个页面，再次点击已选页面即可取消选择。
- 查看并编辑结果，导出文本或结构化数据。

## 开始使用

1. 从[发行页面](../../releases/latest)下载Windows x64安装程序。
2. 选择安装语言，以及用于保存设置、模型和工作区的文件夹。
3. 打开应用、添加文件、选择识别模式，然后开始扫描。

您也可以先使用附带的[示例文档](docs/assets/sample-invoice.pdf)体验。

首次扫描会从Hugging Face下载约 **1.82 GB** 的OCR模型和约 **133 MB** 的区域检测模型。关闭区域检测时，只需要OCR模型。下载可以暂停或中止，重启应用后也能继续下载。模型准备完成后，OCR可离线运行。文档和识别结果不会上传。

## 区域与文档结构

区域检测默认开启。您可以直接在原始页面上查看文本区域、表格和标题的位置。JSON导出包含区域坐标、阅读顺序和各区域的识别结果。

<img src="docs/assets/regions.png" alt="区域检测与结构化OCR结果">

区域检测可能遗漏装饰性标题或特殊排版。如果发现缺字，可以关闭区域检测后重新扫描整页。文本和HTML按阅读顺序导出，并不完全复现原始页面的布局。漫画模式使用识别指令和通用文档检测模型，复杂气泡或阅读方向可能需要重新识别。

## 个性化设置

在设置中可以选择 **English、한국어、日本語**。默认界面语言为英语，首次启动时使用安装过程中选择的语言。界面语言与文档识别语言相互独立。

您可以选择OCR模型和运行设备，并调整各模型独立的选项与指令。没有独立显卡也能使用CPU运行。CUDA需要兼容的NVIDIA GPU及驱动程序。

<img src="docs/assets/settings.png" alt="包含模型与运行设备选项的识别设置">

您可以在设置中检查更新，也可以启用启动时通知。安装更新时，可选择保留现有设置，或备份设置后重新开始。已下载的模型和文档会保留。卸载时，可以分别选择是否删除设置与模型，以及是否删除应用数据文件夹内的工作区。

安装程序已包含所需运行环境，无需另外安装Rust、Python、Node.js或LM Studio。

## 许可证

本项目的原创代码和文档采用 **[CC BY 4.0](LICENSE)** 许可证。署名信息请参阅 [NOTICE.txt](NOTICE.txt)。第三方模型和库遵循各自的许可证。

[构建与开发指南](docs/development/BUILDING.md)
