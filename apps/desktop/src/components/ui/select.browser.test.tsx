import { useState } from 'react';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { AppearanceControl } from '@/features/appearance/components/appearance-control';
import '@/index.css';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from './select';

function PopupFixture() {
  const [value, setValue] = useState('second');
  return (
    <div className="mx-auto w-full max-w-sm p-5">
      <Select value={value} onValueChange={setValue}>
        <SelectTrigger aria-label="Local provider" className="w-full">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="first">First local option</SelectItem>
          <SelectItem value="second">Second local option</SelectItem>
          <SelectItem value="third">
            A longer local option with enough descriptive text to exercise
            wrapping on narrow screens
          </SelectItem>
        </SelectContent>
      </Select>
      <p role="status">Selected: {value}</p>
      <AppearanceControl />
    </div>
  );
}
afterEach(async () => {
  cleanup();
  await page.viewport(1440, 900);
});

test('pointer and keyboard selections dismiss the popup and return focus to its trigger', async () => {
  render(<PopupFixture />);
  const trigger = screen.getByRole('combobox', { name: 'Local provider' });
  await userEvent.click(trigger);
  await userEvent.click(
    await screen.findByRole('option', { name: 'First local option' }),
  );
  await waitFor(() => expect(screen.queryByRole('listbox')).toBeNull());
  expect(document.activeElement).toBe(trigger);
  expect(screen.getByRole('status').textContent).toContain('first');
  await userEvent.keyboard('{Enter}');
  await screen.findByRole('listbox');
  await waitFor(() =>
    expect(document.activeElement).toBe(
      screen.getByRole('option', { name: 'First local option' }),
    ),
  );
  await userEvent.keyboard('{ArrowDown}');
  await waitFor(() =>
    expect(document.activeElement).toBe(
      screen.getByRole('option', { name: 'Second local option' }),
    ),
  );
  await userEvent.keyboard('{Enter}');
  await waitFor(() => expect(screen.queryByRole('listbox')).toBeNull());
  expect(document.activeElement).toBe(trigger);
  expect(screen.getByRole('status').textContent).toContain('second');
  await userEvent.keyboard('{Tab}');
  expect(document.activeElement).toBe(
    screen.getByRole('combobox', { name: 'Color appearance' }),
  );
});

test('visual popup audit keeps options outside the trigger and inside the viewport in both themes', async () => {
  for (const mode of ['light', 'dark'])
    for (const width of [1440, 1024, 768, 390, 320]) {
      document.documentElement.dataset.theme = mode;
      await page.viewport(width, 900);
      const view = render(<PopupFixture />);
      const trigger = screen.getByRole('combobox', { name: 'Local provider' });
      await userEvent.click(trigger);
      const popup = await screen.findByRole('listbox');
      // This is an intentional layout audit of the approved popup geometry,
      // backed by real selection and image artifacts rather than class snapshots.
      await waitFor(() => {
        const box = popup.getBoundingClientRect();
        expect(box.top).toBeGreaterThanOrEqual(
          trigger.getBoundingClientRect().bottom,
        );
        expect(box.left).toBeGreaterThanOrEqual(0);
        expect(box.right).toBeLessThanOrEqual(window.innerWidth);
      });
      await page.screenshot({
        path: `__screenshots__/select-${mode}-${width}.png`,
      });
      await userEvent.click(
        screen.getByRole('option', { name: /A longer local option/ }),
      );
      await waitFor(() => expect(screen.queryByRole('listbox')).toBeNull());
      expect(document.activeElement).toBe(trigger);
      view.unmount();
    }
});
