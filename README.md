# Glyph OCR

문서·PDF 스캔본·사진·PC 캡처를 인식하는 Windows 데스크톱 앱입니다. Rust 코어와 Tauri 2 UI를 사용하고, OCR은 PaddleOCR-VL 1.6을 동봉한 llama.cpp에서 실행합니다.

## 사용 방법

1. 파일 선택, 드래그 앤 드롭 또는 **Ctrl+V**로 이미지를 추가합니다. 문서 목록 주변을 클릭한 뒤 붙여넣어도 됩니다.
2. 목록에서 페이지를 선택합니다. 클릭은 한 페이지, Ctrl+클릭은 개별 선택, Shift+클릭은 범위 선택, Ctrl+Shift+클릭은 범위 추가, Ctrl+A는 현재 목록 전체 선택입니다. 검색 입력 안에서는 일반 텍스트 선택을 유지합니다.
3. 인식 모드와 사용자 지침을 정하고 **선택 스캔** 또는 **전체 스캔**을 누릅니다. 완료한 페이지도 몇 번이든 다시 스캔할 수 있습니다.
4. 결과를 검토·편집하고 TXT, Markdown, HTML, JSON으로 내보냅니다.

**영역 탐지 후 세부 OCR** 체크박스가 꺼져 있으면 페이지 전체를 한 번에 인식합니다. 켜져 있으면 PP-DocLayoutV3로 영역과 읽기 순서를 찾고 각 영역을 OCR합니다. 원본 위의 번호를 누르거나 구조 탭에서 영역별 텍스트와 위치를 확인할 수 있습니다. 문서 모드에서는 탐지된 표와 수식을 해당 인식 작업으로 처리합니다.

모드는 문서 구조 유지, 텍스트 그대로, 표, 수식, 만화·말풍선입니다. 만화의 읽는 방향은 사용자 지침에 지정할 수 있습니다. 만화 모드는 프롬프트 설정이며, 말풍선 전용으로 학습된 탐지 모델은 아닙니다.

지원 입력: **PDF, PNG, JPG/JPEG, WEBP, BMP**. Word·PowerPoint 파일은 PDF로 저장한 뒤 추가합니다. 암호화된 PDF는 먼저 암호를 해제해야 합니다. 한 작업 최대 1,000페이지, 이미지 파일 최대 100MB, PDF 최대 1GB입니다.

## 최초 모델 다운로드

설치기와 Git 저장소에 모델 가중치를 넣지 않습니다. 최초 스캔 때 필요한 파일을 공식 Hugging Face 저장소에서 내려받고 SHA-256으로 확인합니다.

