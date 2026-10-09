import { useEffect, useState } from 'react';
import type { RefObject } from 'react';
import { previewDemandSize } from './crop-editor-geometry';
import { updateRecorderPreviewDemand } from './recorder-client';

let nextGeneration = Date.now();
export function usePreviewDemand(
  container: RefObject<HTMLDivElement | null>,
  sourceId: string,
  sourceWidth: number,
  sourceHeight: number,
  zoom: number,
  disabled: boolean,
) {
  const [viewport, setViewport] = useState({ width: 1, height: 1 });
  const [demand, setDemand] = useState<{ generation: number; error: boolean }>({
    generation: 0,
    error: false,
  });
  useEffect(() => {
    const element = container.current;
    if (!element) return;
    let scheduled = 0;
    function measure() {
      if (element)
        setViewport((previous) =>
          previous.width === element.clientWidth &&
          previous.height === element.clientHeight
            ? previous
            : { width: element.clientWidth, height: element.clientHeight },
        );
    }
    const observer = new ResizeObserver(() => {
      cancelAnimationFrame(scheduled);
      scheduled = requestAnimationFrame(measure);
    });
    observer.observe(element);
    window.addEventListener('resize', measure);
    measure();
    return () => {
      observer.disconnect();
      cancelAnimationFrame(scheduled);
      window.removeEventListener('resize', measure);
    };
  }, [container]);
  useEffect(() => {
    if (disabled || viewport.width <= 1 || viewport.height <= 1) return;
    let active = true;
    const generation = ++nextGeneration;
    setDemand({ generation, error: false });
    const size = previewDemandSize(
      { width: sourceWidth, height: sourceHeight },
      viewport.width,
      viewport.height,
      window.devicePixelRatio || 1,
      zoom,
    );
    void updateRecorderPreviewDemand({
      sceneSourceId: sourceId,
      ...size,
      generation,
    }).catch(() => {
      if (active) setDemand({ generation, error: true });
    });
    return () => {
      active = false;
    };
  }, [disabled, sourceId, sourceWidth, sourceHeight, viewport, zoom]);
  return { viewport, ...demand };
}
