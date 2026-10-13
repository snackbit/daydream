import { useState, type ComponentProps } from 'react';
import type { RecallDay, RecallTask } from '@daydream/shared';
import {
  cleanup,
  render,
  screen,
  waitFor,
  act,
  within,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { DayReviewPanel as DayScreen } from '@/features/recall/day/day-review-panel';
import type { RecallSearchLocation } from '@/features/recall/search/search-controls';
import {
  recallDayFixture,
  recallEvidenceFixture,
  recallSearchFixture,
  recallStatusFixture,
} from '@/features/recall/testing/recall-browser-fixtures';
import '../index.css';
import { SearchScreen } from './SearchScreen';

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
    params,
    search,
    ...anchorProps
  }: {
    to: string;
    params?: { recordingId?: string };
    search?: { evidenceId?: string };
  } & ComponentProps<'a'>) => (
    <a
      {...anchorProps}
      href={
        params?.recordingId
          ? `/library/${params.recordingId}?evidenceId=${search?.evidenceId ?? ''}`
          : to
      }
    >
      {children}
    </a>
  ),
}));

let savedDay: RecallDay;
beforeEach(async () => {
  await page.viewport(1440, 960);
  vi.resetAllMocks();
  savedDay = structuredClone(recallDayFixture);
  native.listen.mockImplementation(
    async (_name: string, handler: (event: { payload: unknown }) => void) => {
      native.changed = handler;
      return () => {
        if (native.changed === handler) native.changed = null;
      };
    },
  );
  native.invoke.mockImplementation(
    async (command: string, args?: { input?: unknown }) => {
      if (command === 'getRecallDay') return savedDay;
      if (command === 'getRecallStatus') return recallStatusFixture;
      if (command === 'cancelRecallSearch') return undefined;
      if (command === 'searchRecallEvidence') return recallSearchFixture;
      if (command === 'setRecallDayReviewed') {
        savedDay = { ...savedDay, day: { ...savedDay.day, reviewed: true } };
        return savedDay;
      }
      if (command === 'acceptRecallSuggestion') {
        const task: RecallTask = {
          id: 'task-1',
          dayId: savedDay.day.dayId,
          title: 'Review focus behavior',
          notes: 'Synthetic task',
          status: 'open',
          citations: [recallEvidenceFixture],
          sourceWithheld: false,
          updatedAtMs: 1,
        };
        savedDay = {
          ...savedDay,
          tasks: [task],
          suggestions: savedDay.suggestions.map((suggestion) => ({
            ...suggestion,
            acceptedTaskId: task.id,
          })),
        };
        return task;
      }
      if (command === 'dismissRecallSuggestion') {
        savedDay = {
          ...savedDay,
          suggestions: savedDay.suggestions.map((suggestion) => ({
            ...suggestion,
            dismissed: true,
          })),
        };
        return savedDay;
      }
      if (command === 'updateRecallTask') {
        const input = args?.input;
        if (typeof input !== 'object' || input === null || !('status' in input))
          throw new Error('Invalid synthetic request');
        const status = input.status;
        if (status !== 'open' && status !== 'done' && status !== 'dismissed')
          throw new Error('Invalid synthetic status');
        const task = savedDay.tasks[0];
        if (!task) throw new Error('Missing synthetic task');
        const updated: RecallTask = { ...task, status };
        savedDay = { ...savedDay, tasks: [updated] };
        return updated;
      }
      throw new Error('Unexpected synthetic command');
    },
  );
});
afterEach(() => {
  cleanup();
  delete document.documentElement.dataset.theme;
});

test('Day sidebar retains coverage without summaries, sessions or feedback setup', async () => {
  savedDay = {
    ...savedDay,
    day: {
      ...savedDay.day,
      summary: null,
      status: 'unconfigured',
      capturedDurationMs: 17_000,
      recordingCount: 2,
    },
    sessions: [],
    suggestions: [],
    tasks: [],
    recordings: [8000, 9000].map((durationMs, index) => ({
      recordingId: `recording-${index}`,
      title: `Synthetic clip ${index}`,
      startMs: savedDay.day.startMs + index * 10_000,
      endMs: savedDay.day.startMs + index * 10_000 + durationMs,
      startOffsetMs: index * 1000,
      durationMs,
      availability: 'available',
      processingStatus: index === 0 ? 'failed' : null,
    })),
  };
  native.invoke.mockImplementation(async (command: string) => {
    if (command === 'getRecallDay') return savedDay;
    if (command === 'getRecallStatus')
      return {
        ...recallStatusFixture,
        generationReady: false,
        generationUnavailableReason: 'recall_generation_disabled',
      };
    throw new Error('Unexpected synthetic command');
  });
  render(<DayScreen date="2026-10-05" />);
  await screen.findByText('17 seconds');
  expect(screen.queryByRole('link', { name: 'Open recording' })).toBeNull();
  expect(screen.queryByRole('button', { name: 'Save feedback' })).toBeNull();
  expect(
    screen.queryByRole('heading', { name: 'Text and transcripts' }),
  ).toBeNull();
  expect(screen.queryByRole('heading', { name: 'Tasks' })).toBeNull();
});

