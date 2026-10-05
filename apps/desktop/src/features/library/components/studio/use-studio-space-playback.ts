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

export function useStudioSpacePlayback(togglePlayback: () => Promise<unknown>) {
  const handleKeyDown = useCallback(
    (event: KeyboardEvent) => {
      if (
        event.code !== 'Space' ||
        event.repeat ||
        event.defaultPrevented ||
        event.altKey ||
        event.ctrlKey ||
        event.metaKey ||
        isPlaybackKeyOwnedByControl(event.target)
      ) {
        return;
      }

      event.preventDefault();
      void togglePlayback();
    },
    [togglePlayback],
  );

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);

    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);
}

function isPlaybackKeyOwnedByControl(target: EventTarget | null): boolean {
  return (
    target instanceof Element && target.closest(spaceOwnedTargets) !== null
  );
}
