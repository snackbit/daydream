"""Offline synthetic passage retrieval proof against the packaged CPU worker.

Usage: python verify-retrieval.py worker model.gguf
The relevant fragment is a small paragraph among unrelated long-source material.
Canonical fragmentation/provenance/privacy are separately tested in Rust.
"""
import json
import math
import subprocess
import sys

english_noise = "The garden notes describe watering the tomatoes, trimming the hedge, cleaning the bicycle and watching birds near the pond. "
german_noise = "Die Gartennotizen beschreiben das Gießen der Tomaten, das Schneiden der Hecke, das Putzen des Fahrrads und Vögel am Teich. "
english_relevant = "Helena asked me to forward the cost worksheet by Wednesday evening so she could finalize the reimbursements. "
german_relevant = "Helena bat mich, die Kostenübersicht bis Mittwochabend zu schicken, damit sie die Erstattungen abschließen kann. "
passages = [english_noise * 3] * 3 + [english_noise * 2 + english_relevant] + [english_noise * 3] * 3
passages += [german_noise * 3] * 3 + [german_noise * 2 + german_relevant] + [german_noise * 3] * 3
assert all(len(passage.encode("utf-8")) <= 480 for passage in passages)
queries = [
    "When did someone ask me to send the expense spreadsheet before Thursday?",
    "Wann sollte ich die Tabelle mit den Ausgaben weiterleiten?",
]
requests = [{"purpose": "query", "texts": queries}, {"purpose": "document", "texts": passages}]
result = subprocess.run([sys.argv[1], sys.argv[2], "multilingual-e5-small-q8_0", "cpu"], input="".join(json.dumps(request) + "\n" for request in requests), text=True, capture_output=True, timeout=120, check=True)
assert result.stderr == "", "Production worker must not emit diagnostics"
responses = [json.loads(line) for line in result.stdout.splitlines()]
assert responses[0] == {"backend": "cpu", "dimensions": 384, "ready": True}
vectors = responses[2]["vectors"]
assert all(len(vector) == 384 and all(math.isfinite(value) for value in vector) for vector in vectors)
scores = [[sum(a*b for a, b in zip(query, passage)) for passage in vectors] for query in responses[1]["vectors"]]
unrelated = [i for i in range(len(passages)) if i not in (3, 10)]
assert scores[0][3] > max(scores[0][i] for i in unrelated), scores
assert scores[1][10] > max(scores[1][i] for i in unrelated), scores
print(json.dumps({"retrievalCosines": scores, "passageTokenCounts": responses[2]["tokenCounts"], "dimensions": 384, "stderrBytes": 0}))
