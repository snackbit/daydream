import { useCallback, useState } from 'react';
import type { OcrHistoryEntry } from '@daydream/shared';
import { Button } from '@/components/ui/button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useOcrDocument } from '@/features/ai/ocr/use-ocr-document';
import { useOcrHistory } from '@/features/ai/ocr/use-ocr-history';
import { formatTimelineOffset } from '../inspector-format';
import { InspectorSection } from '../inspector-section';
import { OcrDetailsPanel } from './ocr-details-panel';
import { OcrDocumentState } from './ocr-document-state';
import { OcrHistoryPanel } from './ocr-history-panel';

export interface OcrInspectorContext {
  debugEnabled: boolean;
  privacyRevision: string;
  processingRevision?: string;
  recordingId: string;
  sourceTrackId: string;
}
export interface OcrDocumentInspectorProps {
  context: OcrInspectorContext;
  timestampMs: number;
  onSeek: (timestampMs: number) => void;
  documentId?: string | null;
}

export function OcrDocumentInspector({
  context,
  timestampMs,
  onSeek,
  documentId = null,
}: OcrDocumentInspectorProps) {
  const [tab, setTab] = useState('text');
  const [revision, setRevision] = useState<OcrHistoryEntry | null>(null);
  const selectedTimestamp = revision?.supportTimestampsMs[0] ?? timestampMs;
  const selected = useOcrDocument(
    {
      recordingId: context.recordingId,
      sourceTrackId: context.sourceTrackId,
      timestampMs: selectedTimestamp,
      documentId: revision?.id ?? documentId,
    },
    context.privacyRevision,
    context.processingRevision,
  );
  const history = useOcrHistory({ ...context, enabled: tab === 'changes' });
  const document = selected.document;
  const reloadDocument = selected.reload;
  const loadOlderRevisions = history.loadMore;
  const reloadHistory = history.retry;
  const selectRevision = useCallback(
    (entry: OcrHistoryEntry) => {
      const firstSample = entry.supportTimestampsMs[0];
      if (firstSample === undefined || entry.status !== 'succeeded') return;
      setRevision(entry);
      onSeek(firstSample);
    },
    [onSeek],
  );
  const jump = useCallback(
    () => onSeek(selectedTimestamp),
    [onSeek, selectedTimestamp],
  );
  const retry = useCallback(() => {
    void reloadDocument();
  }, [reloadDocument]);
  const loadMore = useCallback(() => {
    void loadOlderRevisions();
  }, [loadOlderRevisions]);
  const retryHistory = useCallback(() => {
    void reloadHistory();
  }, [reloadHistory]);
  return (
    <InspectorSection title="OCR text">
      <Tabs onValueChange={setTab} value={tab}>
        <TabsList aria-label="OCR views" className="grid w-full grid-cols-3">
          <TabsTrigger value="text">Text</TabsTrigger>
          <TabsTrigger value="changes">Changes</TabsTrigger>
          <TabsTrigger value="details">Details</TabsTrigger>
        </TabsList>
        {selected.updateError ? (
          <p className="text-muted-foreground text-[10px]" role="status">
            {selected.updateError}
          </p>
        ) : null}
        {revision ? (
          <div className="text-muted-foreground flex items-center justify-between gap-1 text-[10px]">
            <span>Revision at {formatTimelineOffset(selectedTimestamp)}</span>
            <Button onClick={jump} size="sm" variant="ghost">
              Jump to revision
            </Button>
          </div>
        ) : null}
        <TabsContent value="text">
          <OcrDocumentState
            document={document}
            job={selected.job}
            error={selected.error}
            loading={selected.status === 'loading'}
            onRetryRead={retry}
            recordingId={context.recordingId}
          />
        </TabsContent>
        <TabsContent value="changes">
          <OcrHistoryPanel
            error={history.error}
            hasMore={history.hasMore}
            items={history.items}
            loading={history.status === 'loading'}
            onLoadMore={loadMore}
            onRetry={retryHistory}
            onSelect={selectRevision}
            selectedId={revision?.id ?? document?.id ?? null}
          />
        </TabsContent>
        <TabsContent value="details">
          {document ? (
            <OcrDetailsPanel
              debugEnabled={context.debugEnabled}
              document={document}
              key={document.id}
            />
          ) : (
            <p className="text-muted-foreground text-[11px]">
              Select a frame with available OCR text to inspect its details.
            </p>
          )}
        </TabsContent>
      </Tabs>
    </InspectorSection>
  );
}
