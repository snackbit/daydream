import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { userEvent } from 'vitest/browser';
import { Button } from '@/components/ui/button';
import type { InspectorContentEntry } from '../inspector/content/inspector-content';
import { PlaybackInspectorFeed } from '../inspector/content/playback-inspector-feed';
import { useStudioVideoSources } from '../playback/sources/use-studio-video-sources';
import { detail } from '../timeline/processing/processing-test-fixtures';
import { useStudioEntryNavigation } from './use-studio-entry-navigation';

afterEach(cleanup);
const note: InspectorContentEntry = {
  id: 'note',
  kind: 'observation',
  observationKind: 'screen-observation',
  trackId: 'screen-b',
  source: 'Second monitor',
  startMs: 2345,
  endMs: 2346,
  text: 'A synthetic update.',
  words: [],
  fragments: [],
  boundary: false,
};
const speech: InspectorContentEntry = {
  ...note,
  id: 'speech',
  kind: 'speech',
  trackId: 'mic',
  source: 'Microphone',
  startMs: 3456,
  endMs: 4456,
  text: 'A synthetic phrase.',
};

function Harness({
  ready,
  privacy = 'initial',
  seek,
}: {
  ready: boolean;
  privacy?: string;
  seek: (time: number, source: string | undefined) => void;
}) {
  const sources = useStudioVideoSources(detail, null);
  const navigate = useStudioEntryNavigation(
    detail.id,
    privacy,
    sources,
    ready,
    (time) => seek(time, sources.selectedTrackId),
  );
  return (
    <div style={{ height: 400, width: 390 }}>
      <output aria-label="Video source">{sources.selectedTrackId}</output>
      <Button onClick={() => sources.select('screen-a')}>First monitor</Button>
      <PlaybackInspectorFeed
        index={{
          entries: [note, speech],
          speechBySource: new Map(),
          focus: [],
        }}
        time={0}
        isPlaying={false}
        onSeek={(time) => seek(time, sources.selectedTrackId)}
        onNavigateEntry={navigate}
      />
    </div>
  );
}

test('annotation navigation selects its monitor before seeking; later audio navigation keeps that monitor', async () => {
  const seek = vi.fn();
  render(<Harness ready seek={seek} />);
  await userEvent.click(screen.getByText(note.text));
  expect(screen.getByLabelText('Video source').textContent).toBe('screen-b');
  expect(seek).toHaveBeenCalledExactlyOnceWith(2345, 'screen-b');
  await userEvent.click(screen.getByText(speech.text));
  expect(seek).toHaveBeenLastCalledWith(3456, 'screen-b');
  await userEvent.click(screen.getByRole('button', { name: 'First monitor' }));
  await userEvent.click(
    screen.getByRole('button', {
      name: 'Jump to AI observation from Second monitor at 0:02',
    }),
  );
  expect(seek).toHaveBeenLastCalledWith(2345, 'screen-b');
});

test('a pending monitor seek is discarded when privacy changes', async () => {
  const seek = vi.fn();
  const view = render(<Harness ready={false} seek={seek} />);
  await userEvent.click(screen.getByText(note.text));
  expect(seek).not.toHaveBeenCalled();
  view.rerender(<Harness ready privacy="redacted" seek={seek} />);
  expect(seek).not.toHaveBeenCalled();
  view.rerender(<Harness ready privacy="initial" seek={seek} />);
  expect(seek).not.toHaveBeenCalled();
});
