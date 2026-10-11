import { useRef } from 'react';
import { useBlocker } from '@tanstack/react-router';

/** A sheet owns its draft until Done or an explicit discard. */
export function useEditorNavigation(dirty: boolean, saving: boolean) {
  const closing = useRef(false);
  const blocker = useBlocker({
    enableBeforeUnload: dirty,
    shouldBlockFn: ({ current, next }) =>
      !closing.current &&
      (dirty || saving) &&
      (current.pathname !== next.pathname ||
        JSON.stringify(current.search) !== JSON.stringify(next.search)),
    withResolver: true,
  });
  function finish(close: () => void) {
    closing.current = true;
    if (blocker.status === 'blocked') blocker.proceed();
    else close();
  }
  function stay() {
    if (blocker.status === 'blocked') blocker.reset();
  }
  return { blocked: blocker.status === 'blocked', finish, stay };
}
