import { expect, test } from 'vitest';
import { groupTimelineEntries } from './timeline-entry-groups';

test('groups include the earliest support even when entries arrive newest first', () => {
  const groups = groupTimelineEntries(
    [
      { id: 'new', startMs: 20, endMs: 23 },
      { id: 'old', startMs: 2, endMs: 5 },
    ],
    1,
  );
  expect(groups).toHaveLength(1);
  expect(groups[0]).toMatchObject({ startMs: 2, endMs: 23 });
});

test('an eight-hour dense day produces bounded readable groups without losing provenance', () => {
  const duration = 8 * 60 * 60_000;
  const items = Array.from({ length: 50_000 }, (_, i) => ({
    id: `entry-${i}`,
    startMs: (i * duration) / 50_000,
    endMs: ((i + 0.8) * duration) / 50_000,
  }));
  const groups = groupTimelineEntries(items, 1200 / duration);
  expect(groups).toHaveLength(25);
  expect(groups.flatMap((group) => group.items.map((item) => item.id))).toEqual(
    items.map((item) => item.id),
  );
  expect(items[0]?.id).toBe('entry-0');
  const detail = groupTimelineEntries(items.slice(0, 5), 1);
  expect(detail).toHaveLength(5);
  expect(detail.every((group) => group.id.startsWith('entry-'))).toBe(true);
});
test('empty input and narrow singleton entries retain a zoomable group', () => {
  expect(groupTimelineEntries([], 1)).toEqual([]);
  expect(
    groupTimelineEntries([{ id: 'tiny', startMs: 100, endMs: 101 }], 1)[0],
  ).toMatchObject({ id: 'group:2', startMs: 100, endMs: 101 });
});
