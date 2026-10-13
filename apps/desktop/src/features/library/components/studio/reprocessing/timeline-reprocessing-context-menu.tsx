import {
  useCallback,
  useState,
  type MouseEvent,
  type ReactNode,
  type RefObject,
} from 'react';
import type {
  LibraryRecordingTrack,
  ReprocessingTarget,
} from '@daydream/shared';
import { RefreshCw, ZoomIn } from 'lucide-react';
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from '@/components/ui/context-menu';
import type { StudioTimeRangeSelection } from '../studio-timeline-store';
import { TRACK_LABEL_WIDTH_PX } from '../track-timeline-math';
import { reprocessingTargets } from './reprocessing-targets';

export interface TimelineReprocessingContextMenuProps {
  children: ReactNode;
  contentRef: RefObject<HTMLDivElement | null>;
  ranges: StudioTimeRangeSelection[];
  tracks: LibraryRecordingTrack[];
  timelineWidthPx: number;
  totalDurationMs: number;
  onReprocess: ((targets: ReprocessingTarget[]) => void) | undefined;
  onOpenChange: (open: boolean) => void;
  onZoomSelection?: (() => void) | undefined;
}
export function TimelineReprocessingContextMenu({
  children,
  contentRef,
  ranges,
  tracks,
  timelineWidthPx,
  totalDurationMs,
  onReprocess,
  onOpenChange,
  onZoomSelection,
}: TimelineReprocessingContextMenuProps) {
  const [target, setTarget] = useState<{
    label: string;
    targets: ReprocessingTarget[];
  }>({ label: 'selection', targets: [] });
  const prepare = useCallback(
    (event: MouseEvent<HTMLDivElement>) => {
      if (!(event.target instanceof Element)) return;
      const content = contentRef.current;
      if (!content) return;
      const trackId = event.target
        .closest('[data-track-id]')
        ?.getAttribute('data-track-id');
      const time =
        ((event.clientX -
          content.getBoundingClientRect().left -
          TRACK_LABEL_WIDTH_PX) /
          timelineWidthPx) *
        totalDurationMs;
      const inside = ranges.some(
        (range) =>
          time >= range.startMs &&
          time <= range.endMs &&
          (trackId
            ? range.laneIds.includes(trackId)
            : event.target instanceof Element &&
              event.target.closest('[data-time-ruler]') !== null),
      );
      const track = tracks.find((track) => track.id === trackId);
      const targets = inside
        ? reprocessingTargets(ranges, new Set(tracks.map((track) => track.id)))
        : track
          ? [{ trackId: track.id, ranges: null }]
          : [];
      setTarget({
        label: inside ? 'selection' : (track?.label ?? 'selection'),
        targets,
      });
    },
    [contentRef, ranges, tracks, timelineWidthPx, totalDurationMs],
  );
  const reprocess = useCallback(() => {
    onReprocess?.(target.targets);
  }, [onReprocess, target.targets]);
  return (
    <ContextMenu onOpenChange={onOpenChange}>
      <ContextMenuTrigger asChild>
        <div className="contents" onContextMenuCapture={prepare}>
          {children}
        </div>
      </ContextMenuTrigger>
      <ContextMenuContent data-studio-preserve-selection className="z-70">
        {onZoomSelection ? (
          <ContextMenuItem
            onSelect={onZoomSelection}
            disabled={ranges.length === 0}
          >
            <ZoomIn />
            Zoom In
          </ContextMenuItem>
        ) : null}
        {onReprocess ? (
          <ContextMenuItem
            onSelect={reprocess}
            disabled={target.targets.length === 0}
          >
            <RefreshCw />
            Reprocess {target.label}…
          </ContextMenuItem>
        ) : null}
      </ContextMenuContent>
    </ContextMenu>
  );
}
