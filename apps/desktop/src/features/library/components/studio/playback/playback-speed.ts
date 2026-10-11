export const playbackSpeeds = [0.25, 0.5, 1, 1.25, 1.5, 2] as const;
export type PlaybackSpeed = (typeof playbackSpeeds)[number];
