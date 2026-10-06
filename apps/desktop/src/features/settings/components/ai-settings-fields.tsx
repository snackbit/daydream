import { useId } from 'react';
import type { AiModelAsset } from '@daydream/shared';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Checkbox } from '@/components/ui/checkbox';
import { Field, FieldDescription, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { Slider } from '@/components/ui/slider';

export function RemoteProviderFields({
  baseUrl,
  customDisclosure,
  dataSentLabel,
  hasSavedKey,
  keyDraft,
  localAlternativeDescription,
  onBaseUrlChange,
  onKeyChange,
  prefix,
}: {
  baseUrl: string;
  customDisclosure?: string;
  dataSentLabel: string;
  hasSavedKey: boolean;
  keyDraft: string;
  localAlternativeDescription: string;
  onBaseUrlChange: (value: string) => void;
  onKeyChange: (value: string) => void;
  prefix: string;
}) {
  return (
    <>
      <Alert>
        <AlertTitle>Remote provider selected</AlertTitle>
        <AlertDescription>
          {customDisclosure ??
            `${dataSentLabel} will be sent to the configured provider when you process a recording.`}{' '}
          {localAlternativeDescription} API keys are encrypted locally.
        </AlertDescription>
      </Alert>
      <Field>
        <FieldLabel htmlFor={`ai-${prefix}-base-url`}>Base URL</FieldLabel>
        <Input
          id={`ai-${prefix}-base-url`}
          onChange={(event) => onBaseUrlChange(event.target.value)}
          placeholder="https://..."
          value={baseUrl}
        />
      </Field>
      <Field>
        <FieldLabel htmlFor={`ai-${prefix}-api-key`}>API key</FieldLabel>
        <Input
          autoComplete="off"
          id={`ai-${prefix}-api-key`}
          onChange={(event) => onKeyChange(event.target.value)}
          placeholder={hasSavedKey ? 'Saved key present' : 'Paste API key'}
          type="password"
          value={keyDraft}
        />
        <FieldDescription>
          {keyDraft.trim()
            ? 'New key, not saved.'
            : hasSavedKey
              ? 'Key saved.'
              : 'Enter an API key.'}
        </FieldDescription>
      </Field>
    </>
  );
}

export function CheckboxField({
  checked,
  description,
  label,
  onCheckedChange,
}: {
  checked: boolean;
  description: string;
  label: string;
  onCheckedChange: (checked: boolean) => void;
}) {
  const id = useId();

  return (
    <Field orientation="horizontal">
      <Checkbox
        checked={checked}
        id={id}
        onCheckedChange={(value) => onCheckedChange(value === true)}
      />
      <div className="min-w-0 flex-1">
        <FieldLabel
          className="flex w-full cursor-pointer flex-col items-start gap-0.5 text-left"
          htmlFor={id}
        >
          <span>{label}</span>
          <span className="text-muted-foreground text-left text-xs/relaxed leading-normal font-normal">
            {description}
          </span>
        </FieldLabel>
      </div>
    </Field>
  );
}

export function NumberField({
  label,
  max,
  min,
  onChange,
  step = 1,
  suffix,
  value,
}: {
  label: string;
  max: number;
  min: number;
  onChange: (value: number) => void;
  step?: number;
  suffix?: string;
  value: number;
}) {
  return (
    <Field>
      <FieldLabel>{label}</FieldLabel>
      <div className="flex max-w-sm items-center gap-2">
        <Input
          max={max}
          min={min}
          onChange={(event) => onChange(Math.round(Number(event.target.value)))}
          step={step}
          type="number"
          value={value}
        />
        {suffix ? (
          <span className="text-muted-foreground shrink-0 text-xs">
            {suffix}
          </span>
        ) : null}
      </div>
      <FieldDescription>
        Range: {min}-{max}
        {suffix ? ` ${suffix}` : ''}.
      </FieldDescription>
    </Field>
  );
}

export function SliderField({
  formatValue,
  label,
  max,
  min,
  onChange,
  rangeLabel,
  step = 1,
  stepLabel,
  suffix,
  value,
}: {
  formatValue?: (value: number) => string;
  label: string;
  max: number;
  min: number;
  onChange: (value: number) => void;
  rangeLabel?: string;
  step?: number;
  stepLabel?: string;
  suffix?: string;
  value: number;
}) {
  const displayValue = formatValue
    ? formatValue(value)
    : formatNumberWithSuffix(value, suffix);
  const rangeDescription =
    rangeLabel ??
    `${formatNumberWithSuffix(min, suffix)} to ${formatNumberWithSuffix(max, suffix)}`;
  const stepDescription = stepLabel ?? formatNumberWithSuffix(step, suffix);

  return (
    <Field>
      <div className="flex max-w-sm items-center justify-between gap-3">
        <FieldLabel>{label}</FieldLabel>
        <span className="text-muted-foreground text-xs tabular-nums">
          {displayValue}
        </span>
      </div>
      <Slider
        aria-label={label}
        className="max-w-sm"
        max={max}
        min={min}
        onValueChange={(values) => {
          const rawValue = values[0] ?? min;
          const steppedValue = Math.round(rawValue / step) * step;
          const clampedValue = Math.min(max, Math.max(min, steppedValue));
          onChange(clampedValue);
        }}
        step={step}
        value={[value]}
      />
      <FieldDescription>
        Range: {rangeDescription}. Step: {stepDescription}.
      </FieldDescription>
    </Field>
  );
}

function formatNumberWithSuffix(value: number, suffix: string | undefined) {
  const formatted = Number.isInteger(value)
    ? value.toString()
    : value.toFixed(2).replace(/\.?0+$/, '');

  return suffix ? `${formatted} ${suffix}` : formatted;
}

export function modelStatusLabel(model: AiModelAsset | undefined): string {
  if (!model) {
    return 'missing';
  }

  return `${model.downloadStatus}; smoke test ${model.selfTestStatus}`;
}
