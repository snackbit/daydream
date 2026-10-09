import type { RecallStatus } from '@daydream/shared';
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { settleVisualTransitions } from '@/app/design-accessibility-audit';
import * as recall from '@/features/recall/recall-client';
import '@/index.css';
import { RecallReadiness } from './recall-readiness';

vi.mock('@/features/recall/recall-client');
const ready: RecallStatus = {
  privacyRevision: 0,
  indexedEvidenceCount: 0,
  pendingEvidenceCount: 49,
  embeddingReady: true,
  generationReady: false,
  embeddingUnavailableReason: null,
  generationUnavailableReason: 'recall_generation_disabled',
  jobs: [],
};
beforeEach(async () => {
  await page.viewport(1440, 960);
  vi.mocked(recall.getRecallStatus).mockReset().mockResolvedValue(ready);
  vi.mocked(recall.listenToRecallStateChanged).mockResolvedValue(
    () => undefined,
  );
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  delete document.documentElement.dataset.theme;
});

test('readiness refresh shows progress and confirms an unchanged result without duplicate checks', async () => {
  const now = vi.spyOn(Date, 'now').mockReturnValue(1_800_000_000_000);
  render(<RecallReadiness />);
  await screen.findByText('Embeddings: Ready');
  const previous = screen.getByRole('status', {
    name: 'Readiness check',
  }).textContent;
  let finish!: (value: RecallStatus) => void;
  vi.mocked(recall.getRecallStatus).mockImplementationOnce(
    () =>
      new Promise<RecallStatus>((resolve) => {
        finish = resolve;
      }),
  );
  const calls = vi.mocked(recall.getRecallStatus).mock.calls.length;
  await userEvent.click(
    screen.getByRole('button', { name: 'Check readiness' }),
  );
  const checking = await screen.findByRole('button', { name: 'Checking…' });
  expect(checking).toBeDisabled();
  fireEvent.click(checking);
  expect(screen.getByText('Embeddings: Ready')).toBeTruthy();
  await waitFor(() =>
    expect(recall.getRecallStatus).toHaveBeenCalledTimes(calls + 1),
  );
  now.mockReturnValue(1_800_000_005_000);
  await act(async () => finish(ready));
  await waitFor(() =>
    expect(
      screen.getByRole('button', { name: 'Check readiness' }),
    ).toBeEnabled(),
  );
  expect(
    screen.getByRole('status', { name: 'Readiness check' }).textContent,
  ).not.toBe(previous);
  expect(screen.getByText('Embeddings: Ready')).toBeTruthy();
  for (const theme of ['light', 'dark']) {
    document.documentElement.dataset.theme = theme;
    for (const width of [1440, 390]) {
      await page.viewport(width, 960);
      await expect
        .element(page.getByRole('button', { name: 'Check readiness' }))
        .toBeVisible();
      await settleVisualTransitions();
      await page.screenshot({
        element: screen.getByRole('region', { name: 'Recall readiness' }),
        path: `__screenshots__/readiness-${theme}-${width}.png`,
      });
    }
  }
});

test('failed readiness refresh has a visible error and can recover on the next check', async () => {
  render(<RecallReadiness />);
  await screen.findByText('Embeddings: Ready');
  vi.mocked(recall.getRecallStatus).mockRejectedValueOnce(
    new Error('Synthetic native failure'),
  );
  await userEvent.click(
    screen.getByRole('button', { name: 'Check readiness' }),
  );
  await screen.findByRole('alert');
  expect(screen.queryByRole('status', { name: 'Readiness check' })).toBeNull();
  await userEvent.click(
    screen.getByRole('button', { name: 'Check readiness' }),
  );
  await screen.findByText('Embeddings: Ready');
  expect(screen.queryByRole('alert')).toBeNull();
  expect(screen.getByRole('status', { name: 'Readiness check' })).toBeTruthy();
});
