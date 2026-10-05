import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, expect, test } from 'vitest';
import { useAppearanceStore } from '../appearance-store';
import { AppearanceControl } from './appearance-control';

const appearanceStorageKey = 'daydream.appearance';

beforeEach(() => {
  window.localStorage.removeItem(appearanceStorageKey);
  document.documentElement.dataset.theme = 'dark';
  useAppearanceStore.setState({
    mode: 'dark',
    persistence: 'saved',
  });
});

afterEach(() => {
  cleanup();
  window.localStorage.removeItem(appearanceStorageKey);
  document.documentElement.dataset.theme = 'dark';
  useAppearanceStore.setState({
    mode: 'dark',
    persistence: 'saved',
  });
});

test('appearance control saves and applies a selectable light mode', async () => {
  const view = render(<AppearanceControl />);
  const trigger = screen.getByRole('combobox', { name: 'Color appearance' });

  fireEvent.click(trigger);
  fireEvent.click(await screen.findByRole('option', { name: 'Light' }));

  expect(document.documentElement.dataset.theme).toBe('light');
  expect(window.localStorage.getItem(appearanceStorageKey)).toBe('light');
  expect(trigger.textContent).toContain('Light');

  view.unmount();
  render(<AppearanceControl />);

  expect(
    screen.getByRole('combobox', { name: 'Color appearance' }).textContent,
  ).toContain('Light');
  expect(document.documentElement.dataset.theme).toBe('light');
});
