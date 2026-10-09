"""Offline synthetic CPU/Vulkan parity proof, on a real supported GPU.

Arguments: CPU executable, optional Vulkan executable, managed GGUF, model ID.
No private recordings, automatic capture or network are involved.
"""
import json
import math
import os
import subprocess
import sys

requests = [
    {"purpose": "query", "texts": ["Where did we discuss travel costs?", "Wann haben wir die Reisekosten besprochen?"]},
    {"purpose": "document", "texts": ["We reviewed the travel budget and hotel prices.", "Wir besprachen das Reisebudget und die Hotelpreise.", "I watered plants in the garden."]},
]
payload = "".join(json.dumps(request) + "\n" for request in requests)

def run(worker, backend, expected_backend=None, environment=None):
    result = subprocess.run([worker, sys.argv[3], sys.argv[4], backend], input=payload, capture_output=True, text=True, timeout=120, check=True, env=environment)
    rows = [json.loads(line) for line in result.stdout.splitlines()]
    assert rows[0]["ready"] is True
    assert rows[0]["dimensions"] == 384
    assert rows[0]["backend"] == (expected_backend or ("cpu" if backend == "cpu" else "vulkan")), rows[0]
    assert result.stderr == "", "Production worker must not emit diagnostics"
    vectors = rows[1]["vectors"] + rows[2]["vectors"]
    assert all(len(vector) == 384 and all(math.isfinite(value) for value in vector) for vector in vectors)
    return vectors

cpu = run(sys.argv[1], "cpu")
gpu = run(sys.argv[2], "vulkan")
automatic = run(sys.argv[2], "auto")
gpu_cosines = [sum(a*b for a, b in zip(left, right)) for left, right in zip(cpu, gpu)]
auto_cosines = [sum(a*b for a, b in zip(left, right)) for left, right in zip(gpu, automatic)]
assert min(gpu_cosines) >= 0.995, gpu_cosines
assert min(auto_cosines) >= 0.9999, auto_cosines
hidden_devices = dict(os.environ, GGML_VK_VISIBLE_DEVICES="")
fallback = run(sys.argv[2], "auto", "cpu", hidden_devices)
fallback_cosines = [sum(a*b for a, b in zip(left, right)) for left, right in zip(cpu, fallback)]
assert min(fallback_cosines) >= 0.9999, fallback_cosines
unavailable = subprocess.run([sys.argv[2], sys.argv[3], sys.argv[4], "vulkan"], input="", capture_output=True, text=True, timeout=120, env=hidden_devices)
assert unavailable.returncode == 3 and json.loads(unavailable.stdout) == {"error": "vulkan_unavailable"}
assert unavailable.stderr == ""
print(json.dumps({"model": sys.argv[4], "cpuVulkanCosines": gpu_cosines, "vulkanAutoCosines": auto_cosines, "autoCpuFallbackCosines": fallback_cosines, "explicitVulkanUnavailableExit": unavailable.returncode, "dimensions": 384, "stderrBytes": 0}))
