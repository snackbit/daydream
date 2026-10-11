import { expect, test } from 'vitest';
import { parseSettingsLocation } from './settings-location';

test.each([
  ['models', 'models', null],
  ['transcription', 'ai', 'transcription'],
  ['vision', 'ai', 'observations'],
  ['recall-generation', 'ai', 'summary'],
  ['recall-embedding', 'ai', 'search'],
])(
  'legacy %s links open the corresponding section and editor',
  (focus, section, detail) => {
    expect(parseSettingsLocation({ tab: 'ai', focus })).toMatchObject({
      section,
      detail,
    });
  },
);
test('explicit sections override legacy tabs and unrecognized locations safely use General', () => {
  expect(
    parseSettingsLocation({
      tab: 'record',
      section: 'privacy',
      detail: 'protection',
    }),
  ).toMatchObject({ section: 'privacy', detail: 'protection' });
  expect(
    parseSettingsLocation({ section: 'anything', detail: 'anything' }),
  ).toMatchObject({ section: 'general', detail: null });
});
