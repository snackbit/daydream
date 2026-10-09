import { useState, type ChangeEvent } from 'react';
import type { RecallFeedback, SetRecallFeedbackInput } from '@daydream/shared';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { setRecallFeedback } from '../recall-client';

export interface DayFeedbackProps {
  dayId: string;
  saved: RecallFeedback | undefined;
  onAction: (action: () => Promise<unknown>) => void;
  busy: boolean;
}
export function DayFeedback({
  dayId,
  saved,
  onAction,
  busy,
}: DayFeedbackProps) {
  const [comment, setComment] = useState(saved?.comment ?? '');
  function changeComment(event: ChangeEvent<HTMLInputElement>) {
    setComment(event.target.value);
  }
  function saveRating(rating: SetRecallFeedbackInput['rating']) {
    onAction(() =>
      setRecallFeedback({ dayId, targetId: dayId, rating, comment }),
    );
  }
  function helpful() {
    saveRating('helpful');
  }
  function unhelpful() {
    saveRating('unhelpful');
  }
  function saveComment() {
    saveRating(saved?.rating ?? null);
  }
  return (
    <div className="border-border mt-6 grid gap-3 border-t pt-5">
      <p className="text-muted-foreground text-xs">
        Your feedback stays on this device.
      </p>
      <div className="flex flex-wrap gap-2">
        <Button
          aria-pressed={saved?.rating === 'helpful'}
          disabled={busy}
          onClick={helpful}
          variant="outline"
        >
          Helpful
        </Button>
        <Button
          aria-pressed={saved?.rating === 'unhelpful'}
          disabled={busy}
          onClick={unhelpful}
          variant="outline"
        >
          Needs work
        </Button>
      </div>
      <Label htmlFor="day-feedback">Feedback or correction</Label>
      <Input
        id="day-feedback"
        disabled={busy}
        value={comment}
        onChange={changeComment}
        placeholder="What should this overview say?"
      />
      <Button
        className="w-fit"
        disabled={busy}
        onClick={saveComment}
        variant="outline"
      >
        Save feedback
      </Button>
    </div>
  );
}
