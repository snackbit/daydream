import { useId } from 'react';
import type { ChangeEvent } from 'react';
import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';

export interface TextSettingProps {
  label: string;
  maxLength: number;
  onChange: (value: string) => void;
  value: string;
}

export function TextSetting({
  label,
  maxLength,
  onChange,
  value,
}: TextSettingProps) {
  const id = useId();
  function handleChange(event: ChangeEvent<HTMLInputElement>) {
    onChange(event.target.value);
  }
  return (
    <Field>
      <FieldLabel htmlFor={id}>{label}</FieldLabel>
      <Input
        id={id}
        maxLength={maxLength}
        onChange={handleChange}
        value={value}
      />
    </Field>
  );
}
