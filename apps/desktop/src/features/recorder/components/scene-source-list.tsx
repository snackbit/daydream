import type { RecorderSceneSource } from '@daydream/shared';
import { Monitor, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { formatPixels } from '@/features/recorder/scene-layout';

export interface SceneSourceListProps {
  deleteDisabled?: boolean;
  onDelete: (sourceId: string) => void;
  onSelect: (sourceId: string) => void;
  selectedSceneSourceId: string | null;
  sources: RecorderSceneSource[];
}

export function SceneSourceList({
  deleteDisabled = false,
  onDelete,
  onSelect,
  selectedSceneSourceId,
  sources,
}: SceneSourceListProps) {
  if (sources.length === 0) {
    return (
      <p className="text-muted-foreground border-y py-2 text-xs">
        No monitor sources selected.
      </p>
    );
  }

  return (
    <div className="divide-y border-y">
      {sources.map((source) => {
        const isSelected = selectedSceneSourceId === source.id;
        const handleSelect = () => onSelect(source.id);
        const handleDelete = () => onDelete(source.id);

        return (
          <div
            className="group/source hover:bg-muted/60 data-[selected=true]:border-l-primary data-[selected=true]:bg-accent/30 flex items-center gap-2 border-l-2 border-l-transparent px-2 py-1.5 transition-colors"
            data-selected={isSelected}
            key={source.id}
          >
            <Button
              aria-label={`Focus ${source.displayName}`}
              aria-pressed={isSelected}
              className="h-auto min-w-0 flex-1 justify-start gap-2 rounded-sm px-0 py-0 text-left font-normal whitespace-normal hover:bg-transparent"
              onClick={handleSelect}
              size="sm"
              type="button"
              variant="ghost"
            >
              <Monitor
                aria-hidden="true"
                className="text-muted-foreground size-3.5 shrink-0"
              />
              <span className="min-w-0 flex-1">
                <span className="block truncate text-xs font-medium">
                  {source.displayName}
                </span>
                <span className="text-muted-foreground block font-mono text-[10px] tabular-nums">
                  {formatPixels(source.rect.width)} x{' '}
                  {formatPixels(source.rect.height)}
                </span>
              </span>
              <span className="text-muted-foreground hidden shrink-0 text-[10px] group-data-[selected=true]/source:block">
                Focused
              </span>
              <span className="text-muted-foreground hidden shrink-0 text-[10px] group-data-[selected=false]/source:block">
                Click to focus
              </span>
            </Button>
            <Button
              aria-label={`Delete ${source.displayName}`}
              disabled={deleteDisabled}
              onClick={handleDelete}
              size="icon-xs"
              type="button"
              variant="ghost"
            >
              <Trash2 className="size-3" />
            </Button>
          </div>
        );
      })}
    </div>
  );
}
