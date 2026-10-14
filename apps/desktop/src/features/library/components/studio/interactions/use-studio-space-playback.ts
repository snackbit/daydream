import { useCallback, useEffect } from 'react';

const spaceOwnedTargets = [
  'a',
  'audio',
  'button',
  'input',
  'select',
  'textarea',
  'video',
  '[contenteditable="true"]',
  '[role="button"]',
  '[role="combobox"]',
  '[role="dialog"]',
  '[role="slider"]',
  '[role="tab"]',
  '[role="textbox"]',
].join(',');

export function useStudioSpacePlayback(
  togglePlayback: () => Promise<unknown>,
  stepFrame?: (direction: -1 | 1) => Promise<unknown>,
) {
  const handleKeyDown = useCallback(
    (event: KeyboardEvent) => {
      if (
        !['Space', 'Comma', 'Period'].includes(event.code) ||
        (event.repeat && event.code === 'Space') ||
        event.defaultPrevented ||
        event.altKey ||
        event.ctrlKey ||
        event.metaKey ||
        isPlaybackKeyOwnedByControl(event.target, event.code)
      ) {
        return;
      }

      event.preventDefault();
      if (event.code === 'Space') void togglePlayback();
      else if (stepFrame) void stepFrame(event.code === 'Comma' ? -1 : 1);
    },
    [togglePlayback, stepFrame],
  );

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);

    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);
}

function isPlaybackKeyOwnedByControl(
  target: EventTarget | null,
  code: string,
): boolean {
  return (
    target instanceof Element &&
    target.closest(
      code === 'Space'
        ? spaceOwnedTargets
        : 'input,select,textarea,[contenteditable="true"],[role="dialog"],[role="textbox"],[role="combobox"]',
    ) !== null
  );
}
