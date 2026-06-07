import type { RecorderSceneSource } from '@daydream/shared';
import { MousePointer2, Trash2 } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { formatPixels } from '@/features/recorder/scene-layout';
import { cn } from '@/lib/utils';

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
      <Card size="sm">
        <CardContent className="text-muted-foreground text-xs">
          No monitor sources selected.
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="grid gap-2">
      {sources.map((source) => {
        const isSelected = selectedSceneSourceId === source.id;

        return (
          <Card
            className={cn(
              'transition-colors',
              isSelected
                ? 'border-primary bg-primary/5'
                : 'hover:border-primary/40 hover:bg-muted',
            )}
            key={source.id}
            size="sm"
          >
            <CardContent className="flex items-center gap-2">
              <button
                aria-label={`Focus ${source.displayName}`}
                aria-pressed={isSelected}
                className="min-w-0 flex-1 text-left"
                onClick={() => onSelect(source.id)}
                type="button"
              >
                <div className="flex min-w-0 items-center gap-2">
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-xs font-medium">
                      {source.displayName}
                    </div>
                    <div className="text-muted-foreground mt-1 font-mono text-xs">
                      {formatPixels(source.rect.width)} x{' '}
                      {formatPixels(source.rect.height)}
                    </div>
                  </div>
                  {isSelected ? (
                    <Badge className="gap-1" variant="default">
                      <MousePointer2 className="size-2.5" />
                      Focused
                    </Badge>
                  ) : (
                    <span className="text-muted-foreground shrink-0 text-[0.625rem]">
                      Click to focus
                    </span>
                  )}
                </div>
              </button>
              <Button
                aria-label={`Delete ${source.displayName}`}
                disabled={deleteDisabled}
                onClick={() => onDelete(source.id)}
                size="icon-xs"
                type="button"
                variant="ghost"
              >
                <Trash2 className="size-3" />
              </Button>
            </CardContent>
          </Card>
        );
      })}
    </div>
  );
}
