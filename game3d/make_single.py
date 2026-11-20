#!/usr/bin/env python3
"""Build one self-contained HTML file with everything embedded as data URIs."""
import base64
import json
import mimetypes
from pathlib import Path

ROOT = Path(__file__).resolve().parent
OUT = ROOT / "dist" / "ForestBloom3D.html"

SKIP_NAMES = {"index.html", "main.js", "make_single.py", "build.sh", "serve.py", "README.md"}
SKIP_DIRS = {"dist", "target", ".git"}

MIME = {
    ".gltf": "model/gltf+json",
    ".glb": "model/gltf-binary",
    ".bin": "application/octet-stream",
    ".png": "image/png",
    ".jpg": "image/jpeg",
    ".ogg": "audio/ogg",
    ".wav": "audio/wav",
    ".json": "application/json",
    ".wasm": "application/wasm",
}


def collect_files():
    files = {}
    for path in sorted(ROOT.rglob("*")):
        if not path.is_file():
            continue
        rel = path.relative_to(ROOT)
        if any(part in SKIP_DIRS for part in rel.parts):
            continue
        if rel.name in SKIP_NAMES or rel.name.startswith("."):
            continue
        mime = MIME.get(path.suffix.lower()) or mimetypes.guess_type(path.name)[0] or "application/octet-stream"
        data = base64.b64encode(path.read_bytes()).decode("ascii")
        files[rel.as_posix()] = f"data:{mime};base64,{data}"
    return files


def inline_script(src: str) -> str:
    code = (ROOT / src).read_text(encoding="utf-8")
    code = code.replace("</script>", "<\\/script>")
    return f"<script>\n{code}\n</script>"


def main():
    html = (ROOT / "index.html").read_text(encoding="utf-8")
    files = collect_files()
    wasm_b64 = base64.b64encode((ROOT / "core.wasm").read_bytes()).decode("ascii")

    bootstrap = (
        "<script>\n"
        "window.__EMBEDDED_FILES__ = " + json.dumps(files, ensure_ascii=False, separators=(",", ":")) + ";\n"
        "window.__WASM_B64__ = \"" + wasm_b64 + "\";\n"
        "</script>\n"
    )

    for vendor in ["vendor/three.min.js", "vendor/GLTFLoader.js", "vendor/PointerLockControls.js"]:
        html = html.replace(f'<script src="{vendor}"></script>', inline_script(vendor))

    html = html.replace('<script src="main.js"></script>', bootstrap + inline_script("main.js"))

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(html, encoding="utf-8")
    size = OUT.stat().st_size
    print(f"built {OUT.relative_to(ROOT)} ({size / 1024 / 1024:.1f} MB, {len(files)} embedded files)")


if __name__ == "__main__":
    main()
