import { expect, test } from 'vitest';
import { navigationItems } from './navigation';

test('desktop navigation exposes recall workflows and diagnostics', () => {
  const destinations = navigationItems.map((item) => item.href);
  expect(destinations).toEqual(
    expect.arrayContaining([
      '/library',
      '/capture',
      '/search',
      '/processing',
      '/settings',
      '/diagnostics',
    ]),
  );
  expect(new Set(destinations).size).toBe(destinations.length);
});
