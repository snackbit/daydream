import type { ComponentProps } from 'react';
import type { RecallDay, RecallJob, RecallStatus } from '@daydream/shared';
import {
  act,
  cleanup,
  render,
  screen,
  waitFor,
  within,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import '@/index.css';
import {
  recallDayFixture,
  recallEvidenceFixture,
  recallStatusFixture,
} from '../testing/recall-browser-fixtures';
import { DayReviewPanel } from './day-review-panel';

const native = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  changed: null as ((event: { payload: unknown }) => void) | null,
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: native.invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: native.listen }));
vi.mock('@tanstack/react-router', () => ({
  Link: ({
    children,
    to,
    search,
    ...props
  }: ComponentProps<'a'> & { to: string; search?: unknown }) => (
    <a {...props} href={to} data-search={JSON.stringify(search)}>
      {children}
    </a>
  ),
}));

let day: RecallDay;
let status: RecallStatus;
const queued: RecallJob = {
  id: 'summary-1',
  dayId: '2026-10-05',
  kind: 'generation',
  status: 'pending',
  errorCode: null,
  progress: 0,
  updatedAtMs: 1,
};
function changeJob(
  jobStatus: RecallJob['status'],
  errorCode: string | null = null,
) {
  status.jobs = [{ ...queued, status: jobStatus, errorCode, updatedAtMs: 2 }];
  act(() =>
    native.changed?.({
      payload: {
        dayId: day.day.dayId,
        privacyRevision: status.privacyRevision,
      },
    }),
  );
}
function deferred<T>() {
  let resolve: (value: T) => void = () => {
    throw new Error('Uninitialized request');
  };
  let reject: (error: unknown) => void = () => {
    throw new Error('Uninitialized request');
  };
  const promise = new Promise<T>((r, j) => {
    resolve = r;
    reject = j;
  });
  return { promise, resolve, reject };
}
beforeEach(async () => {
  await page.viewport(1440, 960);
  vi.resetAllMocks();
  day = structuredClone(recallDayFixture);
  status = structuredClone(recallStatusFixture);
  native.listen.mockImplementation(
    async (_event: string, handler: (event: { payload: unknown }) => void) => {
      native.changed = handler;
      return () => {
        if (native.changed === handler) native.changed = null;
      };
    },
  );
  native.invoke.mockImplementation(async (command: string) => {
    if (command === 'getRecallDay') return structuredClone(day);
    if (command === 'getRecallStatus') return structuredClone(status);
    if (command === 'regenerateRecallDay' || command === 'controlRecallJob') {
      status.jobs = [queued];
      return queued;
    }
    if (command === 'reportFrontendFailure') return undefined;
    throw new Error(`Unexpected synthetic command: ${command}`);
  });
});
afterEach(() => {
  cleanup();
  delete document.documentElement.dataset.theme;
});

test('generation stays visible from click through queue and worker completion without blocking review content', async () => {
  const pending = deferred<RecallJob>();
  const invoke = native.invoke.getMockImplementation();
  native.invoke.mockImplementation((command: string) =>
    command === 'regenerateRecallDay' ? pending.promise : invoke?.(command),
  );
  render(<DayReviewPanel date="2026-10-05" />);
  const button = await screen.findByRole('button', {
    name: 'Generate summary',
  });
  await userEvent.click(button);
  expect(button).toBeDisabled();
  expect(button).toHaveAttribute('aria-busy', 'true');
  expect(screen.getByText('Starting summary generation…')).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Mark reviewed' })).toBeEnabled();
  expect(
    native.invoke.mock.calls.filter(
      ([command]) => command === 'regenerateRecallDay',
    ),
  ).toHaveLength(1);
  status.jobs = [queued];
  await act(async () => pending.resolve(queued));
  await screen.findByText('Summary queued.');
  changeJob('running');
  await screen.findByText('Generating summary…', {}, { timeout: 2500 });
  expect(button).toBeDisabled();
  day.day.summary = 'Synthetic completed day summary';
  changeJob('succeeded');
  await screen.findByText(
    'Synthetic completed day summary',
    {},
    { timeout: 2500 },
  );
  await screen.findByText('Summary generation completed.');
  expect(button).toBeEnabled();
  expect(button).toHaveAttribute('aria-busy', 'false');
});

test('an enqueue failure stays beside the action through refresh, then a user retry recovers', async () => {
  const invoke = native.invoke.getMockImplementation();
  let fail = true;
  native.invoke.mockImplementation((command: string) => {
    if (command === 'regenerateRecallDay' && fail)
      return Promise.reject({
        code: 'recall_provider_auth_failed',
        message: 'Synthetic private provider body',
      });
    return invoke?.(command);
  });
  render(<DayReviewPanel date="2026-10-05" />);
  await userEvent.click(
    await screen.findByRole('button', { name: 'Generate summary' }),
  );
  const alert = await screen.findByRole('alert');
  expect(alert.textContent).toContain('authenticate');
  expect(screen.getByRole('link', { name: 'Summary settings' })).toBeTruthy();
  expect(screen.queryByText('Synthetic private provider body')).toBeNull();
  const calls = native.invoke.mock.calls.length;
  act(() => native.changed?.({ payload: { dayId: null, privacyRevision: 1 } }));
  await waitFor(
    () => expect(native.invoke.mock.calls.length).toBeGreaterThan(calls),
    { timeout: 2500 },
  );
  expect(screen.getByRole('alert').textContent).toContain('authenticate');
  fail = false;
  await userEvent.click(
    screen.getByRole('button', { name: 'Generate summary' }),
  );
  await screen.findByText('Summary queued.');
  expect(screen.queryByRole('alert')).toBeNull();
});

