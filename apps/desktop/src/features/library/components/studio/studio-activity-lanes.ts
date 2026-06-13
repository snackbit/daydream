export const ACTIVITY_MOUSE_LANE_ID = 'activity:mouse';
export const ACTIVITY_KEYBOARD_LANE_ID = 'activity:keyboard';
export const ACTIVITY_FOCUS_LANE_ID = 'activity:focus';

export type StudioActivityLaneKind =
  | 'activity-focus'
  | 'activity-keyboard'
  | 'activity-mouse';

export interface StudioActivityLaneDefinition {
  colorId: string;
  description: string;
  id: string;
  kind: StudioActivityLaneKind;
  label: string;
}

export const STUDIO_ACTIVITY_LANES: StudioActivityLaneDefinition[] = [
  {
    colorId: 'emerald',
    description: 'Mouse movement and clicks',
    id: ACTIVITY_MOUSE_LANE_ID,
    kind: 'activity-mouse',
    label: 'Mouse',
  },
  {
    colorId: 'violet',
    description: 'Keyboard input activity',
    id: ACTIVITY_KEYBOARD_LANE_ID,
    kind: 'activity-keyboard',
    label: 'Keyboard',
  },
  {
    colorId: 'rose',
    description: 'Window focus and activity events',
    id: ACTIVITY_FOCUS_LANE_ID,
    kind: 'activity-focus',
    label: 'Activity',
  },
];

export function isStudioActivityLaneKind(
  kind: string,
): kind is StudioActivityLaneKind {
  return (
    kind === 'activity-focus' ||
    kind === 'activity-keyboard' ||
    kind === 'activity-mouse'
  );
}