test('Day review accepts a local task, marks it done, and records the reviewed fingerprint', async () => {
  render(<DayScreen date="2026-10-05" />);
  await screen.findByText(recallDayFixture.day.summary ?? '');
  const source = screen.getAllByRole('link', { name: /screen text/i })[0];
  expect(source?.getAttribute('href')).toContain('evidenceId=evidence-1');
  await userEvent.click(
    screen.getByRole('button', { name: 'Accept as local task' }),
  );
  await userEvent.click(
    await screen.findByRole('button', { name: 'Mark done' }),
  );
  await screen.findByText('done');
  await userEvent.click(screen.getByRole('button', { name: 'Mark reviewed' }));
  await screen.findByRole('button', { name: 'Mark unreviewed' });
  expect(native.invoke).toHaveBeenCalledWith('setRecallDayReviewed', {
    input: {
      dayId: '2026-10-05',
      reviewed: true,
      inputFingerprint: 'fingerprint-1',
    },
  });
});

function SearchHarness() {
  const [location, setLocation] = useState<RecallSearchLocation>({
    q: '',
    source: 'all',
    date: null,
  });
  return (
    <SearchScreen
      query={location.q}
      source={location.source}
      date={location.date}
      onSearchChange={setLocation}
    />
  );
}
test('Search submits typed words and literal mode, exposes source links, and pages native results', async () => {
  render(<SearchHarness />);
  expect(native.invoke).not.toHaveBeenCalled();
  await userEvent.fill(
    screen.getByRole('textbox', { name: 'Find a moment' }),
    'focus',
  );
  await userEvent.click(screen.getByRole('combobox', { name: 'Match' }));
  await userEvent.click(screen.getByRole('option', { name: 'Literal words' }));
  await waitFor(() => expect(screen.queryByRole('listbox')).toBeNull());
  await userEvent.click(screen.getByRole('textbox', { name: 'Find a moment' }));
  await userEvent.keyboard('{Enter}');
  await screen.findByRole('link', { name: /screen text/i });
  expect(screen.queryByText(recallEvidenceFixture.text)).toBeNull();
  expect(native.invoke).toHaveBeenCalledWith('searchRecallEvidence', {
    input: expect.objectContaining({
      query: 'focus',
      mode: 'lexical',
      sourceKinds: [],
      cursor: null,
    }),
  });
  expect(
    screen.getByRole('link', { name: /screen text/i }).getAttribute('href'),
  ).toContain('evidenceId=evidence-1');
});

test('audio search retains transcript excerpts while screen matches keep source links without OCR', async () => {
  native.invoke.mockImplementation(async (command: string) => {
    if (command === 'getRecallStatus') return recallStatusFixture;
    if (command === 'cancelRecallSearch') return undefined;
    if (command === 'searchRecallEvidence')
      return {
        ...recallSearchFixture,
        hits: [
          { evidence: recallEvidenceFixture, score: 1 },
          {
            evidence: {
              ...recallEvidenceFixture,
              evidenceId: 'audio-evidence',
              sourceKind: 'microphone',
              text: 'Synthetic spoken excerpt',
            },
            score: 0.9,
          },
        ],
      };
    throw new Error('Unexpected synthetic command');
  });
  render(
    <SearchScreen
      query="focus"
      source="all"
      date={null}
      onSearchChange={vi.fn()}
    />,
  );
  await screen.findByText('Synthetic spoken excerpt');
  expect(screen.getByRole('link', { name: /screen text/i })).toBeTruthy();
  expect(screen.getByRole('link', { name: /microphone/i })).toBeTruthy();
  expect(screen.queryByText(recallEvidenceFixture.text)).toBeNull();
});

