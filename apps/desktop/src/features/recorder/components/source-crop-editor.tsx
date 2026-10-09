import {
  useEffect,
  useRef,
  useState,
  type KeyboardEvent,
  type PointerEvent,
} from 'react';
import type {
  RecorderPreviewFrame,
  RecorderSceneSource,
} from '@daydream/shared';
import { Button } from '@/components/ui/button';
import {
  cropGesture,
  sourcePoint,
  type CropBox,
  type CropHandle,
  type Point,
} from '../crop-editor-geometry';
import { cropSource, croppedDimensions } from '../source-capture-geometry';
import { useDecodedPreview } from '../use-decoded-preview';
import { usePreviewDemand } from '../use-preview-demand';

export interface SourceCropEditorProps {
  source: RecorderSceneSource;
  frame: RecorderPreviewFrame | undefined;
  disabled: boolean;
  unavailable?: boolean;
  onChange: (source: RecorderSceneSource) => void;
}
const handles: { handle: CropHandle; x: number; y: number }[] = [
  { handle: 'nw', x: 0, y: 0 },
  { handle: 'n', x: 0.5, y: 0 },
  { handle: 'ne', x: 1, y: 0 },
  { handle: 'e', x: 1, y: 0.5 },
  { handle: 'se', x: 1, y: 1 },
  { handle: 's', x: 0.5, y: 1 },
  { handle: 'sw', x: 0, y: 1 },
  { handle: 'w', x: 0, y: 0.5 },
];
function cropHandle(value: string | undefined): CropHandle {
  return (
    handles.find((item) => item.handle === value)?.handle ??
    (value === 'move' ? 'move' : 'draw')
  );
}
export function SourceCropEditor({
  source,
  frame,
  disabled,
  unavailable = false,
  onChange,
}: SourceCropEditorProps) {
  const container = useRef<HTMLDivElement>(null);
  const image = useRef<HTMLImageElement>(null);
  const [view, setView] = useState({ zoom: 1, panX: 0, panY: 0 });
  const [drawing, setDrawing] = useState(false);
  const space = useRef(false);
  const gesture = useRef<{
    handle: CropHandle | 'pan';
    start: Point;
    client: Point;
    crop: CropBox;
    original: RecorderSceneSource;
    panX: number;
    panY: number;
    pointerId: number;
  } | null>(null);
  const bounds = {
    width: source.sourceWidth ?? 1,
    height: source.sourceHeight ?? 1,
  };
  const demand = usePreviewDemand(
    container,
    source.id,
    bounds.width,
    bounds.height,
    view.zoom,
    disabled,
  );
  const { decoded, imageError } = useDecodedPreview(frame, demand.generation);
  const scale = Math.min(
    demand.viewport.width / bounds.width,
    demand.viewport.height / bounds.height,
  );
  const left = (demand.viewport.width - bounds.width * scale) / 2,
    top = (demand.viewport.height - bounds.height * scale) / 2;
  const transform = { left, top, scale, ...view };
  const crop = {
    x: source.crop.left,
    y: source.crop.top,
    ...croppedDimensions(source),
  };
  useEffect(() => {
    const element = container.current;
    if (!element) return;
    function wheel(event: WheelEvent) {
      event.preventDefault();
      const box = element?.getBoundingClientRect();
      if (!box) return;
      const x = event.clientX - box.left - left,
        y = event.clientY - box.top - top;
      setView((current) => {
        const zoom = Math.min(
          8,
          Math.max(1, current.zoom * Math.exp(-event.deltaY * 0.001)),
        );
        const ratio = zoom / current.zoom;
        return {
          zoom,
          panX: x - (x - current.panX) * ratio,
          panY: y - (y - current.panY) * ratio,
        };
      });
    }
    element.addEventListener('wheel', wheel, { passive: false });
    return () => element.removeEventListener('wheel', wheel);
  }, [left, top]);
  function localPoint(event: PointerEvent<HTMLDivElement>) {
    const box = event.currentTarget.getBoundingClientRect();
    return { x: event.clientX - box.left, y: event.clientY - box.top };
  }
  function begin(event: PointerEvent<HTMLDivElement>) {
    if (event.button !== 0 || disabled) return;
    event.preventDefault();
    event.currentTarget.focus();
    event.currentTarget.setPointerCapture(event.pointerId);
    const point = localPoint(event);
    const target =
      event.target instanceof Element
        ? event.target.closest('[data-crop-handle]')
        : null;
    gesture.current = {
      handle: space.current
        ? 'pan'
        : drawing
          ? 'draw'
          : cropHandle(target?.getAttribute('data-crop-handle') ?? undefined),
      start: sourcePoint(point, transform, bounds),
      client: point,
      crop,
      original: source,
      panX: view.panX,
      panY: view.panY,
      pointerId: event.pointerId,
    };
  }
  function move(event: PointerEvent<HTMLDivElement>) {
    const current = gesture.current;
    if (!current || disabled) return;
    const point = localPoint(event);
    if (current.handle === 'pan')
      setView({
        ...view,
        panX: current.panX + point.x - current.client.x,
        panY: current.panY + point.y - current.client.y,
      });
    else
      onChange(
        cropSource(
          current.original,
          cropGesture(
            current.handle,
            current.crop,
            current.start,
            sourcePoint(point, transform, bounds),
            bounds,
          ),
        ),
      );
  }
  function finish() {
    gesture.current = null;
    setDrawing(false);
  }
  function cancel() {
    const current = gesture.current;
    if (current) {
      if (current.handle === 'pan')
        setView({ ...view, panX: current.panX, panY: current.panY });
      else onChange(current.original);
    }
    gesture.current = null;
    space.current = false;
  }
  function keyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (event.code === 'Space') {
      event.preventDefault();
      space.current = true;
    }
    if (event.key === 'Escape') cancel();
    if (
      disabled ||
      !['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(event.key)
    )
      return;
    event.preventDefault();
    const step = event.shiftKey ? 10 : 1;
    const delta = {
      x:
        event.key === 'ArrowLeft'
          ? -step
          : event.key === 'ArrowRight'
            ? step
            : 0,
      y: event.key === 'ArrowUp' ? -step : event.key === 'ArrowDown' ? step : 0,
    };
    onChange(
      cropSource(
        source,
        cropGesture(
          event.altKey ? 'se' : 'move',
          crop,
          { x: 0, y: 0 },
          delta,
          bounds,
        ),
      ),
    );
  }
  function keyUp(event: KeyboardEvent<HTMLDivElement>) {
    if (event.code === 'Space') space.current = false;
  }
  const handleSize = 10 / Math.max(0.001, scale * view.zoom);
  return (
    <div className="grid min-h-0 flex-1 gap-2">
      <Button
        size="sm"
        variant="outline"
        disabled={disabled}
        aria-pressed={drawing}
        onClick={() => setDrawing(!drawing)}
      >
        Draw crop
      </Button>
      <div
        ref={container}
        role="region"
        tabIndex={0}
        aria-label="Crop editor"
        aria-disabled={disabled}
        className="relative min-h-64 touch-none overflow-hidden rounded-lg border bg-black outline-offset-2"
        onPointerDown={begin}
        onPointerMove={move}
        onPointerUp={finish}
        onPointerCancel={cancel}
        onLostPointerCapture={finish}
        onKeyDown={keyDown}
        onKeyUp={keyUp}
        onBlur={cancel}
      >
        <div
          className="absolute origin-top-left"
          style={{
            left: left + view.panX,
            top: top + view.panY,
            width: bounds.width * scale,
            height: bounds.height * scale,
            transform: `scale(${view.zoom})`,
          }}
        >
          {decoded ? (
            <img
              ref={image}
              alt={`${source.displayName} preview`}
              src={decoded.url}
              draggable={false}
              className="size-full select-none"
            />
          ) : (
            <p className="text-muted-foreground p-4 text-sm">
              {imageError || demand.error || unavailable
                ? 'Preview unavailable'
                : 'Loading preview…'}
            </p>
          )}
          <svg
            className="absolute inset-0 size-full"
            viewBox={`0 0 ${bounds.width} ${bounds.height}`}
            aria-label="Capture region"
          >
            <rect
              x={crop.x}
              y={crop.y}
              width={crop.width}
              height={crop.height}
              fill="transparent"
              stroke="black"
              strokeWidth={4 / Math.max(0.001, scale * view.zoom)}
              pointerEvents="none"
            />
            <rect
              data-crop-handle="move"
              x={crop.x}
              y={crop.y}
              width={crop.width}
              height={crop.height}
              fill="transparent"
              className="cursor-move stroke-white"
              strokeWidth={2 / Math.max(0.001, scale * view.zoom)}
            />
            {!disabled
              ? handles.map((item) => (
                  <rect
                    key={item.handle}
                    data-crop-handle={item.handle}
                    x={crop.x + crop.width * item.x - handleSize / 2}
                    y={crop.y + crop.height * item.y - handleSize / 2}
                    width={handleSize}
                    height={handleSize}
                    className="fill-black stroke-white"
                    strokeWidth={1 / Math.max(0.001, scale * view.zoom)}
                  />
                ))
              : null}
          </svg>
        </div>
      </div>
      <p className="text-muted-foreground text-xs">
        Drag to crop; drag the rectangle to move it. Wheel to zoom, Space-drag
        to pan. Arrow keys move; Alt-arrow resizes. Escape cancels.
      </p>
    </div>
  );
}
