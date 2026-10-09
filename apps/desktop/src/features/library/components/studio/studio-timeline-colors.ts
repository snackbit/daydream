export interface StudioTimelineColor {
  border: string;
  fill: string;
  id: string;
  label: string;
  strong: string;
}

/** Persisted palette IDs remain stable; the shared theme owns contrast in both appearances. */
const colorNames = [
  'Blue',
  'Violet',
  'Cyan',
  'Emerald',
  'Lime',
  'Amber',
  'Orange',
  'Red',
  'Pink',
  'Purple',
  'Teal',
  'Sky',
  'Indigo',
  'Green',
  'Chartreuse',
  'Gold',
  'Tangerine',
  'Rose',
  'Fuchsia',
  'Grape',
  'Slate',
  'Stone',
  'Sea',
  'Steel',
];

export const STUDIO_TIMELINE_COLORS: StudioTimelineColor[] = colorNames.map(
  (label) => {
    const id = label.toLowerCase();
    const strong = `var(--annotation-${id})`;
    return {
      id,
      label,
      strong,
      border: strong,
      fill: `color-mix(in oklch, ${strong} 15%, var(--card))`,
    };
  },
);

export function studioColorById(colorId: string): StudioTimelineColor {
  return (
    STUDIO_TIMELINE_COLORS.find((color) => color.id === colorId) ??
    STUDIO_TIMELINE_COLORS[0]!
  );
}
