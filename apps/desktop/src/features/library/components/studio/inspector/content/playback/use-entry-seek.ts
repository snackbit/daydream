import { useCallback, type MouseEvent } from 'react';
import type { InspectorContentEntry } from '../inspector-content';

export function useEntrySeek(
  entry: InspectorContentEntry,
  onSeek: (time: number) => void,
  onNavigateEntry?: ((entry: InspectorContentEntry) => void) | undefined,
) {
  return useCallback(
    (event: MouseEvent<HTMLElement>) => {
      const selection = window.getSelection();
      if (
        event.detail > 0 &&
        selection &&
        !selection.isCollapsed &&
        (event.currentTarget.contains(selection.anchorNode) ||
          event.currentTarget.contains(selection.focusNode))
      )
        return;
      if (onNavigateEntry) onNavigateEntry(entry);
      else onSeek(entry.startMs);
    },
    [entry, onSeek, onNavigateEntry],
  );
}
