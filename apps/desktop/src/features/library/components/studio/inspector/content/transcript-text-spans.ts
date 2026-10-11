import type { RecordingTranscriptWord } from '@daydream/shared';

export interface TranscriptTextSpan {
  text: string;
  word: RecordingTranscriptWord | null;
}

/** Timings annotate canonical text; token boundaries never introduce whitespace. */
export function transcriptTextSpans(
  text: string,
  words: readonly RecordingTranscriptWord[],
): TranscriptTextSpan[] {
  const spans: TranscriptTextSpan[] = [];
  let cursor = 0;
  for (const word of words) {
    if (word.endOffsetMs <= word.startOffsetMs || !word.text.trim()) continue;
    const token = word.text.trim();
    const start = text.indexOf(token, cursor);
    if (start < 0) return [{ text, word: null }];
    if (start > cursor)
      spans.push({ text: text.slice(cursor, start), word: null });
    cursor = start + token.length;
    spans.push({ text: text.slice(start, cursor), word });
  }
  if (cursor < text.length)
    spans.push({ text: text.slice(cursor), word: null });
  return spans;
}
