import type { RecorderSceneSource } from '@daydream/shared';
import { Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { formatPixels } from '@/features/recorder/scene-layout';
import { cn } from '@/lib/utils';

export interface SceneSourceListProps {
  onDelete: (sourceId: string) => void;
  onSelect: (sourceId: string) => void;
  selectedSceneSourceId: string | null;
  sources: RecorderSceneSource[];
}

export function SceneSourceList({
  onDelete,
  onSelect,
  selectedSceneSourceId,
  sources,
}: SceneSourceListProps) {
  if (sources.length === 0) {
    return (
      <Card size="sm">
        <CardContent className="text-muted-foreground text-xs">
          No monitor sources selected.
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="grid gap-2">
      {sources.map((source) => (
        <Card
          className={cn(
            'transition-colors',
            selectedSceneSourceId === source.id
              ? 'border-primary'
              : 'hover:bg-muted',
          )}
          key={source.id}
          size="sm"
        >
          <CardContent className="flex items-center gap-2">
            <button
              className="min-w-0 flex-1 text-left"
              onClick={() => onSelect(source.id)}
              type="button"
            >
              <div className="truncate text-xs font-medium">
                {source.displayName}
              </div>
              <div className="text-muted-foreground mt-1 font-mono text-xs">
                {formatPixels(source.rect.width)} x{' '}
                {formatPixels(source.rect.height)}
              </div>
            </button>
            <Button
              aria-label={`Delete ${source.displayName}`}
              onClick={() => onDelete(source.id)}
              size="icon-xs"
              type="button"
              variant="ghost"
            >
              <Trash2 className="size-3" />
            </Button>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
