import { useCallback } from 'react';
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
    | 'status'
    | 'viewMode'
  >;
}

const sortKeys: LibraryRecordingSortKey[] = [
  'started',
  'duration',
  'status',
  'availability',
];

export function LibraryOverviewToolbar({
  actions,
  model,
}: LibraryOverviewToolbarProps) {
  const { refresh, requestSelectedDelete, setSortKey, setViewMode } = actions;
  const handleSortChange = useCallback(
    (value: string) => {
      const sortKey = sortKeys.find((candidate) => candidate === value);
      if (sortKey) {
        setSortKey(sortKey);
      }
    },
    [setSortKey],
  );
  const handleGridView = useCallback(() => {
    setViewMode('grid');
  }, [setViewMode]);
  const handleListView = useCallback(() => {
    setViewMode('list');
  }, [setViewMode]);
  const handleRefresh = useCallback(() => {
    void refresh();
  }, [refresh]);

  const hasSelection = model.selectedIds.length > 0;

  return (
    <div className="flex flex-col gap-2 border-b pb-2 sm:flex-row sm:items-center sm:justify-between">
      <p
        aria-label="Library summary"
        aria-live="polite"
        className="text-muted-foreground min-w-0 text-xs"
        role="status"
      >
        <span className="text-foreground font-medium tabular-nums">
          {hasSelection
            ? `${model.selectedIds.length} selected`
            : `${model.recordings.length} recordings`}
        </span>
        <span className="mx-1.5" aria-hidden="true">
          ·
        </span>
        {hasSelection
          ? `${formatFileSize(model.selectedSizeBytes)} selected`
          : `${formatFileSize(model.librarySizeBytes)} stored locally`}
      </p>

      <div className="flex flex-wrap items-center gap-1">
        {hasSelection ? (
          <Button
            className="gap-1.5"
            onClick={requestSelectedDelete}
            size="sm"
            variant="destructive"
          >
            <Trash2 aria-hidden="true" className="size-3" />
            Delete selected
          </Button>
        ) : null}

        {model.recordings.length > 0 ? (
          <Select onValueChange={handleSortChange} value={model.sortKey}>
            <SelectTrigger aria-label="Sort recordings" className="w-32">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="started">Newest first</SelectItem>
              <SelectItem value="duration">Duration</SelectItem>
              <SelectItem value="status">Status</SelectItem>
              <SelectItem value="availability">Availability</SelectItem>
            </SelectContent>
          </Select>
        ) : null}

        {model.recordings.length > 0 ? (
          <div
            aria-label="Recording layout"
            className="flex items-center"
            role="group"
          >
            <Button
              aria-label="Grid view"
              aria-pressed={model.viewMode === 'grid'}
              onClick={handleGridView}
              size="icon"
              variant={model.viewMode === 'grid' ? 'secondary' : 'ghost'}
            >
              <Grid2X2 aria-hidden="true" className="size-3.5" />
            </Button>
            <Button
              aria-label="List view"
              aria-pressed={model.viewMode === 'list'}
              onClick={handleListView}
              size="icon"
              variant={model.viewMode === 'list' ? 'secondary' : 'ghost'}
            >
              <List aria-hidden="true" className="size-3.5" />
            </Button>
          </div>
        ) : null}

        <Button
          className="gap-1.5"
          disabled={model.status === 'loading'}
          onClick={handleRefresh}
          size="sm"
          variant="outline"
        >
          <RefreshCw
            aria-hidden="true"
            className="size-3 data-[status=loading]:animate-spin"
            data-status={model.status}
          />
          Refresh
        </Button>
      </div>
    </div>
  );
}