| 모델 | 받는 시점 | 용량 |
| --- | --- | --- |
| [PaddleOCR-VL-1.6-GGUF](https://huggingface.co/PaddlePaddle/PaddleOCR-VL-1.6-GGUF) + mmproj | 첫 스캔 | 약 1.82GB |
| [PP-DocLayoutV3 Safetensors](https://huggingface.co/PaddlePaddle/PP-DocLayoutV3_safetensors) | 영역 탐지를 켠 첫 스캔 | 약 133MB |

우측 하단에 진행률과 일시정지·계속 받기·중단 버튼이 표시됩니다. 중단하거나 앱을 종료해도 부분 파일을 보관하며, 다음 스캔에서 이어받습니다. 인터넷은 모델 준비에 필요하고 이후 OCR은 오프라인에서 실행할 수 있습니다. 문서나 인식 결과는 다운로드 서버로 보내지 않습니다.

- 모델: `%LOCALAPPDATA%\com.glyph.localocr\models\<revision>`
- 설정·실행 로그: `%APPDATA%\com.glyph.localocr`
- 기본 작업: 사용자 문서 폴더의 `Glyph\Glyph-<id>`
- 작업 폴더에 원본 사본, 페이지 이미지, `project.json`, 페이지별 결과를 저장합니다.
- 앱 제거 후에도 작업 문서와 모델 캐시는 보존합니다.

## 개발과 설치기 빌드

빌드 환경: Windows x64, Node.js 22 이상, Rust(`rust-toolchain.toml`), Visual Studio의 **C++를 사용한 데스크톱 개발** 및 Windows SDK, Python 3.12. Python은 빌드 시 레이아웃 실행 그래프를 생성하는 용도로만 사용하며 설치된 앱에는 필요하지 않습니다.

```powershell
npm ci
npm run resources:prepare
npm run desktop
```

`resources:prepare`는 고정 버전 llama.cpp CPU/Vulkan, PDFium, ONNX Runtime, MSVC DLL과 라이선스를 준비합니다. 레이아웃 그래프 생성 시 공식 Safetensors를 빌드 캐시에 받지만, 설치기에는 가중치를 참조하는 작은 그래프만 포함합니다. 모델 바이트는 사용자 앱이 공식 저장소에서 받습니다.

**설치기 EXE 빌드:**

```powershell
npm run installer
```

출력:

```text
src-tauri/target/release/bundle/inno/Glyph-OCR_2.0.0_x64-setup.exe
```

동일 폴더에 SHA-256 파일과 `build-info.json`을 생성합니다. 설치기는 현재 사용자 범위에 설치하고 WebView2가 없으면 포함된 오프라인 설치 프로그램으로 준비합니다. 최종 사용자에게 Node.js, Python, Rust, LM Studio, Ollama를 요구하지 않습니다. 기본 산출물에는 코드 서명이 적용되지 않습니다.

검증 명령:

```powershell
npm run check
npm test
npm run resources:check
```

## GitHub 태그 릴리즈

`.github/workflows/release.yml`이 `v1.0.0` 같은 태그 푸시를 감지합니다. 태그의 버전을 앱·Rust·설치기에 반영하고, 검증과 EXE 빌드를 거친 뒤 GitHub Release에 설치기·체크섬·빌드 정보를 첨부합니다. 추가 토큰 없이 저장소의 `GITHUB_TOKEN`을 사용합니다.

GitHub 원격 저장소를 연결한 뒤:

```powershell
git push -u origin HEAD
git tag v1.0.0
git push origin v1.0.0
```

태그는 워크플로 파일이 포함된 커밋에 생성해야 합니다. 기존 릴리즈를 재실행하면 같은 파일명의 자산을 갱신합니다. 모델, DLL, 빌드 캐시, 사용자 문서는 Git에서 제외합니다.

## 결과 형식과 제한

JSON schema v2는 전체 페이지 원문·편집본·문서 블록·표 셀 병합 정보·적용 지침과 설정을 포함합니다. 영역 OCR을 사용한 페이지에는 `regions`에 `bbox=[left,top,right,bottom]`, 실제 탐지 신뢰도, 읽기 순서, 영역별 원문과 OCR 결과를 기록합니다.

좌표는 화면에 표시된 정규화 페이지 이미지의 픽셀 기준입니다. PDF는 페이지별로 렌더링하고 사진은 회전 정보를 반영하여 최대 변 길이를 제한합니다. 원본 파일의 바이트와 크기는 별도 사본에 보존합니다. 문서 전체 편집 후에도 영역의 최초 OCR 결과는 유지하므로 JSON의 `regionTextMatchesDocument`를 확인할 수 있습니다.

TXT/Markdown/HTML은 읽기 순서에 따른 문서 출력이며 위치를 고정한 원본 복제 형식은 아닙니다. 영역 위치는 원본 미리보기와 JSON에서 제공하고 검색 가능한 PDF는 생성하지 않습니다. 작은 글씨, 세로쓰기, 만화의 컷과 말풍선, 복잡한 표는 누락되거나 읽기 순서가 틀릴 수 있으므로 모드·지침·영역 탐지 설정을 바꿔 재시도할 수 있습니다.

설계: [docs/REDESIGN.md](docs/REDESIGN.md) · 검증 기록: [docs/VALIDATION.md](docs/VALIDATION.md)
