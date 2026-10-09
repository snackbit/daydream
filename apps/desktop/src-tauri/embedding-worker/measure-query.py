"""Synthetic persistent-worker query latency, including private pipe/JSON costs.

Arguments: worker, GGUF, model ID, backend. No network or captured content.
"""
import json
import math
import select
import statistics
import subprocess
import sys
import time

started = time.perf_counter()
worker = subprocess.Popen(sys.argv[1:5], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)

def receive():
    if not select.select([worker.stdout], [], [], 120)[0]:
        raise TimeoutError("worker timeout")
    line = worker.stdout.readline(1024*1024)
    assert line.endswith("\n"), "bounded worker reply required"
    return json.loads(line)

try:
    ready = receive()
    assert ready["ready"] is True
    startup_ms = (time.perf_counter()-started)*1000
    queries = ["Where did we discuss travel costs?", "Wann haben wir die Reisekosten besprochen?"]
    samples = []
    for iteration in range(23):
        started = time.perf_counter()
        worker.stdin.write(json.dumps({"purpose": "query", "texts": [queries[iteration % 2]]})+"\n")
        worker.stdin.flush()
        reply = receive()
        vector = reply["vectors"][0]
        assert len(vector) == 384 and all(math.isfinite(value) for value in vector)
        if iteration >= 3:
            samples.append((time.perf_counter()-started)*1000)
    worker.stdin.close()
    assert worker.wait(timeout=10) == 0
    assert worker.stderr.read() == ""
    print(json.dumps({"backend": ready["backend"], "model": sys.argv[3], "startupMs": startup_ms, "warmSamples": len(samples), "p50Ms": statistics.median(samples), "p95Ms": sorted(samples)[18], "maxMs": max(samples)}))
finally:
    if worker.poll() is None:
        worker.kill()
        worker.wait(timeout=10)
