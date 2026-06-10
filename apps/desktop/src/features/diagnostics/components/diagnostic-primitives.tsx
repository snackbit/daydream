import type { ReactNode } from 'react';
import type { RecorderMediaWriterCapabilities } from '@daydream/shared';
import { CheckCircle2, XCircle } from 'lucide-react';
import { Badge } from '@/components/ui/badge';

function formatValue(value: string | null, emptyValue: string): string {
  return value && value.length > 0 ? value : emptyValue;
}

export interface MediaWriterCapabilityListProps {
  capabilities: RecorderMediaWriterCapabilities | null;
}

const mediaWriterCapabilityRows = [
  {
    key: 'appsrc',
    label: 'appsrc',
    requiredFor: 'video',
  },
  {
    key: 'matroskaMuxer',
    label: 'Matroska muxer',
    requiredFor: 'segment output',
  },
  {
    key: 'videoConvert',
    label: 'videoconvert',
    requiredFor: 'pixel format conversion',
  },
  {
    key: 'x264Encoder',
    label: 'x264enc',
    requiredFor: 'video',
  },
  {
    key: 'h264Parse',
    label: 'h264parse',
    requiredFor: 'video',
  },
  {
    key: 'pulseSource',
    label: 'pulsesrc',
    requiredFor: 'audio sources',
  },
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
  {
    key: 'opusEncoder',
    label: 'opusenc',
    requiredFor: 'audio sources',
  },
  {
    key: 'queue',
    label: 'queue',
    requiredFor: 'pipeline buffering',
  },
  {
    key: 'fileSink',
    label: 'filesink',
    requiredFor: 'segment output',
  },
] satisfies Array<{
  key: keyof RecorderMediaWriterCapabilities;
  label: string;
  requiredFor: string;
}>;

export function MediaWriterCapabilityList({
  capabilities,
}: MediaWriterCapabilityListProps) {
  return (
    <div className="mt-4">
      <h3 className="text-muted-foreground text-xs font-medium tracking-normal">
        Required capabilities
      </h3>
      <div className="mt-2 grid gap-2">
        {mediaWriterCapabilityRows.map((capability) => (
          <MediaWriterCapabilityRow
            key={capability.key}
            label={capability.label}
            requiredFor={capability.requiredFor}
            supported={capabilities?.[capability.key] ?? false}
          />
        ))}
      </div>
    </div>
  );
}

interface MediaWriterCapabilityRowProps {
  label: string;
  requiredFor: string;
  supported: boolean;
}

function MediaWriterCapabilityRow({
  label,
  requiredFor,
  supported,
}: MediaWriterCapabilityRowProps) {
  return (
    <div className="bg-muted/40 grid grid-cols-[1fr_auto] items-center gap-3 p-2 text-sm">
      <div className="min-w-0">
        <div className="flex items-center gap-2">
          {supported ? (
            <CheckCircle2 className="text-primary size-3.5 shrink-0" />
          ) : (
            <XCircle className="text-destructive size-3.5 shrink-0" />
          )}
          <span className="truncate">{label}</span>
        </div>
        <div className="text-muted-foreground mt-0.5 text-xs">
          Required for {requiredFor}
        </div>
      </div>
      <Badge variant={supported ? 'secondary' : 'destructive'}>
        {supported ? 'Supported' : 'Missing'}
      </Badge>
    </div>
  );
}

export interface HealthBlockProps {
  available: boolean;
  children: ReactNode;
  label: string;
}

export function HealthBlock({ available, children, label }: HealthBlockProps) {
  return (
    <div className="bg-muted/40 border p-3">
      <div className="flex items-center gap-2 text-sm">
        {available ? (
          <CheckCircle2 className="text-primary size-4" />
        ) : (
          <XCircle className="text-destructive size-4" />
        )}
        <span>{label}</span>
      </div>
      <dl className="mt-3 grid gap-2 text-sm">{children}</dl>
    </div>
  );
}

export interface DiagnosticRowProps {
  emptyValue?: string;
  label: string;
  value: string | null;
}

export function DiagnosticRow({
  emptyValue = 'Unavailable',
  label,
  value,
}: DiagnosticRowProps) {
  return (
    <div className="grid min-h-5 grid-cols-[8rem_1fr] items-baseline gap-3">
      <dt className="text-muted-foreground text-sm leading-5">{label}</dt>
      <dd className="min-w-0 font-mono text-xs leading-5 wrap-break-word">
        {formatValue(value, emptyValue)}
      </dd>
    </div>
  );
}
