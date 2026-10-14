import { useMemo } from 'react';
import type {
  CreateLibraryRedactionRangeInput,
  LibraryRecordingDetail,
  RecordingAiProcessingState,
  RecallEvidence,
  RecordingActivity,
} from '@daydream/shared';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import type { StudioInspectorSelection } from '../navigation/studio-inspector-selection';
import type { StudioVisualFrame } from '../observations/studio-visual-observations';
import { buildInspectorContent } from './content/inspector-content';
import type { InspectorContentEntry } from './content/inspector-content';
import { PlaybackInspectorFeed } from './content/playback-inspector-feed';
import { type StudioSourceLocation } from './details/source-location-inspector';
import { renderStudioSidebarDetails } from './details/studio-sidebar-details';
import { useStudioInspectorSelection } from './details/use-studio-inspector-selection';
import { useStudioSidebarMode } from './use-studio-sidebar-mode';

export interface StudioSidebarActions {
  readPlaybackTimeMs?: (() => number) | undefined;
  onClearActivitySelection: () => void;
  onCreateRedactions: (
    ranges: CreateLibraryRedactionRangeInput[],
  ) => Promise<void>;
  onDeleteRedaction: (redactionId: string) => Promise<void>;
  onSeek: (timelineOffsetMs: number) => void;
  onNavigateEntry?: ((entry: InspectorContentEntry) => void) | undefined;
  onUnredactRanges: (
    ranges: CreateLibraryRedactionRangeInput[],
  ) => Promise<void>;
}

export interface StudioSidebarModel {
  resolveSource?:
    | ((trackId: string, time: number) => StudioSourceLocation | undefined)
    | undefined;
  summaryDetail?: LibraryRecordingDetail | undefined;
  summaryProcessingState?: RecordingAiProcessingState | null | undefined;
  seekRevision?: number | undefined;
  aiProcessingState: RecordingAiProcessingState | null;
  detail: LibraryRecordingDetail;
  selection: StudioInspectorSelection | null;
  visualFrames: StudioVisualFrame[];
  debugEnabled?: boolean;
  timelineOffsetMs?: number;
  recallEvidence?: RecallEvidence | null;
  activity?: RecordingActivity | null;
  isPlaying?: boolean;
  resetRevision?: number;
}

export interface StudioSidebarProps {
  actions: StudioSidebarActions;
  model: StudioSidebarModel;
}

export function StudioSidebar({ actions, model }: StudioSidebarProps) {
  const content = useMemo(
    () =>
      buildInspectorContent({
        detail: model.detail,
        state: model.aiProcessingState,
        activity: model.activity ?? null,
        visualFrames: model.visualFrames,
      }),
    [model.detail, model.aiProcessingState, model.activity, model.visualFrames],
  );
  const selection = useStudioInspectorSelection(model);
  const { available, mode, changeMode } = useStudioSidebarMode({
    model,
    ...selection,
  });

  return (
    <aside
      data-studio-preserve-selection
      className="bg-muted text-foreground rounded-panel flex h-full min-h-0 min-w-0 flex-col overflow-hidden"
    >
      <Tabs
        value={mode}
        onValueChange={changeMode}
        className="min-h-0 flex-1 gap-0"
      >
        <div className="shrink-0 border-b px-6 py-5">
          <TabsList
            aria-label="Studio sidebar"
            variant="underline"
            className="w-full"
          >
            <TabsTrigger value="teleprompter" disabled={!available}>
              Review
            </TabsTrigger>
            <TabsTrigger value="inspector">Details</TabsTrigger>
          </TabsList>
        </div>
        <TabsContent
          value="teleprompter"
          forceMount
          className="flex min-h-0 flex-1 flex-col overflow-hidden data-[state=inactive]:hidden"
        >
          <PlaybackInspectorFeed
            active={mode === 'teleprompter'}
            ownerId={model.detail.id}
            durationMs={model.detail.durationMs ?? undefined}
            index={content}
            time={model.timelineOffsetMs ?? 0}
            isPlaying={mode === 'teleprompter' && model.isPlaying === true}
            onSeek={actions.onSeek}
            onNavigateEntry={actions.onNavigateEntry}
            readPlaybackTimeMs={actions.readPlaybackTimeMs}
            seekRevision={model.seekRevision}
          />
        </TabsContent>
        <TabsContent
          value="inspector"
          forceMount
          className="flex min-h-0 flex-1 flex-col overflow-hidden data-[state=inactive]:hidden"
        >
          <ScrollArea className="min-h-0 flex-1 overflow-hidden">
            {renderStudioSidebarDetails({
              active: mode === 'inspector',
              actions,
              model,
              content,
              selection,
            })}
          </ScrollArea>
        </TabsContent>
      </Tabs>
    </aside>
  );
}