test('A privacy change clears private results immediately and rejects a late search response', async () => {
  let resolveLate: ((result: typeof recallSearchFixture) => void) | undefined;
  let calls = 0;
  native.invoke.mockImplementation((command: string) => {
    if (command === 'cancelRecallSearch') return Promise.resolve(undefined);
    if (command === 'getRecallStatus')
      return Promise.resolve(recallStatusFixture);
    if (command !== 'searchRecallEvidence')
      throw new Error('Unexpected synthetic command');
    calls += 1;
    if (calls === 1) return Promise.resolve(recallSearchFixture);
    return new Promise<typeof recallSearchFixture>((resolve) => {
      resolveLate = resolve;
    });
  });
  render(
    <SearchScreen
      query="focus"
      source="all"
      date={null}
      onSearchChange={vi.fn()}
    />,
  );
  await screen.findByRole('link', { name: /screen text/i });
  expect(screen.queryByText(recallEvidenceFixture.text)).toBeNull();
  act(() => {
    native.changed?.({ payload: { dayId: null, privacyRevision: 2 } });
  });
  expect(screen.queryByRole('link', { name: /screen text/i })).toBeNull();
  await waitFor(() => expect(calls).toBe(2));
  act(() => {
    native.changed?.({ payload: { dayId: null, privacyRevision: 3 } });
    resolveLate?.(recallSearchFixture);
  });
  await waitFor(() => expect(calls).toBe(3));
  expect(screen.queryByRole('link', { name: /screen text/i })).toBeNull();
});

test('Malformed eligibility metadata withholds private results until explicit recovery', async () => {
  render(
    <SearchScreen
      query="focus"
      source="all"
      date={null}
      onSearchChange={vi.fn()}
    />,
  );
  await screen.findByRole('link', { name: /screen text/i });
  expect(screen.queryByText(recallEvidenceFixture.text)).toBeNull();
  act(() => {
    native.changed?.({ payload: { dayId: null, privacyRevision: 'invalid' } });
  });
  await screen.findByRole('alert');
  expect(screen.queryByRole('link', { name: /screen text/i })).toBeNull();
  expect(
    native.invoke.mock.calls.filter(
      ([command]) => command === 'searchRecallEvidence',
    ),
  ).toHaveLength(1);
  await userEvent.click(screen.getByRole('button', { name: 'Try again' }));
  await screen.findByRole('link', { name: /screen text/i });
  expect(screen.queryByText(recallEvidenceFixture.text)).toBeNull();
});

test('Progress bursts preserve an in-flight search and coalesce one trailing refresh while privacy still cancels immediately', async () => {
  const complete: ((result: typeof recallSearchFixture) => void)[] = [];
  native.invoke.mockImplementation((command: string) => {
    if (command === 'getRecallStatus')
      return Promise.resolve(recallStatusFixture);
    if (command === 'cancelRecallSearch') return Promise.resolve(undefined);
    if (command !== 'searchRecallEvidence')
      throw new Error('Unexpected synthetic command');
    return new Promise<typeof recallSearchFixture>((resolve) => {
      complete.push(resolve);
    });
  });
  render(
    <SearchScreen
      query="focus"
      source="all"
      date={null}
      onSearchChange={vi.fn()}
    />,
  );
  await waitFor(() => expect(complete).toHaveLength(1));
  act(() => {
    for (let count = 0; count < 25; count += 1)
      native.changed?.({ payload: { dayId: null, privacyRevision: 1 } });
  });
  await new Promise<void>((resolve) => setTimeout(resolve, 900));
  expect(complete).toHaveLength(1);
  expect(native.invoke).not.toHaveBeenCalledWith('cancelRecallSearch');
  await act(async () => {
    complete[0]?.(recallSearchFixture);
  });
  await screen.findByRole('link', { name: /screen text/i });
  expect(screen.queryByText(recallEvidenceFixture.text)).toBeNull();
  await waitFor(() => expect(complete).toHaveLength(2));
  expect(native.invoke).not.toHaveBeenCalledWith('cancelRecallSearch');
  act(() => {
    native.changed?.({ payload: { dayId: null, privacyRevision: 2 } });
  });
  expect(screen.queryByRole('link', { name: /screen text/i })).toBeNull();
  expect(native.invoke).toHaveBeenCalledWith('cancelRecallSearch');
  await act(async () => {
    complete[1]?.(recallSearchFixture);
  });
  await waitFor(() => expect(complete).toHaveLength(3));
  expect(screen.queryByRole('link', { name: /screen text/i })).toBeNull();
});

