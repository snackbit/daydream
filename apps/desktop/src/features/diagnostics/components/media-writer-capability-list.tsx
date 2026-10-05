import type { RecorderMediaWriterCapabilities } from '@daydream/shared';
import { CheckCircle2, XCircle } from 'lucide-react';
import { Badge } from '@/components/ui/badge';

interface MediaWriterCapabilityListProps {
  capabilities: RecorderMediaWriterCapabilities | null;
}

interface CapabilityRow {
  key: keyof RecorderMediaWriterCapabilities;
  label: string;
  requiredFor: string;
}

interface CapabilityGroup {
  id: string;
  label: string;
  rows: CapabilityRow[];
}

const capabilityGroups: CapabilityGroup[] = [
  {
    id: 'video',
    label: 'Video output',
    rows: [
      { key: 'appsrc', label: 'appsrc', requiredFor: 'video frames' },
      {
        key: 'videoConvert',
        label: 'videoconvert',
        requiredFor: 'pixel format conversion',
      },
      { key: 'x264Encoder', label: 'x264enc', requiredFor: 'video encoding' },
      { key: 'h264Parse', label: 'h264parse', requiredFor: 'video output' },
      {
        key: 'matroskaMuxer',
        label: 'Matroska muxer',
        requiredFor: 'segment output',
      },
      { key: 'fileSink', label: 'filesink', requiredFor: 'segment output' },
      { key: 'queue', label: 'queue', requiredFor: 'pipeline buffering' },
    ],
  },
  {
    id: 'audio',
    label: 'Audio input and encoding',
    rows: [
      { key: 'pulseSource', label: 'pulsesrc', requiredFor: 'audio sources' },
      {
        key: 'audioConvert',
        label: 'audioconvert',
        requiredFor: 'audio sources',
      },
      {
        key: 'audioResample',
        label: 'audioresample',
        requiredFor: 'audio sources',
      },
      { key: 'opusEncoder', label: 'opusenc', requiredFor: 'audio encoding' },
    ],
  },
];

export function MediaWriterCapabilityList({
  capabilities,
}: MediaWriterCapabilityListProps) {
  const missingCount = capabilityGroups
    .flatMap((group) => group.rows)
    .filter((row) => capabilities?.[row.key] !== true).length;

  return (
    <section className="mt-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h3 className="text-muted-foreground text-xs font-medium">
          Required capabilities
        </h3>
        <Badge variant={missingCount === 0 ? 'secondary' : 'outline'}>
          {missingCount === 0 ? 'All supported' : `${missingCount} unavailable`}
        </Badge>
      </div>

      <div className="mt-2 grid gap-2">
        {capabilityGroups.map((group) => (
          <CapabilityGroupDetails
            capabilities={capabilities}
            group={group}
            key={group.id}
          />
        ))}
      </div>
    </section>
  );
}

function CapabilityGroupDetails({
  capabilities,
  group,
}: {
  capabilities: RecorderMediaWriterCapabilities | null;
  group: CapabilityGroup;
}) {
  const missingCount = group.rows.filter(
    (row) => capabilities?.[row.key] !== true,
  ).length;

  return (
    <details className="bg-muted/40 rounded-lg border px-3 py-2">
      <summary className="flex cursor-pointer list-none items-center justify-between gap-3 text-xs font-medium marker:hidden">
        <span>{group.label}</span>
        <Badge variant={missingCount === 0 ? 'secondary' : 'outline'}>
          {missingCount === 0 ? 'Ready' : `${missingCount} unavailable`}
        </Badge>
      </summary>
      <div className="mt-2 grid gap-1.5 border-t pt-2 sm:grid-cols-2">
        {group.rows.map((row) => (
          <CapabilityRowItem
            key={row.key}
            label={row.label}
            requiredFor={row.requiredFor}
            supported={capabilities?.[row.key] === true}
          />
        ))}
      </div>
    </details>
  );
}

function CapabilityRowItem({
  label,
  requiredFor,
  supported,
}: {
  label: string;
  requiredFor: string;
  supported: boolean;
}) {
  return (
    <div className="border-border/70 bg-card flex min-w-0 items-start justify-between gap-2 rounded-md border px-2.5 py-2">
      <div className="flex min-w-0 items-start gap-2">
        {supported ? (
          <CheckCircle2
            aria-hidden="true"
            className="text-primary mt-0.5 size-3.5 shrink-0"
          />
        ) : (
          <XCircle
            aria-hidden="true"
            className="text-destructive mt-0.5 size-3.5 shrink-0"
          />
        )}
        <div className="min-w-0">
          <div className="truncate text-xs font-medium">{label}</div>
          <div className="text-muted-foreground mt-0.5 text-[11px] leading-4">
            Needed for {requiredFor}
          </div>
        </div>
      </div>
      <Badge variant={supported ? 'secondary' : 'destructive'}>
        {supported ? 'Supported' : 'Missing'}
      </Badge>
    </div>
  );
}
