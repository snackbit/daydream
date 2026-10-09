import type { LibraryRecordingDetail } from '@daydream/shared';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page } from 'vitest/browser';
import { LibraryRecoveryAction } from './library-recovery-action';

const native = vi.hoisted(() => ({
  recoverLibraryRecording: vi.fn(),
  navigate: vi.fn(),
}));
vi.mock('../recovery-client', () => native);
vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => native.navigate,
}));
const detail: LibraryRecordingDetail = {
  id: 'synthetic-original',
  status: 'failed',
  availability: 'incomplete',
  durationMs: 1000,
  endedAtMs: 2000,
  startedAtMs: 1000,
  playableSegmentCount: 0,
  missingReasons: [],
  redactions: [],
  segments: [],
  sizeBytes: 100,
  storageDir: '/tmp/synthetic-original',
  thumbnailPath: null,
  title: null,
  tracks: [],
};
beforeEach(() => {
  vi.resetAllMocks();
  native.recoverLibraryRecording.mockResolvedValue({
    operationId: 'synthetic-operation',
    originalRecordingId: detail.id,
    recoveredRecordingId: 'synthetic-copy',
    status: 'recovered-partial',
    recoveredSegmentCount: 1,
    skippedSegmentCount: 1,
    coverage: [],
  });
});
afterEach(cleanup);
test('recovery requires explicit confirmation and opens the newly recovered copy', async () => {
  render(<LibraryRecoveryAction detail={detail} />);
  await page
    .getByRole('button', { name: 'Recover recording', exact: true })
    .click();
  expect(native.recoverLibraryRecording).not.toHaveBeenCalled();
  await page.getByRole('button', { name: 'Create recovered copy' }).click();
  await waitFor(() =>
    expect(native.recoverLibraryRecording).toHaveBeenCalledExactlyOnceWith({
      recordingId: detail.id,
    }),
  );
  await page.getByRole('button', { name: 'View recovered recording' }).click();
  expect(native.navigate).toHaveBeenCalledExactlyOnceWith({
    to: '/library/$recordingId',
    params: { recordingId: 'synthetic-copy' },
  });
});
test('media repair failure is visible and allows another explicit attempt', async () => {
  native.recoverLibraryRecording.mockRejectedValueOnce(
    'No usable media was recovered.',
  );
  render(<LibraryRecoveryAction detail={detail} />);
  await page
    .getByRole('button', { name: 'Recover recording', exact: true })
    .click();
  await page.getByRole('button', { name: 'Create recovered copy' }).click();
  await waitFor(() =>
    expect(screen.getByRole('alert').textContent).toContain('No usable media'),
  );
  expect(native.navigate).not.toHaveBeenCalled();
  await page.getByRole('button', { name: 'Create recovered copy' }).click();
  await page.getByRole('button', { name: 'View recovered recording' }).click();
  expect(native.recoverLibraryRecording).toHaveBeenCalledTimes(2);
});
