import { useId } from 'react';
import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { formatPixels } from '@/features/recorder/scene-layout';

export interface PixelInputFieldProps {
  disabled?: boolean;
  label: string;
  onChange: (value: number) => void;
  value: number;
}

export function PixelInputField({
  disabled = false,
  label,
  onChange,
  value,
}: PixelInputFieldProps) {
  const id = useId();

  return (
    <Field>
      <FieldLabel htmlFor={id}>{label}</FieldLabel>
      <Input
        className="font-mono"
        disabled={disabled}
        id={id}
        inputMode="numeric"
        onChange={(event) => onChange(pixelInputValue(event.target.value))}
        type="number"
        value={formatPixels(value)}
      />
    </Field>
  );
}

function pixelInputValue(value: string): number {
  const parsed = Number.parseFloat(value);
  return Number.isFinite(parsed) ? parsed : 0;
}
