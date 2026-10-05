export interface StudioTimelineColor {
  border: string;
  fill: string;
  id: string;
  label: string;
  strong: string;
}

const timelineColorIds: StudioTimelineColor[] = [
  {
    border: '#2563eb',
    fill: '#dbeafe',
    id: 'blue',
    label: 'Blue',
    strong: '#1d4ed8',
  },
  {
    border: '#7c3aed',
    fill: '#ede9fe',
    id: 'violet',
    label: 'Violet',
    strong: '#6d28d9',
  },
  {
    border: '#0891b2',
    fill: '#cffafe',
    id: 'cyan',
    label: 'Cyan',
    strong: '#0e7490',
  },
  {
    border: '#059669',
    fill: '#d1fae5',
    id: 'emerald',
    label: 'Emerald',
    strong: '#047857',
  },
  {
    border: '#65a30d',
    fill: '#ecfccb',
    id: 'lime',
    label: 'Lime',
    strong: '#4d7c0f',
  },
  {
    border: '#ca8a04',
    fill: '#fef3c7',
    id: 'amber',
    label: 'Amber',
    strong: '#a16207',
  },
  {
    border: '#ea580c',
    fill: '#ffedd5',
    id: 'orange',
    label: 'Orange',
    strong: '#c2410c',
  },
  {
    border: '#dc2626',
    fill: '#fee2e2',
    id: 'red',
    label: 'Red',
    strong: '#b91c1c',
  },
  {
    border: '#db2777',
    fill: '#fce7f3',
    id: 'pink',
    label: 'Pink',
    strong: '#be185d',
  },
  {
    border: '#9333ea',
    fill: '#f3e8ff',
    id: 'purple',
    label: 'Purple',
    strong: '#7e22ce',
  },
  {
    border: '#0d9488',
    fill: '#ccfbf1',
    id: 'teal',
    label: 'Teal',
    strong: '#0f766e',
  },
  {
    border: '#0284c7',
    fill: '#e0f2fe',
    id: 'sky',
    label: 'Sky',
    strong: '#0369a1',
  },
  {
    border: '#4f46e5',
    fill: '#e0e7ff',
    id: 'indigo',
    label: 'Indigo',
    strong: '#4338ca',
  },
  {
    border: '#16a34a',
    fill: '#dcfce7',
    id: 'green',
    label: 'Green',
    strong: '#15803d',
  },
  {
    border: '#84cc16',
    fill: '#f7fee7',
    id: 'chartreuse',
    label: 'Chartreuse',
    strong: '#65a30d',
  },
  {
    border: '#d97706',
    fill: '#fef3c7',
    id: 'gold',
    label: 'Gold',
    strong: '#b45309',
  },
  {
    border: '#f97316',
    fill: '#ffedd5',
    id: 'tangerine',
    label: 'Tangerine',
    strong: '#ea580c',
  },
  {
    border: '#e11d48',
    fill: '#ffe4e6',
    id: 'rose',
    label: 'Rose',
    strong: '#be123c',
  },
  {
    border: '#c026d3',
    fill: '#fae8ff',
    id: 'fuchsia',
    label: 'Fuchsia',
    strong: '#a21caf',
  },
  {
    border: '#6d28d9',
    fill: '#ede9fe',
    id: 'grape',
    label: 'Grape',
    strong: '#5b21b6',
  },
  {
    border: '#475569',
    fill: '#f1f5f9',
    id: 'slate',
    label: 'Slate',
    strong: '#334155',
  },
  {
    border: '#78716c',
    fill: '#f5f5f4',
    id: 'stone',
    label: 'Stone',
    strong: '#57534e',
  },
  {
    border: '#0f766e',
    fill: '#ccfbf1',
    id: 'sea',
    label: 'Sea',
    strong: '#115e59',
  },
  {
    border: '#0369a1',
    fill: '#e0f2fe',
    id: 'steel',
    label: 'Steel',
    strong: '#075985',
  },
];

const studioColorHues: Record<string, number> = {
  amber: 82,
  blue: 245,
  chartreuse: 120,
  cyan: 220,
  emerald: 165,
  fuchsia: 325,
  gold: 90,
  grape: 290,
  green: 145,
  indigo: 255,
  lime: 120,
  orange: 68,
  pink: 350,
  purple: 285,
  red: 25,
  rose: 12,
  sea: 195,
  sky: 225,
  slate: 240,
  steel: 230,
  stone: 52,
  tangerine: 60,
  teal: 190,
  violet: 275,
};

export const STUDIO_TIMELINE_COLORS = timelineColorIds.map((color) => {
  const hue = studioColorHues[color.id] ?? 240;

  return {
    ...color,
    border: `oklch(0.69 0.035 ${hue})`,
    fill: `oklch(0.88 0.018 ${hue})`,
    strong: `oklch(0.62 0.055 ${hue})`,
  };
});

export function studioColorById(colorId: string): StudioTimelineColor {
  return (
    STUDIO_TIMELINE_COLORS.find((color) => color.id === colorId) ??
    STUDIO_TIMELINE_COLORS[0]!
  );
}
