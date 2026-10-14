import type {
  LibraryDayTimeline,
  LibraryDayTimelineInput,
  LibraryDaysPage,
  ListRecallDaysInput,
} from '@daydream/shared';
import { decodeLibraryDaysPage } from '@/features/recall/recall-decoders';
import { invoke, decodeResponse } from '@/lib/diagnostics/invoke';
import { decodeDayTimeline } from './day-timeline-decoder';

export async function listLibraryDays(
  input: ListRecallDaysInput,
): Promise<LibraryDaysPage> {
  return decodeResponse(
    'listLibraryDays',
    await invoke<unknown>('listLibraryDays', { input }),
    decodeLibraryDaysPage,
  );
}

export async function getLibraryDayTimeline(
  input: LibraryDayTimelineInput,
): Promise<LibraryDayTimeline> {
  return decodeResponse(
    'getLibraryDayTimeline',
    await invoke<unknown>('getLibraryDayTimeline', { input }),
    decodeDayTimeline,
  );
}
