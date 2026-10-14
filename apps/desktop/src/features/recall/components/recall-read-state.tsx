import { Button } from '@/components/ui/button';

/** Loading and recovery never retain previous private source content. */
export interface RecallReadStateProps {
  status: 'loading' | 'invalidated' | 'error';
  onRetry: () => void;
  loadingLabel?: string;
  errorLabel?: string;
}
export function RecallReadState({
  status,
  onRetry,
  loadingLabel = 'Loading local text and transcripts…',
  errorLabel = 'Recall could not be loaded.',
}: RecallReadStateProps) {
  return (
    <div
      className="bg-muted rounded-[20px] p-6"
      role={status === 'error' ? 'alert' : 'status'}
    >
      <p>
        {status === 'error'
          ? errorLabel
          : status === 'invalidated'
            ? 'Source eligibility changed. Private results have been cleared.'
            : loadingLabel}
      </p>
      {status === 'error' || status === 'invalidated' ? (
        <Button className="mt-4" onClick={onRetry} variant="outline">
          {status === 'invalidated' ? 'Refresh eligible sources' : 'Try again'}
        </Button>
      ) : null}
    </div>
  );
}
