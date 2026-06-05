import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { formatPixels } from '@/features/recorder/scene-layout';

export interface PixelInputFieldProps {
  label: string;
  onChange: (value: number) => void;
  value: number;
}

export function PixelInputField({
  label,
  onChange,
  value,
}: PixelInputFieldProps) {
  return (
    <Field>
      <FieldLabel>{label}</FieldLabel>
      <Input
        className="font-mono"
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
