import { useEffect, useState } from 'react';

/** Sample only the small consumer that displays time, never Studio's owner. */
export function useLivePlaybackTime(
  anchor: number,
  playing: boolean,
  read: () => number,
) {
  const [sample, setSample] = useState<{ anchor: number; time: number } | null>(
    null,
  );
  useEffect(() => {
    if (!playing) return;
    const timer = window.setInterval(
      () => setSample({ anchor, time: read() }),
      100,
    );
    return () => window.clearInterval(timer);
  }, [anchor, playing, read]);
  return playing && sample?.anchor === anchor ? sample.time : anchor;
}
