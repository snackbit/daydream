import { useCallback } from 'react';
import { useOcrDocument } from '@/features/ai/ocr/use-ocr-document';
import { formatTimelineOffset } from '../inspector-format';
import { InspectorField, InspectorSection } from '../inspector-section';
import { OcrDocumentState } from './ocr-document-state';

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
  showSourceLocation?: boolean;
}

/** Recognition metadata and recovery remain available without exposing OCR. */
export function OcrDocumentInspector({
  context,
  timestampMs,
  documentId = null,
  showSourceLocation = true,
}: OcrDocumentInspectorProps) {
  const selected = useOcrDocument(
    {
      recordingId: context.recordingId,
      sourceTrackId: context.sourceTrackId,
      timestampMs,
      documentId,
    },
    context.privacyRevision,
    context.processingRevision,
  );
  const reload = selected.reload;
  const retry = useCallback(() => {
    void reload();
  }, [reload]);
  return (
    <InspectorSection title="Screen recognition">
      <dl className="space-y-1.5 text-xs">
        {showSourceLocation ? (
          <>
            <InspectorField
              label="Time"
              value={formatTimelineOffset(timestampMs)}
            />
            <InspectorField label="Source" value={context.sourceTrackId} />
          </>
        ) : null}
        {context.debugEnabled && selected.document ? (
          <InspectorField label="Engine" value={selected.document.engine} />
        ) : null}
      </dl>
      {selected.updateError ? (
        <p className="text-muted-foreground text-xs" role="status">
          {selected.updateError}
        </p>
      ) : null}
      <OcrDocumentState
        document={selected.document}
        job={selected.job}
        error={selected.error}
        loading={selected.status === 'loading'}
        onRetryRead={retry}
        recordingId={context.recordingId}
      />
    </InspectorSection>
  );
}
