import { useId, useState, type ChangeEvent, type KeyboardEvent } from 'react';
import { Input } from '@/components/ui/input';

export interface NumberPreferenceProps {
  label: string;
  value: number;
  min: number;
  max?: number | undefined;
  step?: number | undefined;
  unit?: string | undefined;
  onCommit: (value: number) => void;
}
export function NumberPreference({ ...props }: NumberPreferenceProps) {
  return <NumberPreferenceInput key={props.value} {...props} />;
}
function NumberPreferenceInput({
  label,
  value,
  min,
  max,
  step = 1,
  unit,
  onCommit,
}: NumberPreferenceProps) {
  const id = useId();
  const [text, setText] = useState(String(value));
  const [error, setError] = useState<string | null>(null);
  function change(event: ChangeEvent<HTMLInputElement>) {
    setText(event.target.value);
    setError(null);
  }
  function commit() {
    const next = text.trim() ? Number(text) : NaN;
    if (
      Number.isInteger(step) &&
      !Number.isInteger(next) &&
      Number.isFinite(next)
    ) {
      setError('Enter a whole number.');
      return;
    }
    if (
      !Number.isFinite(next) ||
      next < min ||
      (max !== undefined && next > max)
    ) {
      setError(
        max !== undefined
          ? `Enter a value from ${min} to ${max}.`
          : `Enter ${min} or more.`,
      );
      return;
    }
    setError(null);
    if (next !== value) {
      onCommit(next);
      setText(String(value));
    }
  }
  function key(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === 'Enter') {
      event.preventDefault();
      event.currentTarget.blur();
    }
    if (event.key === 'Escape') {
      setText(String(value));
      setError(null);
    }
  }
  return (
    <div className="grid gap-1.5">
      <div className="flex items-center gap-2">
        <Input
          aria-label={label}
          aria-invalid={Boolean(error)}
          aria-describedby={error ? id : undefined}
          className="w-24 text-right"
          type="number"
          value={text}
          min={min}
          max={max}
          step={step}
          onChange={change}
          onBlur={commit}
          onKeyDown={key}
        />
        {unit ? (
          <span className="text-muted-foreground text-xs">{unit}</span>
        ) : null}
      </div>
      {error ? (
        <p id={id} role="alert" className="text-destructive max-w-56 text-xs">
          {error}
        </p>
      ) : null}
    </div>
  );
}
