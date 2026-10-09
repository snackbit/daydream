import { expect, test } from 'vitest';
import { parseAppearancePreference } from './appearance-store';

test('appearance preferences preserve saved modes and default new or invalid storage to light', () => {
  expect(parseAppearancePreference('light')).toBe('light');
  expect(parseAppearancePreference('dark')).toBe('dark');
  expect(parseAppearancePreference(null)).toBe('light');
  expect(parseAppearancePreference('auto')).toBe('light');
});
