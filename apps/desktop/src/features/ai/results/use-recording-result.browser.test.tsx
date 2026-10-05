import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { useRecordingResult } from './use-recording-result';

interface Result {
  recordingId: string;
  text: string;
}
function deferred<T>() {
  let resolve: (value: T) => void = () => {
    throw new Error('uninitialized promise');
  };
  let reject: (error: Error) => void = () => {
    throw new Error('uninitialized promise');
  };
  const promise = new Promise<T>((nextResolve, nextReject) => {
    resolve = nextResolve;
    reject = nextReject;
  });
  return { promise, resolve, reject };
}
function ResultView({
  revision,
  read,
}: {
  revision: string;
  read: (input: { recordingId: string }) => Promise<Result>;
}) {
  const result = useRecordingResult({
    recordingId: 'recording-1',
    visibilityRevision: revision,
    processingSignature: '',
    read,
  });
  return (
    <div>
      {result.state?.text ?? 'No visible result'}
      {result.error ? <p role="alert">{result.error}</p> : null}
    </div>
  );
}
afterEach(cleanup);

test('a redaction hides in-flight private results and failed reloads cannot restore them', async () => {
  const stale = deferred<Result>();
  const redacted = deferred<Result>();
  const read = vi
    .fn()
    .mockReturnValueOnce(stale.promise)
    .mockReturnValueOnce(redacted.promise);
  const { rerender } = render(<ResultView revision="original" read={read} />);
  await waitFor(() => expect(read).toHaveBeenCalledTimes(1));
  rerender(<ResultView revision="cut" read={read} />);
  await waitFor(() => expect(read).toHaveBeenCalledTimes(2));
  stale.resolve({ recordingId: 'recording-1', text: 'private stale result' });
  redacted.reject(new Error('Reload unavailable'));
  expect((await screen.findByRole('alert')).textContent).toBe(
    'Reload unavailable',
  );
  expect(screen.queryByText('private stale result')).toBeNull();
  expect(screen.getByText('No visible result')).toBeTruthy();
});
