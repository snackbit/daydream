export interface Point {
  x: number;
  y: number;
}
export interface CropBox extends Point {
  width: number;
  height: number;
}
export interface SourceSize {
  width: number;
  height: number;
}
export interface EditorView {
  left: number;
  top: number;
  scale: number;
  zoom: number;
  panX: number;
  panY: number;
}
export type CropHandle =
  'draw' | 'move' | 'n' | 's' | 'e' | 'w' | 'ne' | 'nw' | 'se' | 'sw';
const clamp = (value: number, min: number, max: number) =>
  Math.min(max, Math.max(min, Math.round(value)));
export function sourcePoint(
  point: Point,
  view: EditorView,
  source: SourceSize,
): Point {
  return {
    x: clamp(
      (point.x - view.left - view.panX) / (view.scale * view.zoom),
      0,
      source.width,
    ),
    y: clamp(
      (point.y - view.top - view.panY) / (view.scale * view.zoom),
      0,
      source.height,
    ),
  };
}
export function cropGesture(
  handle: CropHandle,
  crop: CropBox,
  start: Point,
  point: Point,
  bounds: SourceSize,
): CropBox {
  const dx = point.x - start.x,
    dy = point.y - start.y;
  if (handle === 'move')
    return {
      ...crop,
      x: clamp(crop.x + dx, 0, bounds.width - crop.width),
      y: clamp(crop.y + dy, 0, bounds.height - crop.height),
    };
  if (handle === 'draw') {
    const left = clamp(Math.min(start.x, point.x), 0, bounds.width - 2),
      top = clamp(Math.min(start.y, point.y), 0, bounds.height - 2);
    return {
      x: left,
      y: top,
      width: clamp(Math.abs(dx), 2, bounds.width - left),
      height: clamp(Math.abs(dy), 2, bounds.height - top),
    };
  }
  let left = crop.x,
    right = crop.x + crop.width,
    top = crop.y,
    bottom = crop.y + crop.height;
  if (handle.includes('w')) left = clamp(left + dx, 0, right - 2);
  if (handle.includes('e')) right = clamp(right + dx, left + 2, bounds.width);
  if (handle.includes('n')) top = clamp(top + dy, 0, bottom - 2);
  if (handle.includes('s')) bottom = clamp(bottom + dy, top + 2, bounds.height);
  return { x: left, y: top, width: right - left, height: bottom - top };
}
export function previewDemandSize(
  source: SourceSize,
  width: number,
  height: number,
  dpr: number,
  zoom: number,
): SourceSize {
  const scale = Math.min(
    1,
    (width * dpr * zoom) / source.width,
    (height * dpr * zoom) / source.height,
    3840 / Math.max(source.width, source.height),
    Math.sqrt(4_000_000 / (source.width * source.height)),
  );
  return {
    width: Math.max(1, Math.floor(source.width * scale)),
    height: Math.max(1, Math.floor(source.height * scale)),
  };
}
