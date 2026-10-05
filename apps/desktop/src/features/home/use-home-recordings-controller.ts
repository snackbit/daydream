import { useCallback, useEffect, useRef, useState } from 'react';
import type { LibraryRecordingSummary } from '@daydream/shared';
import { sortRecordings } from '@/features/library/components/overview/library-overview-utils';
import { listLibraryRecordings } from '@/features/library/library-client';
import { unknownErrorMessage } from '@/lib/async-store-action';

export interface HomeRecordingsModel {
  error: string | null;
  librarySizeBytes: number;
  recentRecordings: LibraryRecordingSummary[];
  recordingCount: number;
  status: 'error' | 'idle' | 'loading' | 'ready';
}

export interface HomeRecordingsActions {
  refresh: () => Promise<void>;
}

export interface HomeRecordingsController {
  actions: HomeRecordingsActions;
  model: HomeRecordingsModel;
}

const initialModel: HomeRecordingsModel = {
  error: null,
  librarySizeBytes: 0,
  recentRecordings: [],
  recordingCount: 0,
  status: 'idle',
};

const recentRecordingLimit = 4;

export function useHomeRecordingsController(): HomeRecordingsController {
  const [model, setModel] = useState(initialModel);
  const requestIdRef = useRef(0);

  const refresh = useCallback(async () => {
    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;
    setModel((current) => ({ ...current, error: null, status: 'loading' }));

    try {
      const recordings = await listLibraryRecordings();
      if (requestIdRef.current !== requestId) {
        return;
      }

      const sortedRecordings = sortRecordings(recordings, 'started');
      setModel({
        error: null,
        librarySizeBytes: recordings.reduce(
          (total, recording) => total + recording.sizeBytes,
          0,
        ),
        recentRecordings: sortedRecordings.slice(0, recentRecordingLimit),
        recordingCount: recordings.length,
        status: 'ready',
      });
    } catch (loadError) {
      if (requestIdRef.current !== requestId) {
        return;
      }

      setModel((current) => ({
        ...current,
        error: unknownErrorMessage(loadError),
        status: 'error',
      }));
    }
  }, []);

  useEffect(() => {
    void refresh();

    return () => {
      requestIdRef.current += 1;
    };
  }, [refresh]);

  return {
    actions: { refresh },
    model,
  };
}
