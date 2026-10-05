import { expect, test } from 'vitest';
import { parseAppearancePreference } from './appearance-store';

test('appearance preferences accept known modes and default invalid storage to dark', () => {
  expect(parseAppearancePreference('light')).toBe('light');
  expect(parseAppearancePreference('dark')).toBe('dark');
  expect(parseAppearancePreference(null)).toBe('dark');
  expect(parseAppearancePreference('auto')).toBe('dark');
});
