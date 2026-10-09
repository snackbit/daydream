"""Extract the official pinned tokenizer normalization table for the native worker.

This is a build-time provenance generator, not an app runtime dependency.
Usage: python extract-normalizer.py official-tokenizer.json e5-normalizer.bin
"""
import base64
import json
import pathlib
import sys

data = json.loads(pathlib.Path(sys.argv[1]).read_text())
normalizers = data["normalizer"]["normalizers"]
assert normalizers[0]["type"] == "Precompiled"
assert normalizers[1] == {"type": "Replace", "pattern": {"Regex": " {2,}"}, "content": " "}
assert data["pre_tokenizer"] == {"type": "Metaspace", "replacement": "▁", "add_prefix_space": True}
pathlib.Path(sys.argv[2]).write_bytes(base64.b64decode(normalizers[0]["precompiled_charsmap"], validate=True))
