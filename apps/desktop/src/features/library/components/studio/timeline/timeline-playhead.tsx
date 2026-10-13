import { useEffect, useRef } from 'react';
import { TRACK_LABEL_WIDTH_PX } from '../track-timeline-math';

export interface TimelinePlayheadProps {
  time: number;
  readTime: () => number;
  playing: boolean;
  width: number;
  duration: number;
  follow: boolean;
  keepVisible: (left: number) => void;
}
export function TimelinePlayhead({
  time,
  readTime,
  playing,
  width,
  duration,
  follow,
  keepVisible,
}: TimelinePlayheadProps) {
  const element = useRef<HTMLDivElement>(null);
  useEffect(() => {
    let frame = 0;
    function draw() {
      const left =
        TRACK_LABEL_WIDTH_PX +
        (Math.max(0, Math.min(duration, playing ? readTime() : time)) /
          duration) *
          width;
      if (follow && playing) keepVisible(left);
      if (element.current)
        element.current.style.transform = `translateX(${left}px)`;
      if (playing) frame = requestAnimationFrame(draw);
    }
    draw();
    return () => cancelAnimationFrame(frame);
  }, [time, readTime, playing, width, duration, follow, keepVisible]);
  return (
    <div
      ref={element}
      aria-hidden="true"
      className="bg-primary pointer-events-none absolute top-0 bottom-0 left-0 z-30 w-px"
    />
  );
}
