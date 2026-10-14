import { useState } from 'react';
import type { StudioVisualFrame } from './studio-visual-observations';

/** Keep an explicitly selected frame while bounded viewport pages are replaced.
 * Privacy/recording changes discard the snapshot synchronously during render. */
export function useSelectedStudioFrames(
  owner: string,
  privacyRevision: string,
  ids: string[],
  frames: StudioVisualFrame[],
) {
  const [snapshot, setSnapshot] = useState({
    owner,
    privacyRevision,
    ids,
    input: frames,
    selected: frames.filter((frame) => ids.includes(frame.id)),
  });
  if (
    snapshot.owner !== owner ||
    snapshot.privacyRevision !== privacyRevision ||
    snapshot.ids !== ids ||
    snapshot.input !== frames
  ) {
    const eligible =
      snapshot.owner === owner && snapshot.privacyRevision === privacyRevision
        ? snapshot.selected
        : [];
    const latest = new Map(frames.map((frame) => [frame.id, frame]));
    const previous = new Map(eligible.map((frame) => [frame.id, frame]));
    const selected = ids
      .map((id) => latest.get(id) ?? previous.get(id))
      .filter((frame): frame is StudioVisualFrame => frame !== undefined);
    setSnapshot({ owner, privacyRevision, ids, input: frames, selected });
    return selected;
  }
  return snapshot.selected;
}
