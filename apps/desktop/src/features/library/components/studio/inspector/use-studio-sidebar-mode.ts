import { useCallback, useState } from 'react';
import type { StudioSelection } from '../timeline/state/studio-timeline-store';
import type { StudioSidebarModel } from './studio-sidebar';

export interface StudioSidebarModeInput {
  model: StudioSidebarModel;
  timelineSelection: StudioSelection;
  hasTimelineSelection: boolean;
  selectionIntent: number;
}

export function useStudioSidebarMode({
  model,
  timelineSelection,
  hasTimelineSelection,
  selectionIntent,
}: StudioSidebarModeInput) {
  const available = model.detail.availability === 'available';
  // Data refreshes can recreate selection objects. Only a semantic selection
  // change represents user navigation; clearing it must retain the chosen tab.
  const selectionKey =
    model.recallEvidence?.evidenceId ??
    (model.selection
      ? `${model.selection.kind}:${model.selection.trackId}:${model.selection.timelineOffsetMs}`
      : hasTimelineSelection
        ? JSON.stringify(timelineSelection)
        : '');
  const [choice, setChoice] = useState({
    recordingId: model.detail.id,
    selectionKey,
    selectionIntent,
    mode: selectionKey ? 'inspector' : 'teleprompter',
  });
  if (
    choice.recordingId !== model.detail.id ||
    choice.selectionKey !== selectionKey ||
    choice.selectionIntent !== selectionIntent
  ) {
    setChoice({
      recordingId: model.detail.id,
      selectionKey,
      selectionIntent,
      mode: selectionKey ? 'inspector' : choice.mode,
    });
  }
  const mode = available ? choice.mode : 'inspector';
  const changeMode = useCallback((value: string) => {
    if (value === 'teleprompter' || value === 'inspector') {
      setChoice((current) => ({ ...current, mode: value }));
    }
  }, []);

  return { available, mode, changeMode };
}
