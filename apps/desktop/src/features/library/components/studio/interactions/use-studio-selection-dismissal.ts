import { useEffect } from 'react';
import { useStudioTimelineStore } from '../studio-timeline-store';

export function useStudioSelectionDismissal(
  onClear: (reason: 'outside' | 'escape') => void,
) {
  useEffect(() => {
    let preservePointerClick = false;
    function pointerDown(event: PointerEvent) {
      preservePointerClick =
        event.target instanceof Element &&
        event.target.closest(
          '[data-studio-preserve-selection], [role="dialog"], [role="menu"], [role="listbox"]',
        ) !== null;
    }
    function clear(reason: 'outside' | 'escape') {
      useStudioTimelineStore.getState().clearSelection();
      onClear(reason);
    }
    function click(event: MouseEvent) {
      // Radix can remove its popup on pointerup, retargeting the subsequent
      // click to body. Keep the ownership of the original pointer gesture.
      const preserve = event.detail > 0 && preservePointerClick;
      preservePointerClick = false;
      if (preserve) return;
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
      clear('outside');
    }
    function key(event: KeyboardEvent) {
      if (event.key !== 'Escape' || event.defaultPrevented) return;
      if (
        document.querySelector(
          '[role="menu"], [role="dialog"][data-state="open"]',
        )
      )
        return;
      clear('escape');
    }
    document.addEventListener('click', click, true);
    document.addEventListener('pointerdown', pointerDown, true);
    document.addEventListener('keydown', key);
    return () => {
      document.removeEventListener('click', click, true);
      document.removeEventListener('pointerdown', pointerDown, true);
      document.removeEventListener('keydown', key);
    };
  }, [onClear]);
}
