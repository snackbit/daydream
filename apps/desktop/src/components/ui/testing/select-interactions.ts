import { screen, waitFor } from '@testing-library/react';
import { expect } from 'vitest';
import { userEvent } from 'vitest/browser';

/** Real popup lifecycle: Presence keeps modal ARIA hiding during exit animation. */
export async function waitForSelectDismissal(
  trigger: HTMLElement,
): Promise<void> {
  await waitFor(() => expect(screen.queryByRole('listbox')).toBeNull());
  await waitFor(() => expect(document.activeElement).toBe(trigger));
}

export async function chooseSelectOption(
  trigger: HTMLElement,
  option: string,
): Promise<void> {
  await userEvent.click(trigger);
  await userEvent.click(await screen.findByRole('option', { name: option }));
  await waitForSelectDismissal(trigger);
}
