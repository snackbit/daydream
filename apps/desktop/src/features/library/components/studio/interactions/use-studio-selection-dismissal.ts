import { useEffect } from 'react';
import { useStudioTimelineStore } from '../studio-timeline-store';

export function useStudioSelectionDismissal(onClear: () => void) {
  useEffect(() => {
    function clear() {
      useStudioTimelineStore.getState().clearSelection();
      onClear();
    }
    function click(event: MouseEvent) {
      const target = event.target;
      if (!(target instanceof Element)) return;
      if (
        document.querySelector(
          '[role="dialog"][data-state="open"], [role="menu"][data-state="open"]',
        )
      )
        return;
      if (
        target.closest(
          '[data-studio-preserve-selection], [aria-label="Track timeline"], [data-studio-timeline-toolbar], [role="menu"], [role="dialog"], [data-slot="dialog-overlay"], [data-slot="sheet-overlay"]',
        )
      )
        return;
      if (event.ctrlKey || event.metaKey || event.shiftKey) return;
      clear();
    }
    function key(event: KeyboardEvent) {
      if (event.key !== 'Escape' || event.defaultPrevented) return;
      if (
        document.querySelector(
          '[role="menu"], [role="dialog"][data-state="open"]',
        )
      )
        return;
      clear();
    }
    document.addEventListener('click', click, true);
    document.addEventListener('keydown', key);
    return () => {
      document.removeEventListener('click', click, true);
      document.removeEventListener('keydown', key);
    };
  }, [onClear]);
}