test.each(['pending', 'running', 'failed', 'cancelled', 'succeeded'] as const)(
  'reopened review shows persisted %s work for its day only',
  async (jobStatus) => {
    status.jobs = [
      {
        ...queued,
        id: 'other-day',
        dayId: '2026-10-04',
        status: 'running',
        updatedAtMs: 10,
      },
      {
        ...queued,
        status: jobStatus,
        errorCode: jobStatus === 'failed' ? 'recall_provider_timeout' : null,
      },
    ];
    const view = render(<DayReviewPanel date="2026-10-05" />);
    const button = await screen.findByRole('button', {
      name: 'Generate summary',
    });
    if (jobStatus === 'failed')
      expect((await screen.findByRole('alert')).textContent).toContain(
        'unavailable',
      );
    else
      expect(screen.getByRole('status').textContent).toContain(
        {
          pending: 'queued',
          running: 'Generating',
          cancelled: 'cancelled',
          succeeded: 'completed',
        }[jobStatus],
      );
    expect(button.hasAttribute('disabled')).toBe(
      jobStatus === 'pending' || jobStatus === 'running',
    );
    view.unmount();
    render(<DayReviewPanel date="2026-10-05" />);
    expect(
      (
        await screen.findByRole('button', { name: 'Generate summary' })
      ).hasAttribute('disabled'),
    ).toBe(jobStatus === 'pending' || jobStatus === 'running');
  },
);

test('paused summary work resumes through the persisted job control', async () => {
  status.jobs = [{ ...queued, status: 'paused' }];
  render(<DayReviewPanel date="2026-10-05" />);
  await screen.findByText('Summary generation paused.');
  await userEvent.click(screen.getByRole('button', { name: 'Resume summary' }));
  await screen.findByText('Summary queued.');
  expect(native.invoke).toHaveBeenCalledWith('controlRecallJob', {
    input: { jobId: queued.id, action: 'resume' },
  });
});

test('late generation responses cannot update a different date or a closed review', async () => {
  const pending = deferred<RecallJob>();
  const invoke = native.invoke.getMockImplementation();
  native.invoke.mockImplementation((command: string) =>
    command === 'regenerateRecallDay' ? pending.promise : invoke?.(command),
  );
  const view = render(<DayReviewPanel date="2026-10-05" />);
  await userEvent.click(
    await screen.findByRole('button', { name: 'Generate summary' }),
  );
  view.rerender(<DayReviewPanel date="2026-10-06" />);
  await screen.findByRole('button', { name: 'Generate summary' });
  await act(async () => pending.resolve(queued));
  expect(screen.queryByText('Summary queued.')).toBeNull();
  expect(
    screen.getByRole('button', { name: 'Generate summary' }),
  ).toBeEnabled();
  view.unmount();
});

test('provider interruption remains visibly queued and a later generation replaces completed work', async () => {
  render(<DayReviewPanel date="2026-10-05" />);
  await userEvent.click(
    await screen.findByRole('button', { name: 'Generate summary' }),
  );
  await screen.findByText('Summary queued.');
  changeJob('pending', 'recall_provider_timeout');
  await screen.findByText(/queued to retry/, {}, { timeout: 2500 });
  expect(
    screen.getByRole('button', { name: 'Generate summary' }),
  ).toBeDisabled();
  changeJob('succeeded');
  await screen.findByText(
    'Summary generation completed.',
    {},
    { timeout: 2500 },
  );
  status.jobs = [
    ...status.jobs,
    { ...queued, id: 'summary-2', status: 'running', updatedAtMs: 3 },
  ];
  act(() =>
    native.changed?.({ payload: { dayId: day.day.dayId, privacyRevision: 1 } }),
  );
  await screen.findByText('Generating summary…', {}, { timeout: 2500 });
  expect(
    screen.getByRole('button', { name: 'Generate summary' }),
  ).toBeDisabled();
});

