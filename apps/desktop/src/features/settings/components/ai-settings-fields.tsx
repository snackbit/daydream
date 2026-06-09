import type { AiModelAsset } from '@daydream/shared';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Checkbox } from '@/components/ui/checkbox';
import { Field, FieldDescription, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';

export function RemoteProviderFields({
  baseUrl,
  hasSavedKey,
  keyDraft,
  onBaseUrlChange,
  onKeyChange,
  prefix,
}: {
  baseUrl: string;
  hasSavedKey: boolean;
  keyDraft: string;
  onBaseUrlChange: (value: string) => void;
  onKeyChange: (value: string) => void;
  prefix: string;
}) {
  return (
    <>
      <Alert variant="destructive">
        <AlertTitle>Remote provider</AlertTitle>
        <AlertDescription>
          Remote processing can send private recordings and derived text outside
          this device in later gates. API keys are encrypted locally.
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
          {hasSavedKey
            ? 'A key is saved. Enter a new key to replace it.'
            : 'No key is saved for this provider.'}
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
  return (
    <Field orientation="horizontal">
      <Checkbox
        checked={checked}
        onCheckedChange={(value) => onCheckedChange(value === true)}
      />
      <div>
        <FieldLabel>{label}</FieldLabel>
        <FieldDescription>{description}</FieldDescription>
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

export function modelStatusLabel(model: AiModelAsset | undefined): string {
  if (!model) {
    return 'missing';
  }

  return `${model.downloadStatus}; smoke test ${model.selfTestStatus}`;
}
