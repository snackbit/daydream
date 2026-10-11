import type { OcrHistoryEntry } from '@daydream/shared';
import { Button } from '@/components/ui/button';
import { formatTimelineOffset } from '../inspector-format';
import { ocrRevisionDiff } from './ocr-text-model';

export interface OcrHistoryPanelProps {
  error: string | null;
  hasMore: boolean;
  items: OcrHistoryEntry[];
  loading: boolean;
  onLoadMore: () => void;
  onRetry: () => void;
  onSelect: (entry: OcrHistoryEntry) => void;
  selectedId: string | null;
}
export function OcrHistoryPanel({
  error,
  hasMore,
  items,
  loading,
  onLoadMore,
  onRetry,
  onSelect,
  selectedId,
}: OcrHistoryPanelProps) {
  return (
    <div className="space-y-2">
      <p className="text-muted-foreground text-xs">
        Identical adjacent text is grouped. Ranges show supporting samples, not
        continuous visibility.
      </p>
      {error ? (
        <div role="alert" className="text-destructive text-sm">
          {error}
          <Button onClick={onRetry} size="sm" variant="outline">
            Retry history
          </Button>
        </div>
      ) : null}
      {items.length === 0 && !loading && !error ? (
        <p className="text-muted-foreground text-sm">
          No OCR history for this source.
        </p>
      ) : null}
      <div className="divide-y">
        {items.map((entry, index) => (
          <OcrHistoryRevision
            entry={entry}
            key={entry.id}
            onSelect={onSelect}
            previous={items[index + 1] ?? null}
            selected={selectedId === entry.id}
          />
        ))}
      </div>
      {loading ? (
        <p className="text-muted-foreground text-xs" role="status">
          Loading OCR history…
        </p>
      ) : null}
      {hasMore ? (
        <Button
          disabled={loading}
          onClick={onLoadMore}
          size="sm"
          variant="outline"
        >
          Load older revisions
        </Button>
      ) : null}
    </div>
  );
}

function OcrHistoryRevision({
  entry,
  onSelect,
  previous,
  selected,
}: {
  entry: OcrHistoryEntry;
  onSelect: (entry: OcrHistoryEntry) => void;
  previous: OcrHistoryEntry | null;
  selected: boolean;
}) {
  const comparable =
    previous !== null &&
    previous.status === 'succeeded' &&
    entry.status === 'succeeded' &&
    previous.configFingerprint === entry.configFingerprint &&
    previous.engine === entry.engine &&
    previous.sourceTrackId === entry.sourceTrackId;
  const diff = comparable ? ocrRevisionDiff(previous, entry) : null;
  function select() {
    onSelect(entry);
  }
  const start = formatTimelineOffset(entry.observedStartMs);
  const end = formatTimelineOffset(entry.observedEndMs);
  return (
    <div className="space-y-1 py-2 first:pt-0">
      <Button
        aria-pressed={selected}
        className="h-auto w-full flex-col items-start gap-1 py-1.5 text-left whitespace-normal"
        disabled={
          entry.status !== 'succeeded' || entry.supportTimestampsMs.length === 0
        }
        onClick={select}
        size="sm"
        variant={selected ? 'secondary' : 'ghost'}
      >
        <span className="font-medium">
          {start === end ? start : `${start} – ${end}`}
        </span>
        <span className="text-muted-foreground text-xs">
          {entry.occurrenceCount} supporting{' '}
          {entry.occurrenceCount === 1 ? 'sample' : 'samples'} ·{' '}
          {entry.engine === 'vision'
            ? 'Vision OCR'
            : entry.engine === 'legacy'
              ? 'Legacy OCR'
              : 'Tesseract'}
        </span>
        <span className="line-clamp-2 text-sm">
          {entry.status === 'failed'
            ? 'OCR failed for this sample.'
            : entry.status === 'gap'
              ? 'OCR unavailable for this sample.'
              : entry.text || 'No text detected.'}
        </span>
      </Button>
      {entry.errorCode ? (
        <p className="text-muted-foreground text-xs">{entry.errorCode}</p>
      ) : null}
      {entry.provenance === 'reconstructed' ? (
        <p className="text-muted-foreground text-xs">
          Reconstructed older text
        </p>
      ) : null}
      {diff && (diff.added || diff.removed) ? (
        <div className="space-y-1 px-1 text-xs">
          <div>
            Removed:{' '}
            <del className="break-words text-red-700 dark:text-red-300">
              {diff.removed || 'None'}
            </del>
          </div>
          <div>
            Added:{' '}
            <ins className="break-words text-emerald-700 dark:text-emerald-300">
              {diff.added || 'None'}
            </ins>
          </div>
        </div>
      ) : previous && !comparable ? (
        <p className="text-muted-foreground text-xs">
          Comparison starts after a source, engine, settings or processing
          boundary.
        </p>
      ) : !previous ? (
        <p className="text-muted-foreground text-xs">
          Earlier revision not loaded.
        </p>
      ) : null}
    </div>
  );
}
