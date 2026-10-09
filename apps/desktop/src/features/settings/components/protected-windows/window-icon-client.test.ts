import { expect, test } from 'vitest';
import { decodeWindowIcon } from './window-icon-client';

const key = 'window-icon:synthetic';
test('icon decoding accepts only bounded PNG data for the requested local handle', () => {
  expect(decodeWindowIcon({ iconKey: key, pngBase64: null }, key)).toBeNull();
  expect(
    decodeWindowIcon({ iconKey: key, pngBase64: 'iVBORw0KGgo=' }, key),
  ).toBe('data:image/png;base64,iVBORw0KGgo=');
  expect(() =>
    decodeWindowIcon({ iconKey: 'other', pngBase64: null }, key),
  ).toThrow('Invalid local icon response');
  expect(() =>
    decodeWindowIcon({ iconKey: key, pngBase64: '<svg/>' }, key),
  ).toThrow('Invalid local PNG icon');
  expect(() =>
    decodeWindowIcon(
      { iconKey: key, pngBase64: 'iVBORw0KGgo' + 'A'.repeat(90_000) },
      key,
    ),
  ).toThrow('Invalid local PNG icon');
});
