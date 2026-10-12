import type {
  LibraryDayTimeline,
  LibraryDayTimelineInput,
} from '@daydream/shared';
import { invoke, decodeResponse } from '@/lib/diagnostics/invoke';
import { decodeDayTimeline } from './day-timeline-decoder';

export async function getLibraryDayTimeline(
  input: LibraryDayTimelineInput,
): Promise<LibraryDayTimeline> {
  return decodeResponse(
    'getLibraryDayTimeline',
    await invoke<unknown>('getLibraryDayTimeline', { input }),
    decodeDayTimeline,
  );
}
