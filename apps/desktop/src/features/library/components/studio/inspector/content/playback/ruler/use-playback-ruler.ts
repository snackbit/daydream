import { useLayoutEffect, useState, type RefObject } from 'react';

export type PlaybackRulerPosition =
  { kind: 'visible'; top: number } | { kind: 'above' | 'below' };
export interface PlaybackRulerInput {
  viewport: HTMLDivElement | null;
  list: RefObject<HTMLOListElement | null>;
  positions: ReadonlyMap<string, number>;
  currentId: string | null;
  wordId: string | undefined;
  boundary: 'before' | 'after' | null;
}

/** Measure only the current mounted row, using the virtual list's stable index. */
export function usePlaybackRuler({
  viewport,
  list,
  positions,
  currentId,
  wordId,
  boundary,
}: PlaybackRulerInput) {
  const [position, setPosition] = useState<PlaybackRulerPosition | null>(null);
  useLayoutEffect(() => {
    const element = list.current;
    if (!viewport || !element) return;
    let handle: number | null = null;
    let observedRow: Element | null = null;
    const measure = () => {
      handle = null;
      const index = currentId === null ? undefined : positions.get(currentId);
      const row =
        index === undefined
          ? null
          : element.querySelector(`[data-index="${index}"]`);
      if (row !== observedRow) {
        if (observedRow) resize.unobserve(observedRow);
        if (row) resize.observe(row);
        observedRow = row;
      }
      let next: PlaybackRulerPosition | null = null;
      if (boundary) next = { kind: boundary === 'before' ? 'above' : 'below' };
      else if (row) {
        const word =
          wordId === undefined
            ? null
            : row.querySelector(`[data-word-id="${CSS.escape(wordId)}"]`);
        const rect = word?.getClientRects()[0];
        const top =
          (rect
            ? rect.top + rect.height / 2
            : row.getBoundingClientRect().top + 12) -
          viewport.getBoundingClientRect().top;
        next =
          top < 0
            ? { kind: 'above' }
            : top >= viewport.clientHeight
              ? { kind: 'below' }
              : { kind: 'visible', top };
      } else if (index !== undefined) {
        const first = element.firstElementChild;
        const firstIndex =
          first instanceof HTMLElement ? Number(first.dataset.index) : 0;
        next = { kind: index < firstIndex ? 'above' : 'below' };
      }
      setPosition((previous) =>
        previous?.kind === next?.kind &&
        (previous?.kind !== 'visible' ||
          (next?.kind === 'visible' && previous.top === next.top))
          ? previous
          : next,
      );
    };
    const schedule = () => {
      if (handle === null) handle = requestAnimationFrame(measure);
    };
    const resize = new ResizeObserver(schedule);
    resize.observe(viewport);
    resize.observe(element);
    const mutations = new MutationObserver(schedule);
    mutations.observe(element, { childList: true, subtree: true });
    viewport.addEventListener('scroll', schedule, { passive: true });
    measure();
    return () => {
      if (handle !== null) cancelAnimationFrame(handle);
      resize.disconnect();
      mutations.disconnect();
      viewport.removeEventListener('scroll', schedule);
    };
  }, [viewport, list, positions, currentId, wordId, boundary]);
  return position;
}