test('summary read failure offers recovery and privacy invalidation clears the old summary', async () => {
  const invoke = native.invoke.getMockImplementation();
  let failRead = false;
  native.invoke.mockImplementation((command: string) =>
    command === 'getRecallStatus' && failRead
      ? Promise.reject(new Error('Synthetic status unavailable'))
      : invoke?.(command),
  );
  render(<DayReviewPanel date="2026-10-05" />);
  await screen.findByText(day.day.summary ?? '');
  failRead = true;
  act(() =>
    native.changed?.({ payload: { dayId: day.day.dayId, privacyRevision: 1 } }),
  );
  await screen.findByRole('alert', {}, { timeout: 2500 });
  expect(screen.queryByText(day.day.summary ?? '')).toBeNull();
  failRead = false;
  await userEvent.click(screen.getByRole('button', { name: 'Try again' }));
  await screen.findByText(day.day.summary ?? '');
  day.privacyRevision = 2;
  day.day.privacyRevision = 2;
  day.day.status = 'invalidated';
  status.privacyRevision = 2;
  act(() =>
    native.changed?.({ payload: { dayId: day.day.dayId, privacyRevision: 2 } }),
  );
  expect(screen.queryByText(day.day.summary ?? '')).toBeNull();
  await screen.findByText(/interpretation needs an update/);
  expect(screen.queryByText(day.day.summary ?? '')).toBeNull();
});

test.each(['disabled', 'unconfigured', 'failed', 'empty'] as const)(
  'summary %s keeps its blocking or recovery condition visible',
  async (condition) => {
    day.day.summary = null;
    if (condition === 'disabled' || condition === 'unconfigured') {
      status.generationReady = false;
      status.generationUnavailableReason =
        condition === 'disabled'
          ? 'recall_generation_disabled'
          : 'recall_profile_invalid';
    } else if (condition === 'failed') day.day.status = 'failed';
    else day.day.evidenceCount = 0;
    render(<DayReviewPanel date="2026-10-05" />);
    await screen.findByText(
      {
        disabled: /Summaries off/,
        unconfigured: /Summary setup incomplete/,
        failed: /Summary failed/,
        empty: /Recorded content is not available/,
      }[condition],
    );
    if (condition === 'failed')
      expect(
        screen.getByRole('button', { name: 'Generate summary' }),
      ).toBeEnabled();
    else
      expect(
        screen.queryByRole('button', { name: 'Generate summary' }),
      ).toBeNull();
  },
);

test.each(['protected', 'empty'] as const)(
  'coverage %s preserves source protection and unknown timing',
  async (condition) => {
    if (condition === 'empty') day.coverage = [];
    else {
      const captured = day.coverage[0];
      if (!captured) throw new Error('Missing synthetic capture');
      day.recordings = [
        {
          recordingId: 'recording-1',
          title: 'Synthetic recording',
          startMs: captured.startMs,
          endMs: null,
          durationMs: null,
          startOffsetMs: 0,
          availability: 'partial',
          processingStatus: null,
        },
        {
          recordingId: 'other',
          title: null,
          startMs: captured.startMs,
          endMs: null,
          durationMs: null,
          startOffsetMs: null,
          availability: 'available',
          processingStatus: null,
        },
      ];
      day.coverage = [
        captured,
        {
          ...captured,
          recordingId: 'other',
          kind: 'redacted',
          sourceTrackIds: ['microphone'],
        },
        { ...captured, kind: 'redacted', sourceTrackIds: [] },
      ];
    }
    render(<DayReviewPanel date="2026-10-05" />);
    if (condition === 'empty') await screen.findByText(/No capture coverage/);
    else {
      await userEvent.click(
        await screen.findByRole('button', {
          name: 'Inspect coverage intervals',
        }),
      );
      const row = screen.getByRole('listitem');
      expect(row.textContent).toContain('Synthetic recording');
      expect(row.textContent).toContain('all sources excluded');
      expect(row.textContent).toContain('microphone');
      expect(
        screen.getByText(/Protected sources shown separately/),
      ).toBeTruthy();
      expect(screen.getByText('Timing unavailable')).toBeTruthy();
    }
  },
);

test('coverage merges segment pairs and Day review never exposes raw evidence', async () => {
  day.evidence = [recallEvidenceFixture];
  day.coverage = [0, 15000].flatMap((startMs) =>
    ['captured', 'uninterpreted'].map((kind) => ({
      recordingId: 'recording-1',
      segmentId: `chunk-${startMs}`,
      startMs: day.day.startMs + startMs,
      endMs: day.day.startMs + startMs + 15000,
      kind:
        kind === 'captured'
          ? ('captured' as const)
          : ('uninterpreted' as const),
      sourceTrackIds: [],
    })),
  );
  render(<DayReviewPanel date="2026-10-05" />);
  await userEvent.click(
    await screen.findByRole('button', { name: 'Inspect coverage intervals' }),
  );
  const rows = within(screen.getByRole('list')).getAllByRole('listitem');
  expect(rows).toHaveLength(1);
  expect(rows[0]?.textContent).toContain('Captured · Not interpreted yet');
  expect(screen.queryByText(recallEvidenceFixture.text)).toBeNull();
  expect(
    screen.queryByRole('button', { name: 'Recorded text and transcripts' }),
  ).toBeNull();
  await screen.findByRole('heading', { name: 'Sessions' });
});
