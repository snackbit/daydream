import { useRef, useState } from 'react';
import type { CSSProperties, PointerEvent } from 'react';
import type { RecorderScene, RecorderSceneSource } from '@daydream/shared';
import {
  moveSceneSource,
  resizeSceneSource,
  sortedSceneSources,
  type ResizeHandle,
} from '@/features/recorder/scene-layout';
import { cn } from '@/lib/utils';

type DragState =
  | {
      clientX: number;
      clientY: number;
      handle: ResizeHandle;
      scene: RecorderScene;
      sourceId: string;
      pointerId: number;
      type: 'resize';
    }
  | {
      clientX: number;
      clientY: number;
      scene: RecorderScene;
      sourceId: string;
      pointerId: number;
      type: 'move';
    };
type PendingDragState =
  | Omit<Extract<DragState, { type: 'resize' }>, 'pointerId'>
  | Omit<Extract<DragState, { type: 'move' }>, 'pointerId'>;

export interface SceneCanvasProps {
  onChange: (scene: RecorderScene) => void;
  onSelect: (sourceId: string | null) => void;
  scene: RecorderScene;
  selectedSourceId: string | null;
}

export function SceneCanvas({
  onChange,
  onSelect,
  scene,
  selectedSourceId,
}: SceneCanvasProps) {
  const dragState = useRef<DragState | null>(null);
  const canvasRef = useRef<HTMLDivElement | null>(null);
  const [activeDrag, setActiveDrag] = useState<DragState['type'] | null>(null);

  const onPointerMove = (event: PointerEvent<HTMLDivElement>) => {
    const drag = dragState.current;
    const canvas = canvasRef.current;
    if (!drag || !canvas) {
      return;
    }
    if (event.pointerId !== drag.pointerId) {
      return;
    }

    const rect = canvas.getBoundingClientRect();
    const scaleX = drag.scene.canvasWidth / rect.width;
    const scaleY = drag.scene.canvasHeight / rect.height;
    const deltaX = (event.clientX - drag.clientX) * scaleX;
    const deltaY = (event.clientY - drag.clientY) * scaleY;
    const nextScene =
      drag.type === 'move'
        ? moveSceneSource(drag.scene, drag.sourceId, deltaX, deltaY)
        : resizeSceneSource(
            drag.scene,
            drag.sourceId,
            drag.handle,
            deltaX,
            deltaY,
          );

    onChange(nextScene);
  };
  const endDrag = (pointerId: number) => {
    const canvas = canvasRef.current;
    const drag = dragState.current;
    if (drag && drag.pointerId !== pointerId) {
      return;
    }

    if (canvas?.hasPointerCapture(pointerId)) {
      try {
        canvas.releasePointerCapture(pointerId);
      } catch {
        // Pointer capture may already be gone after pointercancel/lostpointercapture.
      }
    }
    dragState.current = null;
    setActiveDrag(null);
  };
  const startDrag = (
    event: PointerEvent<HTMLElement>,
    drag: PendingDragState,
  ) => {
    const canvas = canvasRef.current;
    try {
      canvas?.setPointerCapture(event.pointerId);
    } catch {
      // Continue without capture in test/browser environments that reject it.
    }
    dragState.current = {
      ...drag,
      pointerId: event.pointerId,
    };
    setActiveDrag(drag.type);
  };

  return (
    <div
      className={cn(
        'bg-muted relative mx-auto w-full overflow-hidden border',
        activeDrag === 'move' ? 'cursor-grabbing' : null,
      )}
      onLostPointerCapture={(event) => {
        endDrag(event.pointerId);
      }}
      onPointerCancel={(event) => {
        endDrag(event.pointerId);
      }}
      onPointerMove={onPointerMove}
      onPointerUp={(event) => {
        endDrag(event.pointerId);
      }}
      ref={canvasRef}
      style={{
        aspectRatio: `${scene.canvasWidth} / ${scene.canvasHeight}`,
        maxWidth: scene.canvasWidth,
      }}
    >
      {sortedSceneSources(scene).map((source) => (
        <div
          className={cn(
            'bg-background/80 absolute border shadow-sm',
            selectedSourceId === source.id
              ? 'border-primary ring-primary/30 ring-2'
              : 'border-foreground/30',
          )}
          data-testid={`scene-source-${source.sourceId}`}
          key={source.id}
          onPointerDown={() => {
            onSelect(source.id);
          }}
          style={sceneSourceStyle(scene, source)}
        >
          <button
            className={cn(
              'bg-background/90 flex w-full cursor-grab items-center gap-2 border-b px-2 py-1 text-left text-xs font-medium active:cursor-grabbing',
            )}
            aria-label={`Move ${source.displayName}`}
            onPointerDown={(event) => {
              event.stopPropagation();
              onSelect(source.id);
              startDrag(event, {
                clientX: event.clientX,
                clientY: event.clientY,
                scene,
                sourceId: source.id,
                type: 'move',
              });
            }}
            type="button"
          >
            <DragThumb />
            {source.displayName}
          </button>
          <SyntheticSourcePreview />
          {resizeHandles.map((handle) => (
            <ResizeButton
              handle={handle}
              key={handle}
              onPointerDown={(event) => {
                event.stopPropagation();
                onSelect(source.id);
                startDrag(event, {
                  clientX: event.clientX,
                  clientY: event.clientY,
                  handle,
                  scene,
                  sourceId: source.id,
                  type: 'resize',
                });
              }}
            />
          ))}
        </div>
      ))}
    </div>
  );
}

