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

**FastFileOCR**는 스캔 문서, 사진, PC 캡처 이미지에서 글자를 인식해 편집 가능한 텍스트로 바꾸는 Windows 앱입니다. OCR은 사용자의 PC에서 실행됩니다.

|                | 지원 내용                                                                                                                                          |
| -------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| OCR 모델       | **[PaddleOCR-VL 1.6](https://huggingface.co/PaddlePaddle/PaddleOCR-VL-1.6-GGUF)**, 영역 탐지에 **PP-DocLayoutV3** 사용                             |
| 코어 개발 언어 | **Rust**                                                                                                                                           |
| 문서 인식 언어 | 한국어, 영어, 일본어, 중국어 등 — **[모델 기준 109개 언어](https://www.paddleocr.ai/main/en/version3.x/algorithm/PaddleOCR-VL/PaddleOCR-VL.html)** |
| 앱 메뉴 언어   | **English, 한국어, 日本語**                                                                                                                        |
| 입력 파일      | **PDF, PNG, JPG/JPEG, WEBP, BMP**, 클립보드 이미지                                                                                                 |
| 내보내기 형식  | **TXT, Markdown, HTML, JSON**                                                                                                                      |
| 지원 환경      | **Windows 10/11 x64**                                                                                                                              |
| 실행 장치      | **자동, CPU, Vulkan, NVIDIA CUDA**                                                                                                                 |

Word와 PowerPoint 문서는 PDF로 내보낸 뒤 추가해 주세요. 암호화된 PDF는 먼저 잠금을 해제해야 합니다.

<p align="center">
  <img src="docs/assets/workspace.png" alt="FastFileOCR에서 샘플 청구서를 인식하는 화면">
</p>

## 주요 기능

- 파일 선택, 드래그 앤 드롭, **Ctrl+V**로 PDF와 이미지를 추가합니다.
- 영역을 탐지한 뒤 OCR을 실행해 위치와 읽기 순서를 보존합니다. **기본으로 체크되어 있으며**, 체크를 해제하면 전체 페이지 OCR로 전환됩니다.
- 문서, 일반 텍스트, 표, 수식, 만화 인식 모드를 제공합니다.
- 지침을 직접 입력하고, 같은 페이지를 원하는 만큼 다시 스캔할 수 있습니다.
- Ctrl, Shift, Ctrl+A로 여러 페이지를 선택합니다. 선택한 페이지를 다시 누르면 선택이 해제됩니다.
- 결과를 검토하고 편집한 뒤 텍스트나 구조화된 데이터로 저장합니다.

## 시작하기

1. [릴리즈](../../releases/latest)에서 Windows x64 설치기를 내려받습니다.
2. 설치 언어와 설정·모델·작업 공간을 저장할 폴더를 선택합니다.
3. 앱을 열고 파일을 추가한 뒤 인식 모드를 선택하고 스캔합니다.

함께 제공되는 [샘플 문서](docs/assets/sample-invoice.pdf)로 먼저 사용해 볼 수 있습니다.

최초 스캔 시 Hugging Face에서 약 **1.82 GB**의 OCR 모델과 약 **133 MB**의 영역 탐지 모델을 내려받습니다. 영역 탐지를 끄면 OCR 모델만 필요합니다. 다운로드는 일시정지하거나 중단할 수 있고, 앱을 다시 실행한 뒤에도 이어받을 수 있습니다. 모델 준비가 끝나면 오프라인으로 OCR을 실행합니다. 문서와 인식 결과는 업로드하지 않습니다.

## 영역과 문서 구조

영역 탐지는 기본으로 켜져 있습니다. 원본 페이지 위에서 텍스트 영역, 표, 제목의 위치를 확인할 수 있습니다. JSON으로 내보내면 영역 좌표, 읽기 순서, 영역별 인식 결과가 함께 저장됩니다.

<img src="docs/assets/regions.png" alt="영역 탐지와 구조화된 OCR 결과">

장식적인 제목이나 특이한 배치는 영역 탐지에서 누락될 수 있습니다. 글자가 빠졌다면 영역 탐지를 해제하고 전체 페이지를 다시 스캔해 보세요. 텍스트와 HTML은 읽기 순서에 따라 내보내며, 원본 페이지의 배치를 그대로 재현하지는 않습니다. 만화 모드는 인식 지침과 범용 문서 탐지기를 사용하므로 복잡한 말풍선이나 읽기 방향은 재인식이 필요할 수 있습니다.

## 나에게 맞게 설정하기

설정에서 **English, 한국어, 日本語**를 선택할 수 있습니다. 기본 언어는 영어이며, 처음 실행할 때는 설치 과정에서 고른 언어를 사용합니다. 앱 메뉴 언어와 문서의 인식 언어는 별개입니다.

OCR 모델과 실행 장치를 선택하고, 모델별 옵션과 지침을 조절할 수 있습니다. 별도 그래픽 카드 없이도 CPU로 실행할 수 있으며, CUDA에는 호환되는 NVIDIA GPU와 드라이버가 필요합니다.

<img src="docs/assets/settings.png" alt="모델 선택과 실행 장치 옵션이 있는 인식 설정">

설정에서 업데이트를 확인하고 시작 시 알림을 켤 수 있습니다. 업데이트 설치 시 기존 설정을 유지하거나, 설정을 백업하고 새로 시작할 수 있습니다. 내려받은 모델과 문서는 유지됩니다. 앱 삭제 시에는 설정·모델을 삭제할지, 앱 데이터 폴더 안의 작업 공간도 삭제할지를 각각 선택합니다.

CPU 엔진과 문서 처리 라이브러리는 설치기에 포함됩니다. CUDA·Vulkan 엔진은 처음 사용할 때 자동으로 내려받습니다. **CUDA 약 537 MB, Vulkan 약 34 MB**이며, 일시정지·이어받기와 앱 업데이트 후 재사용을 지원합니다. 자동 모드는 호환되는 GPU 엔진 하나를 선택하고, 필요하면 CPU로 전환합니다.

Rust, Python, Node.js, LM Studio, CUDA Toolkit을 별도로 설치할 필요가 없습니다. GPU 사용에는 호환되는 그래픽 드라이버가 필요합니다.

## 기여하기

AI를 활용한 기여도 환영합니다. 새 기능이나 구조 변경은 먼저 Issue에서 필요성과 목적을 논의해 주세요. 자세한 PR 지침은 [CONTRIBUTING.md](CONTRIBUTING.md)를 참고하세요.

## 라이선스

이 프로젝트의 자체 코드와 문서는 **[CC BY 4.0](LICENSE)**으로 제공됩니다. 저작자 표시는 [NOTICE.txt](NOTICE.txt)를 참고하세요. 외부 모델과 라이브러리는 각각의 라이선스를 따릅니다.

[빌드 및 개발 가이드](docs/development/BUILDING.md)
