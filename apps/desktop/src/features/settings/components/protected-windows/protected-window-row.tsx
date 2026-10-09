import { useCallback } from 'react';
import type { ProtectableWindow } from '@daydream/shared';
import { Button } from '@/components/ui/button';
import { WindowApplicationIcon } from './window-application-icon';

export interface ProtectedWindowRowProps {
  window: ProtectableWindow;
  onPick: (window: ProtectableWindow) => void;
}
export function ProtectedWindowRow({
  window,
  onPick,
}: ProtectedWindowRowProps) {
  const handlePick = useCallback(() => onPick(window), [onPick, window]);
  const app =
    window.processName?.trim() || window.appIdentity?.trim() || 'Unknown app';
  return (
    <Button
      onClick={handlePick}
      type="button"
      variant="ghost"
      className="h-auto min-h-16 w-full justify-start gap-3 rounded-xl px-3 py-3 text-left whitespace-normal"
    >
      <WindowApplicationIcon iconKey={window.iconKey} />
      <span className="min-w-0 flex-1">
        <span className="text-control line-clamp-2 font-medium break-words">
          {window.title || app}
        </span>
        <span className="text-muted-foreground text-metadata mt-1 block break-words">
          {[
            app,
            window.appIdentity && window.appIdentity !== app
              ? window.appIdentity
              : null,
            window.processId !== null ? `PID ${window.processId}` : null,
          ]
            .filter(Boolean)
            .join(' · ')}
        </span>
      </span>
    </Button>
  );
}
