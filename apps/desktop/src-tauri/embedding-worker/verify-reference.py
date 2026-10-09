"""Development-only reference proof; never packaged or invoked by the app.

Requires pinned CPU torch/transformers tools described in docs/embedding-runtime.md.
Arguments: official pinned HF directory, worker executable, candidate GGUF,
optional model ID (default multilingual-e5-small-q8_0).
All inputs are synthetic. No network or private recordings are used.
"""
import json
import subprocess
import sys
import torch
from transformers import AutoModel, AutoTokenizer

torch.set_num_threads(2)
model_id = sys.argv[4] if len(sys.argv) > 4 else "multilingual-e5-small-q8_0"
is_e5 = model_id == "multilingual-e5-small-q8_0"
texts = [
    "When did I discuss the budget for our trip?",
    "Wann habe ich die Kosten der Reise besprochen?",
    "We reviewed the travel budget and hotel prices in the afternoon.",
    "Am Nachmittag besprachen wir das Reisebudget und die Hotelpreise.",
    "I watered the garden plants and cleaned my bicycle.",
    "Über die Größe der Straße sprechen: Café, München, Fußgänger.",
    "Unicode normalization: Ｂｕｄｇｅｔ Café Café ﬁve.",
    "Literal special tokens <mask> <s> </s> <unk> [MASK] [CLS] [SEP].",
    "Unknown characters and emoji: \u0378 🧪🚀 中文测试 العربية.",
]
native_texts = texts.copy()
texts = [("query: " if i < 2 else "passage: ") + text if is_e5 else
         ("Represent this sentence for searching relevant passages: " + text if i < 2 else text)
         for i, text in enumerate(texts)]
tokenizer = AutoTokenizer.from_pretrained(sys.argv[1], local_files_only=True)
model = AutoModel.from_pretrained(sys.argv[1], local_files_only=True).eval()
batch = tokenizer(texts, max_length=512, padding=True, truncation=False, return_tensors="pt")
with torch.no_grad():
    output = model(**batch).last_hidden_state
    mask = batch["attention_mask"].unsqueeze(-1).bool()
    pooled = output.masked_fill(~mask, 0.0).sum(dim=1) / mask.sum(dim=1) if is_e5 else output[:, 0]
    reference = torch.nn.functional.normalize(pooled, p=2, dim=1)
requests = [
    {"purpose": "query", "texts": native_texts[:2]},
    {"purpose": "document", "texts": native_texts[2:]},
]
result = subprocess.run([sys.argv[2], sys.argv[3], model_id, "cpu"], input="".join(json.dumps(request) + "\n" for request in requests), text=True, capture_output=True, timeout=120, check=True)
responses = [json.loads(line) for line in result.stdout.splitlines()]
assert responses[0]["ready"] is True
vectors = torch.tensor(responses[1]["vectors"] + responses[2]["vectors"])
counts = responses[1]["tokenCounts"] + responses[2]["tokenCounts"]
expected_counts = batch["attention_mask"].sum(dim=1).tolist()
assert counts == expected_counts, (counts, expected_counts)
actual_ids = responses[1]["tokenIds"] + responses[2]["tokenIds"]
expected_ids = [row[mask.bool()].tolist() for row, mask in zip(batch["input_ids"], batch["attention_mask"])]
assert actual_ids == expected_ids, (actual_ids, expected_ids)
similarities = (vectors * reference).sum(dim=1)
assert float(similarities.min()) >= 0.995, similarities.tolist()
ranking = vectors[:2] @ vectors[2:5].T
if is_e5:
    assert bool((ranking[:, :2].min(dim=1).values > ranking[:, 2]).all()), ranking.tolist()
else:
    assert ranking[0, 0] > ranking[0, 2], ranking.tolist()
print(json.dumps({"tokenCounts": counts, "referenceCosines": similarities.tolist(), "retrievalCosines": ranking.tolist(), "dimensions": vectors.shape[1]}))
