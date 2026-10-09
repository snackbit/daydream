import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type ChangeEvent,
} from 'react';
import type {
  ProtectableWindow,
  ProtectableWindowList,
  WindowProtectionStatus,
} from '@daydream/shared';
import { Loader2, RefreshCw } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import {
  getWindowProtectionStatus,
  listProtectableWindows,
} from '@/features/recorder/recorder-client';
import { ProtectedWindowRow } from './protected-window-row';

export interface ProtectedWindowPickerProps {
  onPick: (window: ProtectableWindow) => void;
  onStatusLoaded: (status: WindowProtectionStatus) => void;
}
export function ProtectedWindowPicker({
  onPick,
  onStatusLoaded,
}: ProtectedWindowPickerProps) {
  const [open, setOpen] = useState(false);
  const [picker, setPicker] = useState<ProtectableWindowList | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState('');
  const request = useRef({ generation: 0 });
  const refresh = useCallback(async () => {
    const id = ++request.current.generation;
    setLoading(true);
    setError(null);
    try {
      const [nextPicker, status] = await Promise.all([
        listProtectableWindows(),
        getWindowProtectionStatus(),
      ]);
      if (id !== request.current.generation) return;
      setPicker(nextPicker);
      onStatusLoaded(status);
      if (nextPicker.status !== 'available')
        setError(
          status.status === nextPicker.status
            ? status.message
            : unavailablePickerMessage(nextPicker.status),
        );
    } catch {
      if (id === request.current.generation)
        setError('Windows could not be loaded. Try refreshing.');
    } finally {
      if (id === request.current.generation) setLoading(false);
    }
  }, [onStatusLoaded]);
  const handleRefresh = useCallback(() => void refresh(), [refresh]);
  const handleQuery = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => setQuery(event.target.value),
    [],
  );
  const handleOpenChange = useCallback(
    (nextOpen: boolean) => {
      setOpen(nextOpen);
      if (nextOpen) void refresh();
      else request.current.generation++;
    },
    [refresh],
  );
  const handlePick = useCallback(
    (window: ProtectableWindow) => {
      onPick(window);
      handleOpenChange(false);
    },
    [handleOpenChange, onPick],
  );
  useEffect(() => {
    const owner = request.current;
    return () => {
      owner.generation++;
    };
  }, []);
  const search = query.trim().toLocaleLowerCase();
  const windows =
    picker?.windows.filter(
      (window) =>
        !search ||
        [
          window.title,
          window.processName,
          window.appIdentity,
          window.processId?.toString(),
        ].some((value) => value?.toLocaleLowerCase().includes(search)),
    ) ?? [];
  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogTrigger asChild>
        <Button size="sm" type="button" variant="outline">
          Pick window
        </Button>
      </DialogTrigger>
      <DialogContent className="max-w-xl">
        <DialogHeader>
          <DialogTitle>Pick a window</DialogTitle>
          <DialogDescription>
            Add a rule from an open app window.
          </DialogDescription>
        </DialogHeader>
        <div className="flex items-center gap-2">
          <Input
            aria-label="Search windows"
            placeholder="Search title, app or process"
            value={query}
            onChange={handleQuery}
          />
          <Button
            aria-label="Refresh windows"
            disabled={loading}
            onClick={handleRefresh}
            size="icon"
            type="button"
            variant="outline"
          >
            <RefreshCw aria-hidden="true" className="size-4" />
          </Button>
        </div>
        <div
          aria-busy={loading}
          className="grid max-h-[min(480px,55dvh)] gap-1 overflow-y-auto"
        >
          {loading ? (
            <p
              role="status"
              className="text-muted-foreground text-control flex items-center gap-2 py-4"
            >
              <Loader2 aria-hidden="true" className="size-4 animate-spin" />
              Loading windows.
            </p>
          ) : error ? (
            <p role="alert" className="text-destructive text-control py-4">
              {error}
            </p>
          ) : windows.length === 0 ? (
            <p
              role="status"
              className="text-muted-foreground text-control py-4"
            >
              {search
                ? 'No windows match your search.'
                : 'No open app windows found.'}
            </p>
          ) : (
            windows.map((window) => (
              <ProtectedWindowRow
                key={window.id}
                window={window}
                onPick={handlePick}
              />
            ))
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}

function unavailablePickerMessage(
  status: Exclude<ProtectableWindowList['status'], 'available'>,
): string {
  if (status === 'permission-denied')
    return 'Allow focused-window access, then refresh the window list.';
  if (status === 'unsupported-platform')
    return 'Window detection is unavailable on this platform.';
  return 'Window detection is unavailable in this desktop session. Try refreshing.';
}
