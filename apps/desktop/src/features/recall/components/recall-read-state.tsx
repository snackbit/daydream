import { Button } from '@/components/ui/button';

/** Loading and recovery never retain previous private source content. */
export interface RecallReadStateProps {
  status: 'loading' | 'invalidated' | 'error';
  onRetry: () => void;
}
export function RecallReadState({ status, onRetry }: RecallReadStateProps) {
  return (
    <div
      className="bg-muted rounded-[20px] p-6"
      role={status === 'error' ? 'alert' : 'status'}
    >
      <p>
        {status === 'error'
          ? 'Recall could not be loaded.'
          : status === 'invalidated'
            ? 'Source eligibility changed. Private results have been cleared.'
            : 'Loading local text and transcripts…'}
      </p>
      {status === 'error' || status === 'invalidated' ? (
        <Button className="mt-4" onClick={onRetry} variant="outline">
          {status === 'invalidated' ? 'Refresh eligible sources' : 'Try again'}
        </Button>
      ) : null}
    </div>
  );
}
