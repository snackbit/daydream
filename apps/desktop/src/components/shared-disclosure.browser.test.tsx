import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { userEvent } from 'vitest/browser';
import '@/index.css';
import { SharedDisclosure } from './shared-disclosure';

afterEach(cleanup);
test('disclosure retains visible status and supports keyboard details without hiding the next action', async () => {
  render(
    <>
      <SharedDisclosure label="Local diagnostics" status="Needs attention">
        <p>Supplementary local details</p>
      </SharedDisclosure>
      <button>Retry operation</button>
    </>,
  );
  const trigger = screen.getByRole('button', {
    name: 'Local diagnostics Needs attention',
  });
  expect(trigger.getAttribute('aria-expanded')).toBe('false');
  expect(screen.queryByText('Supplementary local details')).toBeNull();
  trigger.focus();
  await userEvent.keyboard('{Enter}');
  await screen.findByText('Supplementary local details');
  expect(trigger.getAttribute('aria-expanded')).toBe('true');
  await userEvent.keyboard(' ');
  await waitFor(() =>
    expect(screen.queryByText('Supplementary local details')).toBeNull(),
  );
  expect(screen.getByRole('button', { name: 'Retry operation' })).toBeTruthy();
});
