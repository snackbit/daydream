import { useCallback } from 'react';
import { Filter } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';

export type ReviewCategory = 'speech' | 'observation' | 'activity';
export interface ReviewFiltersProps {
  enabled: ReadonlySet<ReviewCategory>;
  onChange: (category: ReviewCategory, enabled: boolean) => void;
}
export function ReviewFilters({ enabled, onChange }: ReviewFiltersProps) {
  const speech = useCallback(
    (value: boolean | 'indeterminate') => onChange('speech', value === true),
    [onChange],
  );
  const observation = useCallback(
    (value: boolean | 'indeterminate') =>
      onChange('observation', value === true),
    [onChange],
  );
  const activity = useCallback(
    (value: boolean | 'indeterminate') => onChange('activity', value === true),
    [onChange],
  );
  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button size="sm" variant="ghost" aria-label="Review filters">
          <Filter />
          Show
        </Button>
      </PopoverTrigger>
      <PopoverContent
        data-studio-preserve-selection
        className="grid w-56 gap-3"
      >
        <label className="flex items-center gap-3">
          <Checkbox checked={enabled.has('speech')} onCheckedChange={speech} />
          Speech
        </label>
        <label className="flex items-center gap-3">
          <Checkbox
            checked={enabled.has('observation')}
            onCheckedChange={observation}
          />
          Screen changes
        </label>
        <label className="flex items-center gap-3">
          <Checkbox
            checked={enabled.has('activity')}
            onCheckedChange={activity}
          />
          Activity
        </label>
      </PopoverContent>
    </Popover>
  );
}
