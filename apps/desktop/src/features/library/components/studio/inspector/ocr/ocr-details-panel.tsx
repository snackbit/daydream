import { useState } from 'react';
import type { OcrDocument } from '@daydream/shared';
import { Button } from '@/components/ui/button';
import { InspectorField } from '../inspector-section';

export interface OcrDetailsPanelProps {
  document: OcrDocument;
  debugEnabled: boolean;
}
export function OcrDetailsPanel({
  document,
  debugEnabled,
}: OcrDetailsPanelProps) {
  const [rawOpen, setRawOpen] = useState(false);
  function toggleRaw() {
    setRawOpen((open) => !open);
  }
  return (
    <div className="space-y-2">
      <dl className="space-y-1.5">
        <InspectorField
          label="Engine"
          value={
            document.engine === 'vision'
              ? 'Vision OCR'
              : document.engine === 'legacy'
                ? 'Legacy OCR'
                : 'Tesseract'
          }
        />
        <InspectorField label="Provider" value={document.provider} />
        <InspectorField label="Status" value={document.status} />
        <InspectorField
          label="Model"
          value={document.model ?? 'Not provided'}
        />
        <InspectorField label="Source" value={document.sourceTrackId} />
        <InspectorField
          label="Image"
          value={
            document.width === null || document.height === null
              ? 'Unavailable for this source'
              : `${document.width} × ${document.height}`
          }
        />
        <InspectorField
          label="Source"
          value={
            document.provenance === 'reconstructed'
              ? 'Reconstructed legacy words'
              : 'Native OCR document'
          }
        />
        <InspectorField
          label="Cache"
          value={document.cacheHit ? 'Exact image reused' : 'Processed image'}
        />
      </dl>
      {debugEnabled && document.status === 'succeeded' ? (
        <>
          <Button
            aria-expanded={rawOpen}
            onClick={toggleRaw}
            size="sm"
            variant="outline"
          >
            {rawOpen ? 'Hide raw OCR details' : 'Show raw OCR details'}
          </Button>
          {rawOpen ? (
            <div className="space-y-2">
              <dl className="space-y-1.5">
                <InspectorField
                  label="Document ID"
                  value={<code className="break-all">{document.id}</code>}
                />
                <InspectorField
                  label="Segment ID"
                  value={
                    <code className="break-all">{document.segmentId}</code>
                  }
                />
                <InspectorField
                  label="Frame reference"
                  value={
                    <code className="break-all">{document.sourceFrameRef}</code>
                  }
                />
                <InspectorField
                  label="Settings"
                  value={
                    <code className="break-all">
                      {document.configFingerprint}
                    </code>
                  }
                />
              </dl>
              {document.blocks.map((block) => (
                <div className="border-t pt-2" key={block.id}>
                  <p className="text-xs font-medium">Block {block.order + 1}</p>
                  <p className="text-sm break-words">{block.text}</p>
                  <div className="mt-1 divide-y">
                    {block.words.map((word) => (
                      <div
                        className="grid grid-cols-[1fr_auto] gap-2 py-1 text-xs"
                        key={word.id}
                      >
                        <span className="break-words">{word.text}</span>
                        <span>
                          {word.confidence === null
                            ? 'Unknown'
                            : word.confidence.toFixed(3)}
                        </span>
                        {word.region ? (
                          <code className="col-span-2 break-all">
                            {word.region.x}, {word.region.y},{' '}
                            {word.region.width} × {word.region.height}
                          </code>
                        ) : null}
                        {word.rawRegionId ? (
                          <code className="text-muted-foreground col-span-2 break-all">
                            {word.rawRegionId}
                          </code>
                        ) : null}
                      </div>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          ) : null}
        </>
      ) : (
        <p className="text-muted-foreground text-xs">
          {document.status === 'succeeded'
            ? 'Raw region details are available when Debug is enabled in Settings.'
            : 'Raw OCR details are unavailable for this sample.'}
        </p>
      )}
    </div>
  );
}
