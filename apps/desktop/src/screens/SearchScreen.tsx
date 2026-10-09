import { useCallback, useState } from 'react';
import type { RecallSourceKind } from '@daydream/shared';
import { PageHeader, PageLayout } from '@/app/PageLayout';
import { Button } from '@/components/ui/button';
import { RecallReadState } from '@/features/recall/components/recall-read-state';
import {
  searchRecallEvidence,
  cancelRecallSearch,
  getRecallStatus,
} from '@/features/recall/recall-client';
import { recallTimezone } from '@/features/recall/recall-format';
import { SearchControls } from '@/features/recall/search/search-controls';
import type { RecallSearchLocation } from '@/features/recall/search/search-controls';
import { SearchResults } from '@/features/recall/search/search-results';
import { useRecallRead } from '@/features/recall/use-recall-read';

export interface SearchScreenProps {
  query: string;
  source: RecallSourceKind | 'all';
  date: string | null;
  onSearchChange: (search: RecallSearchLocation) => void;
}
async function readSearchPrivacyRevision(): Promise<number> {
  return (await getRecallStatus()).privacyRevision;
}
export function SearchScreen({
  query,
  source,
  date,
  onSearchChange,
}: SearchScreenProps) {
  const [mode, setMode] = useState<'hybrid' | 'lexical'>('hybrid');
  function changeSearch(location: RecallSearchLocation) {
    onSearchChange(location);
  }
  function changeMode(value: 'hybrid' | 'lexical') {
    setMode(value);
  }
  return (
    <PageLayout>
      <PageHeader
        title="Search"
        description="Return to the words, ideas, and moments you remember."
      />
      <SearchControls
        key={`${query}-${source}-${date}`}
        location={{ q: query, source, date }}
        onSearchChange={changeSearch}
        mode={mode}
        onModeChange={changeMode}
      />
      {!query.trim() ? (
        <div className="text-muted-foreground py-12 text-center text-sm">
          Start with a phrase or an idea. Your eligible sources stay within
          reach.
        </div>
      ) : (
        <SearchResultPages
          query={query}
          source={source}
          date={date}
          mode={mode}
        />
      )}
    </PageLayout>
  );
}

interface SearchResultPagesProps {
  query: string;
  source: RecallSourceKind | 'all';
  date: string | null;
  mode: 'hybrid' | 'lexical';
}
function SearchResultPages({
  query,
  source,
  date,
  mode,
}: SearchResultPagesProps) {
  const scope = `${query}-${source}-${date}-${mode}`;
  const [pageState, setPageState] = useState<{
    scope: string;
    cursors: (string | null)[];
  }>({ scope, cursors: [null] });
  const cursors = pageState.scope === scope ? pageState.cursors : [null];
  const cursor = cursors[cursors.length - 1] ?? null;
  const read = useCallback(
    () =>
      searchRecallEvidence({
        query,
        dayId: date,
        timezone: recallTimezone,
        sourceKinds: source === 'all' ? [] : [source],
        mode,
        cursor,
        limit: 30,
      }),
    [query, date, source, mode, cursor],
  );
  const { state, refresh } = useRecallRead(
    read,
    cancelRecallSearch,
    readSearchPrivacyRevision,
  );
  function nextPage() {
    if (state.status === 'ready' && state.data.nextCursor !== null)
      setPageState({ scope, cursors: [...cursors, state.data.nextCursor] });
  }
  function previousPage() {
    setPageState({ scope, cursors: cursors.slice(0, -1) });
  }
  function restartSearch() {
    if (cursor === null) refresh();
    else setPageState({ scope, cursors: [null] });
  }
  return (
    <>
      {state.status !== 'ready' ? (
        <RecallReadState status={state.status} onRetry={restartSearch} />
      ) : (
        <>
          <SearchResults result={state.data} />
          <div className="flex flex-wrap items-center justify-between gap-3">
            <Button
              disabled={cursors.length === 1}
              onClick={previousPage}
              variant="outline"
            >
              Previous results
            </Button>
            <span className="text-muted-foreground text-xs">
              Page {cursors.length}
            </span>
            <Button
              disabled={state.data.nextCursor === null}
              onClick={nextPage}
              variant="outline"
            >
              Next results
            </Button>
          </div>
        </>
      )}
    </>
  );
}