const resizeHandles: ResizeHandle[] = [
  'n',
  's',
  'e',
  'w',
  'ne',
  'nw',
  'se',
  'sw',
];

interface ResizeButtonProps {
  handle: ResizeHandle;
  onPointerDown: (event: PointerEvent<HTMLButtonElement>) => void;
}

function ResizeButton({ handle, onPointerDown }: ResizeButtonProps) {
  return (
    <button
      aria-label={`Resize ${handle}`}
      className={cn(
        'bg-primary border-background absolute z-10 size-3 border',
        resizeHandleClassName(handle),
      )}
      onPointerDown={onPointerDown}
      type="button"
    />
  );
}

function DragThumb() {
  return (
    <span
      aria-hidden="true"
      className="grid grid-cols-2 gap-0.5"
      title="Move source"
    >
      {Array.from({ length: 6 }).map((_, index) => (
        <span className="bg-muted-foreground size-1 rounded-full" key={index} />
      ))}
    </span>
  );
}

function SyntheticSourcePreview() {
  return (
    <div
      aria-hidden="true"
      className="absolute inset-x-0 top-6 bottom-0 overflow-hidden"
      style={{
        background:
          'linear-gradient(135deg, var(--muted) 0%, var(--background) 55%, var(--muted) 100%)',
      }}
    >
      <div className="absolute inset-0 [background-image:linear-gradient(to_right,var(--border)_1px,transparent_1px),linear-gradient(to_bottom,var(--border)_1px,transparent_1px)] [background-size:32px_32px] opacity-70" />
      <div className="bg-primary/10 border-primary/20 absolute top-[18%] left-[8%] h-[20%] w-[36%] border" />
      <div className="bg-foreground/5 border-foreground/10 absolute right-[10%] bottom-[18%] h-[26%] w-[42%] border" />
    </div>
  );
}

function resizeHandleClassName(handle: ResizeHandle): string {
  switch (handle) {
    case 'n':
      return 'left-1/2 top-0 -translate-x-1/2 -translate-y-1/2 cursor-ns-resize';
    case 's':
      return 'bottom-0 left-1/2 -translate-x-1/2 translate-y-1/2 cursor-ns-resize';
    case 'e':
      return 'right-0 top-1/2 -translate-y-1/2 translate-x-1/2 cursor-ew-resize';
    case 'w':
      return 'left-0 top-1/2 -translate-x-1/2 -translate-y-1/2 cursor-ew-resize';
    case 'ne':
      return 'right-0 top-0 translate-x-1/2 -translate-y-1/2 cursor-nesw-resize';
    case 'nw':
      return 'left-0 top-0 -translate-x-1/2 -translate-y-1/2 cursor-nwse-resize';
    case 'se':
      return 'bottom-0 right-0 translate-x-1/2 translate-y-1/2 cursor-nwse-resize';
    case 'sw':
      return 'bottom-0 left-0 -translate-x-1/2 translate-y-1/2 cursor-nesw-resize';
  }
}

function sceneSourceStyle(
  scene: RecorderScene,
  source: RecorderSceneSource,
): CSSProperties {
  return {
    height: `${(source.rect.height / scene.canvasHeight) * 100}%`,
    left: `${(source.rect.x / scene.canvasWidth) * 100}%`,
    top: `${(source.rect.y / scene.canvasHeight) * 100}%`,
    width: `${(source.rect.width / scene.canvasWidth) * 100}%`,
  };
}
