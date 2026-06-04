import { expect, test } from 'vitest';
import { navigationItems } from './navigation';

test('desktop shell includes home and diagnostics routes', () => {
  expect(navigationItems.map((item) => item.href)).toContain('/');
  expect(navigationItems.map((item) => item.href)).toContain('/diagnostics');
});
