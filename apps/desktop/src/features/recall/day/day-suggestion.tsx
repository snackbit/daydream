import type { RecallSuggestion } from '@daydream/shared';
import { Button } from '@/components/ui/button';
import { EvidenceSourceLink } from '../components/evidence-source-link';
import {
  acceptRecallSuggestion,
  dismissRecallSuggestion,
} from '../recall-client';

export interface DaySuggestionProps {
  suggestion: RecallSuggestion;
  dayId: string;
  busy: boolean;
  onAction: (action: () => Promise<unknown>) => void;
}
export function DaySuggestion({
  suggestion,
  dayId,
  busy,
  onAction,
}: DaySuggestionProps) {
  function accept() {
    onAction(() =>
      acceptRecallSuggestion({ dayId, suggestionId: suggestion.id }),
    );
  }
  function dismiss() {
    onAction(() =>
      dismissRecallSuggestion({ dayId, suggestionId: suggestion.id }),
    );
  }
  return (
    <article className="border-border grid gap-3 border-t py-5">
      <h3 className="text-sm font-medium">{suggestion.title}</h3>
      <p className="text-muted-foreground text-sm leading-6">
        {suggestion.description}
      </p>
      <div className="flex flex-wrap gap-3">
        {suggestion.citations.map((citation) => (
          <EvidenceSourceLink key={citation.evidenceId} citation={citation} />
        ))}
      </div>
      <div className="flex flex-wrap gap-2">
        <Button disabled={busy} onClick={accept} variant="outline">
          Accept as local task
        </Button>
        <Button disabled={busy} onClick={dismiss} variant="ghost">
          Dismiss
        </Button>
      </div>
    </article>
  );
}
