import { Link } from '@tanstack/react-router';
import type { RecallSearchResult } from '@daydream/shared';
import { Badge } from '@/components/ui/badge';
import { EvidenceSourceLink } from '../components/evidence-source-link';

export interface SearchResultsProps {
  result: RecallSearchResult;
}
export function SearchResults({ result }: SearchResultsProps) {
  return (
    <section aria-label="Search results" className="grid gap-6">
      <div className="flex flex-wrap items-center gap-3">
        <h2 className="text-lg">Source moments</h2>
        <Badge variant="outline">
          {result.semanticUsed ? 'Semantic + literal' : 'Literal matches'}
        </Badge>
        <p className="text-muted-foreground text-xs">
          {result.indexedEvidenceCount} indexed / {result.eligibleEvidenceCount}{' '}
          eligible moments
        </p>
      </div>
      {!result.complete ? (
        <p role="status" className="bg-muted rounded-2xl p-4 text-sm">
          Search coverage is incomplete. More eligible evidence may still need
          indexing.{' '}
          <Link to="/processing" className="underline">
            Inspect processing
          </Link>
        </p>
      ) : null}
      {result.unavailableReason !== null ? (
        <p role="status" className="text-muted-foreground text-sm">
          Semantic search is unavailable. Literal results remain usable.{' '}
          <Link
            to="/settings"
            search={{ tab: 'ai', focus: 'recall' }}
            className="underline"
          >
            Review recall setup
          </Link>
        </p>
      ) : null}
      {result.hits.length === 0 ? (
        <div className="bg-muted rounded-[20px] p-8">
          <h3 className="text-xl font-light">No eligible matches found.</h3>
          <p className="text-muted-foreground mt-3 text-sm">
            Try different words or broaden the date and source filters.
            Protected or hidden content is excluded.
          </p>
        </div>
      ) : (
        result.hits.map(({ evidence }) => (
          <article
            key={evidence.evidenceId}
            className="border-border grid gap-4 rounded-[20px] border p-6"
          >
            <p className="text-muted-foreground text-xs">
              {new Date(evidence.absoluteStartMs).toLocaleDateString(
                undefined,
                { month: 'long', day: 'numeric', year: 'numeric' },
              )}
            </p>
            <p className="text-sm leading-7 break-words whitespace-pre-wrap">
              {evidence.text}
            </p>
            <EvidenceSourceLink citation={evidence} />
          </article>
        ))
      )}
      <p className="text-muted-foreground text-xs">
        Results preserve source wording. Semantic matches may use different
        words from your query. Opening a source does not start playback.
      </p>
    </section>
  );
}