test('A changed query cancels its previous scope independently of progress refreshes', async () => {
  const complete: ((result: typeof recallSearchFixture) => void)[] = [];
  native.invoke.mockImplementation((command: string) => {
    if (command === 'getRecallStatus')
      return Promise.resolve(recallStatusFixture);
    if (command === 'cancelRecallSearch') return Promise.resolve(undefined);
    if (command !== 'searchRecallEvidence')
      throw new Error('Unexpected synthetic command');
    return new Promise<typeof recallSearchFixture>((resolve) => {
      complete.push(resolve);
    });
  });
  const view = render(
    <SearchScreen
      query="old query"
      source="all"
      date={null}
      onSearchChange={vi.fn()}
    />,
  );
  await waitFor(() => expect(complete).toHaveLength(1));
  view.rerender(
    <SearchScreen
      query="new query"
      source="ocr"
      date="2026-10-05"
      onSearchChange={vi.fn()}
    />,
  );
  await waitFor(() => expect(complete).toHaveLength(2));
  expect(
    native.invoke.mock.calls.filter(
      ([command]) => command === 'cancelRecallSearch',
    ),
  ).toHaveLength(1);
  await act(async () => {
    complete[0]?.(recallSearchFixture);
  });
  expect(screen.queryByRole('link', { name: /screen text/i })).toBeNull();
  await act(async () => {
    complete[1]?.(recallSearchFixture);
  });
  await screen.findByRole('link', { name: /screen text/i });
  expect(screen.queryByText(recallEvidenceFixture.text)).toBeNull();
  expect(native.invoke).toHaveBeenCalledWith('searchRecallEvidence', {
    input: expect.objectContaining({
      query: 'new query',
      sourceKinds: ['ocr'],
      dayId: '2026-10-05',
    }),
  });
});

test('Hybrid search preserves paraphrased evidence and sends source, day, and cursor filters', async () => {
  native.invoke.mockImplementation(
    async (command: string, args?: { input?: unknown }) => {
      if (command === 'cancelRecallSearch') return undefined;
      if (command === 'getRecallStatus') return recallStatusFixture;
      if (command !== 'searchRecallEvidence')
        throw new Error('Unexpected synthetic command');
      const input = args?.input;
      const nextPage =
        typeof input === 'object' &&
        input !== null &&
        'cursor' in input &&
        input.cursor === 'cursor-2';
      return nextPage
        ? { ...recallSearchFixture, hits: [], nextCursor: null }
        : { ...recallSearchFixture, nextCursor: 'cursor-2' };
    },
  );
  render(<SearchHarness />);
  await userEvent.fill(
    screen.getByRole('textbox', { name: 'Find a moment' }),
    'navigate without a mouse',
  );
  await userEvent.fill(screen.getByLabelText('Day'), '2026-10-05');
  await userEvent.click(screen.getByRole('combobox', { name: 'Source' }));
  await userEvent.click(screen.getByRole('option', { name: 'Screen text' }));
  await waitFor(() => expect(screen.queryByRole('listbox')).toBeNull());
  await userEvent.click(screen.getByRole('button', { name: 'Search' }));
  await screen.findByRole('link', { name: /screen text/i });
  expect(screen.queryByText(recallEvidenceFixture.text)).toBeNull();
  expect(
    within(screen.getByRole('region', { name: 'Search results' })).getByText(
      'Semantic + literal',
    ),
  ).toBeTruthy();
  expect(screen.queryByText('navigate without a mouse')).toBeNull();
  expect(native.invoke).toHaveBeenCalledWith('searchRecallEvidence', {
    input: expect.objectContaining({
      query: 'navigate without a mouse',
      dayId: '2026-10-05',
      sourceKinds: ['ocr'],
      mode: 'hybrid',
    }),
  });
  await userEvent.click(screen.getByRole('button', { name: 'Next results' }));
  await screen.findByText('Page 2');
  await screen.findByText('No eligible matches found.');
  expect(native.invoke).toHaveBeenCalledWith('searchRecallEvidence', {
    input: expect.objectContaining({
      cursor: 'cursor-2',
      sourceKinds: ['ocr'],
      dayId: '2026-10-05',
    }),
  });
  await userEvent.click(
    screen.getByRole('button', { name: 'Previous results' }),
  );
  await screen.findByRole('link', { name: /screen text/i });
  expect(screen.queryByText(recallEvidenceFixture.text)).toBeNull();
});

