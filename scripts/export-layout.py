"""Build-time export only. End users run Rust + ONNX Runtime; no Python needed.
The graph contains no learned weights. Its external tensor offsets refer directly
to the unchanged official model.safetensors downloaded by the app.
"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import sys

ROOT = Path(__file__).resolve().parent.parent
CACHE = ROOT / ".cache" / "layout-export"
CACHE.mkdir(parents=True, exist_ok=True)
os.environ["HF_HOME"] = str(ROOT / ".cache" / "hf")
os.environ["HF_HUB_DISABLE_PROGRESS_BARS"] = "1"
import numpy as np
import onnx
import onnxruntime as ort
import torch
from huggingface_hub import snapshot_download
from transformers import AutoModelForObjectDetection

REPO = "PaddlePaddle/PP-DocLayoutV3_safetensors"
REVISION = "97d101e6db2642e162a1d05392d1b0231c91033e"
WEIGHT_HASH = "5ea422c6cc5fe759a47e1357c35639b58173508e025a3131cbe4b6ac59e2b85e"
snapshot = Path(snapshot_download(REPO, revision=REVISION, local_dir=CACHE/"official", allow_patterns=["config.json", "preprocessor_config.json", "model.safetensors"]))
weight_file = snapshot / "model.safetensors"
weights = weight_file.read_bytes()
assert hashlib.sha256(weights).hexdigest() == WEIGHT_HASH
header_length = struct.unpack("<Q", weights[:8])[0]
header = json.loads(weights[8:8+header_length])
tensor_by_hash = {}
for name, info in header.items():
    if name == "__metadata__":
        continue
    start, end = info["data_offsets"]
    raw = weights[8+header_length+start:8+header_length+end]
    tensor_by_hash[(hashlib.sha256(raw).digest(), len(raw))] = (name,8+header_length+start,end-start)

torch.set_num_threads(4)
torch.manual_seed(7)
model = AutoModelForObjectDetection.from_pretrained(snapshot, local_files_only=True).eval()
class LayoutGraph(torch.nn.Module):
    def __init__(self, detector):
        super().__init__()
        self.detector = detector
    def forward(self, pixel_values):
        result = self.detector(pixel_values=pixel_values)
        return result.logits, result.pred_boxes, result.order_logits
wrapper = LayoutGraph(model).eval()
sample = torch.rand(1,3,800,800)
with torch.inference_mode():
    expected = tuple(t.numpy() for t in wrapper(sample))
    torch.onnx.export(wrapper, sample, str(CACHE/"full.onnx"), opset_version=18,
        input_names=["pixel_values"], output_names=["logits","pred_boxes","order_logits"],
        dynamo=False, do_constant_folding=False)
graph = onnx.load(CACHE/"full.onnx")
linked = []
unmatched = []
for tensor in graph.graph.initializer:
    raw = onnx.numpy_helper.to_array(tensor).tobytes()
    entry = tensor_by_hash.get((hashlib.sha256(raw).digest(),len(raw)))
    if entry:
        name, offset, length = entry
        onnx.external_data_helper.set_external_data(tensor, location="model.safetensors", offset=offset, length=length)
        tensor.ClearField("raw_data")
        linked.append({"initializer":tensor.name,"weight":name,"offset":offset,"bytes":length})
    else:
        unmatched.append({"name":tensor.name,"bytes":len(raw)})
# All model parameters must stay external. Only small exporter-generated constants may remain.
if sum(t["bytes"] for t in unmatched)>65536:
    raise RuntimeError("Unexpected embedded tensor weights: "+str(unmatched))
for node in graph.graph.node:
    node.doc_string=""
graph.doc_string=""
onnx.save_model(graph, CACHE/"layout.onnx")
shutil.copyfile(weight_file,CACHE/"model.safetensors")
session=ort.InferenceSession(str(CACHE/"layout.onnx"),providers=["CPUExecutionProvider"])
actual=session.run(None,{"pixel_values":sample.numpy()})
for name,want,got in zip(["logits","pred_boxes","order_logits"],expected,actual):
    difference=float(np.max(np.abs(want-got)))
    print(name,got.shape,"max absolute difference",difference,flush=True)
    np.testing.assert_allclose(want,got,rtol=0.01,atol=0.015)
destination=ROOT/"src-tauri"/"resources"/"layout"
destination.mkdir(parents=True,exist_ok=True)
shutil.copyfile(CACHE/"layout.onnx",destination/"layout.onnx")
config=json.loads((snapshot/"config.json").read_text())
(destination/"labels.json").write_text(json.dumps(config["id2label"],indent=2)+"\n",encoding="utf-8")
report={"repository":REPO,"revision":REVISION,"weightsSha256":WEIGHT_HASH,"linkedTensors":len(linked),"embeddedConstants":unmatched,"graphSha256":hashlib.sha256((destination/"layout.onnx").read_bytes()).hexdigest(),"input":[1,3,800,800],"transformers":"5.6.2","torch":"2.11.0","onnx":"1.21.0"}
(destination/"export-info.json").write_text(json.dumps(report,indent=2)+"\n",encoding="utf-8")
print("Exported weight-free layout graph:",destination,flush=True)
