import { useCallback } from 'react';
import { Moon, Sun } from 'lucide-react';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useAppearanceStore } from '../appearance-store';

export function AppearanceControl() {
  const mode = useAppearanceStore((state) => state.mode);
  const persistence = useAppearanceStore((state) => state.persistence);
  const setMode = useAppearanceStore((state) => state.setMode);

  const handleValueChange = useCallback(
    (value: string) => {
      if (value === 'dark' || value === 'light') {
        setMode(value);
      }
    },
    [setMode],
  );

  return (
    <>
      <Select onValueChange={handleValueChange} value={mode}>
        <SelectTrigger
          aria-describedby={
            persistence === 'session-only'
              ? 'appearance-storage-status'
              : undefined
          }
          aria-label="Color appearance"
          className="w-28 rounded-full"
        >
          {mode === 'dark' ? (
            <Moon
              aria-hidden="true"
              className="text-muted-foreground size-3.5"
            />
          ) : (
            <Sun
              aria-hidden="true"
              className="text-muted-foreground size-3.5"
            />
          )}
          <SelectValue />
        </SelectTrigger>
        <SelectContent align="end">
          <SelectItem value="dark">Dark</SelectItem>
          <SelectItem value="light">Light</SelectItem>
        </SelectContent>
      </Select>
      {persistence === 'session-only' ? (
        <span className="sr-only" id="appearance-storage-status" role="status">
          Appearance changed for this session, but browser storage is
          unavailable. It will reset when the app restarts.
        </span>
      ) : null}
    </>
  );
}
