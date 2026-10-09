import { useRef, useState } from 'react';
import type { CreateLibraryRedactionRangeInput } from '@daydream/shared';
import { unknownErrorMessage } from '@/lib/async-store-action';
import {
  useStudioTimelineStore,
  type StudioTimeRangeSelection,
} from '../studio-timeline-store';
import { redactionRangesFromSelection } from '../track-timeline-redaction-actions';

/** Owns user-visible completion of a privacy write, without swallowing client failures upstream. */
export function useRedactionMutation() {
  const inFlight = useRef(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function run(write: () => Promise<void>, onSuccess?: () => void) {
    if (inFlight.current) return;
    inFlight.current = true;
    setPending(true);
    setError(null);
    try {
      await write();
      onSuccess?.();
    } catch (writeError) {
      setError(unknownErrorMessage(writeError));
    } finally {
      inFlight.current = false;
      setPending(false);
    }
  }

  return { error, pending, run };
}

export function useSelectionRedactionMutation({
  ranges,
  onCreate,
  onUnredact,
  onClear,
}: {
  ranges: StudioTimeRangeSelection[];
  onCreate: (ranges: CreateLibraryRedactionRangeInput[]) => Promise<void>;
  onUnredact: (ranges: CreateLibraryRedactionRangeInput[]) => Promise<void>;
  onClear: () => void;
}) {
  const mutation = useRedactionMutation();
  function submit(
    write: (input: CreateLibraryRedactionRangeInput[]) => Promise<void>,
  ) {
    if (ranges.length === 0) return;
    const input = redactionRangesFromSelection(ranges);
    const submittedSignature = JSON.stringify(input);
    const recordingId =
      useStudioTimelineStore.getState().initializedRecordingId;
    function clearSubmittedSelection() {
      const current = useStudioTimelineStore.getState();
      if (
        current.initializedRecordingId === recordingId &&
        JSON.stringify(
          redactionRangesFromSelection(current.selection.ranges),
        ) === submittedSignature
      )
        onClear();
    }
    void mutation.run(() => write(input), clearSubmittedSelection);
  }
  function redact() {
    submit(onCreate);
  }
  function unredact() {
    submit(onUnredact);
  }
  return { error: mutation.error, pending: mutation.pending, redact, unredact };
}
