import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { GroupedRedactions } from './grouped-redactions';

const client = vi.hoisted(() => ({ resizeLibraryRedaction: vi.fn() }));
vi.mock('../../../library-client', () => client);
beforeEach(() => {
  client.resizeLibraryRedaction.mockReset();
  client.resizeLibraryRedaction.mockResolvedValue(undefined);
});
afterEach(cleanup);
const redaction = {
  createdAtMs: 0,
  startOffsetMs: 1000,
  endOffsetMs: 2000,
  id: 'synthetic-cut',
  recordingId: 'synthetic-recording',
  reason: null,
  trackIds: ['video', 'audio', 'microphone'],
};
const lanes = [
  { id: 'video', heightPx: 144, label: 'Video' },
  { id: 'audio', heightPx: 96, label: 'Desktop audio' },
  { id: 'microphone', heightPx: 96, label: 'Microphone' },
];
test('connected lane masks form one selectable group and save both resized edges atomically', async () => {
  const saved = vi.fn();
  const selected = vi.fn();
  render(
    <GroupedRedactions
      redactions={[redaction]}
      lanes={lanes}
      selectedIds={new Set()}
      timelineWidthPx={600}
      totalDurationMs={6000}
      onSelect={selected}
      onSaved={saved}
    />,
  );
  const groups = screen.getAllByRole('button', { name: /redacted range$/i });
  expect(groups).toHaveLength(1);
  await fireEvent.click(groups[0]!);
  expect(selected).toHaveBeenCalledWith('synthetic-cut', false);
  await fireEvent.keyDown(
    screen.getByRole('button', { name: 'Resize redaction end' }),
    { key: 'ArrowRight' },
  );
  await waitFor(() =>
    expect(client.resizeLibraryRedaction).toHaveBeenCalledWith({
      recordingId: 'synthetic-recording',
      redactionId: 'synthetic-cut',
      startOffsetMs: 1000,
      endOffsetMs: 2100,
      expectedStartOffsetMs: 1000,
      expectedEndOffsetMs: 2000,
    }),
  );
  await waitFor(() => expect(saved).toHaveBeenCalledTimes(1));
});
test('a failed resize retains the original mask and exposes recovery', async () => {
  client.resizeLibraryRedaction.mockRejectedValue(
    new Error('Synthetic save failure'),
  );
  const saved = vi.fn();
  render(
    <GroupedRedactions
      redactions={[redaction]}
      lanes={lanes}
      selectedIds={new Set()}
      timelineWidthPx={600}
      totalDurationMs={6000}
      onSelect={vi.fn()}
      onSaved={saved}
    />,
  );
  await fireEvent.keyDown(
    screen.getByRole('button', { name: 'Resize redaction start' }),
    { key: 'ArrowRight' },
  );
  await screen.findByRole('alert');
  expect(saved).not.toHaveBeenCalled();
  expect(
    screen.getAllByRole('button', { name: /redacted range$/i }),
  ).toHaveLength(1);
});
