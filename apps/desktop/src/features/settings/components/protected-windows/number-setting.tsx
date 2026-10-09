import { useId } from 'react';
import type { ChangeEvent } from 'react';
import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';

export interface NumberSettingProps {
  label: string;
  max: number;
  min: number;
  onChange: (value: number) => void;
  value: number;
}

export function NumberSetting({
  label,
  max,
  min,
  onChange,
  value,
}: NumberSettingProps) {
  const id = useId();
  function handleChange(event: ChangeEvent<HTMLInputElement>) {
    onChange(Math.round(Number(event.target.value)));
  }
  return (
    <Field>
      <FieldLabel htmlFor={id}>{label}</FieldLabel>
      <div className="flex items-center gap-2">
        <Input
          id={id}
          max={max}
          min={min}
          onChange={handleChange}
          step={250}
          type="number"
          value={value}
        />
        <span className="text-muted-foreground shrink-0 text-xs">ms</span>
      </div>
    </Field>
  );
}
