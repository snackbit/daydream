import type { LibraryRecordingSortKey } from '@daydream/shared';
import { Grid2X2, List, RefreshCw, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { formatFileSize } from '@/features/library/library-format';
import type {
  LibraryOverviewActions,
  LibraryOverviewModel,
} from './use-library-overview-controller';

interface LibraryOverviewToolbarProps {
  actions: Pick<
    LibraryOverviewActions,
    'refresh' | 'requestSelectedDelete' | 'setSortKey' | 'setViewMode'
  >;
  model: Pick<
    LibraryOverviewModel,
    | 'librarySizeBytes'
    | 'recordings'
    | 'selectedIds'
    | 'selectedSizeBytes'
    | 'sortKey'
    | 'viewMode'
  >;
}

export function LibraryOverviewToolbar({
  actions,
  model,
}: LibraryOverviewToolbarProps) {
  return (
    <div className="flex flex-wrap items-center gap-2">
      <div className="text-muted-foreground mr-2 text-xs">
        {model.selectedIds.length > 0
          ? `${model.selectedIds.length} selected · ${formatFileSize(
              model.selectedSizeBytes,
            )}`
          : `${model.recordings.length} recordings · ${formatFileSize(
              model.librarySizeBytes,
            )} library size`}
      </div>
      {model.selectedIds.length > 0 ? (
        <Button
          className="gap-2"
          variant="destructive"
          onClick={actions.requestSelectedDelete}
        >
          <Trash2 className="size-3.5" />
          Delete
        </Button>
      ) : null}
      <Select
        onValueChange={(value) =>
          actions.setSortKey(value as LibraryRecordingSortKey)
        }
        value={model.sortKey}
      >
        <SelectTrigger aria-label="Sort recordings">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="started">Newest first</SelectItem>
          <SelectItem value="duration">Duration</SelectItem>
          <SelectItem value="status">Status</SelectItem>
          <SelectItem value="availability">Availability</SelectItem>
        </SelectContent>
      </Select>
      <div className="flex rounded-md border">
        <Button
          aria-label="Grid view"
          size="icon"
          variant={model.viewMode === 'grid' ? 'secondary' : 'ghost'}
          onClick={() => actions.setViewMode('grid')}
        >
          <Grid2X2 className="size-4" />
        </Button>
        <Button
          aria-label="List view"
          size="icon"
          variant={model.viewMode === 'list' ? 'secondary' : 'ghost'}
          onClick={() => actions.setViewMode('list')}
        >
          <List className="size-4" />
        </Button>
      </div>
      <Button
        className="gap-2"
        variant="outline"
        onClick={() => void actions.refresh()}
      >
        <RefreshCw className="size-3.5" />
        Refresh
      </Button>
    </div>
  );
}
