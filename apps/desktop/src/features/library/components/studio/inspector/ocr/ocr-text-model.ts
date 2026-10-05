import type { OcrDocument, OcrHistoryEntry } from '@daydream/shared';

export interface OcrTextPiece {
  text: string;
  match: boolean;
  uncertain: boolean;
}
export function normalizedOcrText(text: string): string {
  return text.replace(/\s+/gu, ' ').trim();
}

function uncertainWord(
  document: OcrDocument,
  confidence: number | null,
): boolean {
  return (
    document.engine !== 'vision' && confidence !== null && confidence < 0.5
  );
}

export function uncertainOcrWordCount(document: OcrDocument): number {
  return document.blocks.reduce(
    (count, block) =>
      count +
      block.words.filter((word) => uncertainWord(document, word.confidence))
        .length,
    0,
  );
}

function textMatches(
  text: string,
  query: string,
): Array<{ start: number; end: number }> {
  const needle = query.trim();
  if (!needle) return [];
  const escaped = needle.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  return [...text.matchAll(new RegExp(escaped, 'giu'))].map((match) => ({
    start: match.index,
    end: match.index + match[0].length,
  }));
}

export function ocrTextPieces(
  document: OcrDocument,
  query: string,
): OcrTextPiece[] {
  const text = document.text;
  const uncertainty: Array<{ start: number; end: number }> = [];
  let cursor = 0;
  for (const block of [...document.blocks].sort(
    (left, right) => left.order - right.order,
  )) {
    for (const word of block.words) {
      const start = text.indexOf(word.text, cursor);
      if (start < 0 || word.text.length === 0) continue;
      cursor = start + word.text.length;
      if (uncertainWord(document, word.confidence))
        uncertainty.push({ start, end: cursor });
    }
  }
  const matches = textMatches(text, query);
  const boundaries = new Set([0, text.length]);
  for (const range of [...matches, ...uncertainty]) {
    boundaries.add(range.start);
    boundaries.add(range.end);
  }
  const positions = [...boundaries].sort((left, right) => left - right);
  return positions.slice(0, -1).map((start, index) => ({
    text: text.slice(start, positions[index + 1]),
    match: matches.some((range) => start >= range.start && start < range.end),
    uncertain: uncertainty.some(
      (range) => start >= range.start && start < range.end,
    ),
  }));
}

export function ocrMatchCount(text: string, query: string): number {
  return textMatches(text, query).length;
}

/** An exact prefix/suffix comparison keeps digits, names, case and punctuation.
 * Only whitespace is normalized; this is a literal revision diff, not a summary. */
export function ocrRevisionDiff(
  previous: OcrHistoryEntry,
  current: OcrHistoryEntry,
) {
  const before = normalizedOcrText(previous.text);
  const after = normalizedOcrText(current.text);
  let prefix = 0;
  while (
    prefix < Math.min(before.length, after.length) &&
    before[prefix] === after[prefix]
  )
    prefix += 1;
  let suffix = 0;
  while (
    suffix < Math.min(before.length - prefix, after.length - prefix) &&
    before[before.length - suffix - 1] === after[after.length - suffix - 1]
  )
    suffix += 1;
  return {
    added: after.slice(prefix, after.length - suffix),
    removed: before.slice(prefix, before.length - suffix),
  };
}