test('Day retains review controls without feedback forms and invalidated days withhold accepted task text', async () => {
  native.invoke.mockImplementation(async (command: string) => {
    if (command === 'getRecallDay') return savedDay;
    if (command === 'getRecallStatus') return recallStatusFixture;
    if (command === 'setRecallFeedback')
      throw new Error('Synthetic local save failure');
    throw new Error('Unexpected synthetic command');
  });
  const view = render(<DayScreen date="2026-10-05" />);
  await screen.findByText(recallDayFixture.day.summary ?? '');
  expect(
    screen.queryByRole('textbox', { name: 'Feedback or correction' }),
  ).toBeNull();
  expect(screen.queryByRole('button', { name: 'Save feedback' })).toBeNull();
  expect(screen.getByRole('button', { name: 'Mark reviewed' })).toBeEnabled();
  act(() => {
    native.changed?.({ payload: { dayId: '2026-10-05', privacyRevision: 1 } });
  });
  await waitFor(() =>
    expect(
      native.invoke.mock.calls.filter(
        ([command]) => command === 'getRecallDay',
      ),
    ).toHaveLength(2),
  );
  view.unmount();
  savedDay = {
    ...savedDay,
    day: { ...savedDay.day, status: 'invalidated' },
    tasks: [
      {
        id: 'task-private',
        dayId: '2026-10-05',
        title: 'Synthetic private derived task',
        notes: 'Synthetic private notes',
        status: 'open',
        citations: [recallEvidenceFixture],
        sourceWithheld: false,
        updatedAtMs: 1,
      },
    ],
  };
  render(<DayScreen date="2026-10-05" />);
  await screen.findByText(/source content withheld/i);
  expect(screen.queryByText('Synthetic private derived task')).toBeNull();
  expect(screen.queryByText(recallDayFixture.day.summary ?? '')).toBeNull();
  expect(
    screen.queryByText(recallDayFixture.sessions[0]?.summary ?? ''),
  ).toBeNull();
});

test('Dismissed suggestions stay dismissed without creating a task', async () => {
  render(<DayScreen date="2026-10-05" />);
  await screen.findByRole('button', { name: 'Accept as local task' });
  await userEvent.click(screen.getByRole('button', { name: 'Dismiss' }));
  await waitFor(() =>
    expect(
      screen.queryByRole('button', { name: 'Accept as local task' }),
    ).toBeNull(),
  );
  expect(native.invoke).toHaveBeenCalledWith('dismissRecallSuggestion', {
    input: { dayId: '2026-10-05', suggestionId: 'suggestion-1' },
  });
  expect(savedDay.tasks).toHaveLength(0);
});

test('Search retries a failed read and reports incomplete literal coverage without inventing semantic matches', async () => {
  let attempts = 0;
  native.invoke.mockImplementation(async (command: string) => {
    if (command === 'cancelRecallSearch') return undefined;
    if (command === 'getRecallStatus') return recallStatusFixture;
    if (command !== 'searchRecallEvidence')
      throw new Error('Unexpected synthetic command');
    attempts += 1;
    if (attempts === 1) throw new Error('Synthetic unavailable index');
    return {
      ...recallSearchFixture,
      complete: false,
      semanticUsed: false,
      indexedEvidenceCount: 0,
      eligibleEvidenceCount: 1,
      unavailableReason: 'embedding_not_ready',
    };
  });
  render(
    <SearchScreen
      query="focus"
      source="all"
      date={null}
      onSearchChange={vi.fn()}
    />,
  );
  await screen.findByRole('alert');
  await userEvent.click(screen.getByRole('button', { name: 'Try again' }));
  await screen.findByRole('link', { name: /screen text/i });
  expect(screen.queryByText(recallEvidenceFixture.text)).toBeNull();
  expect(screen.getByText('Literal matches')).toBeTruthy();
  expect(
    within(screen.getByRole('region', { name: 'Search results' })).queryByText(
      'Semantic + literal',
    ),
  ).toBeNull();
  expect(
    screen
      .getByRole('link', { name: 'Inspect processing' })
      .getAttribute('href'),
  ).toBe('/processing');
  expect(
    screen
      .getByRole('link', { name: 'Review recall setup' })
      .getAttribute('href'),
  ).toBe('/settings');
});

test('Day and Search controls remain visible across both themes and narrow viewports', async () => {
  for (const dark of [false, true]) {
    document.documentElement.dataset.theme = dark ? 'dark' : 'light';
    for (const width of [1440, 1024, 768, 390, 320]) {
      await page.viewport(width, 960);
      render(<DayScreen date="2026-10-05" />);
      await screen.findByRole('button', { name: 'Mark reviewed' });
      await expect
        .element(
          page.getByRole('button', { name: 'Mark reviewed', exact: true }),
        )
        .toBeVisible();
      await page.screenshot({
        element: screen.getByRole('region', { name: 'Day overview' }),
      });
      await page.screenshot({ element: screen.getByRole('complementary') });
      cleanup();
      const search = render(<SearchHarness />);
      await expect
        .element(page.getByRole('textbox', { name: 'Find a moment' }))
        .toBeVisible();
      await page.screenshot({ element: search.container });
      cleanup();
    }
  }
});
