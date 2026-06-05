import type { AudioSource } from '@daydream/shared';
import { Field, FieldLabel } from '@/components/ui/field';
import {
  NativeSelect,
  NativeSelectOption,
} from '@/components/ui/native-select';

export interface AudioSourceSelectProps {
  label: string;
  onChange: (sourceId: string | null) => void;
  role: 'desktop-audio' | 'microphone';
  selectedSourceId: string | null;
  sources: AudioSource[];
}

export function AudioSourceSelect({
  label,
  onChange,
  role,
  selectedSourceId,
  sources,
}: AudioSourceSelectProps) {
  const roleSources = sources.filter((source) => source.roles.includes(role));

  return (
    <Field>
      <FieldLabel>{label}</FieldLabel>
      <NativeSelect
        className="w-full"
        onChange={(event) => onChange(event.target.value || null)}
        value={selectedSourceId ?? ''}
      >
        <NativeSelectOption value="">None</NativeSelectOption>
        {roleSources.map((source) => (
          <NativeSelectOption key={source.id} value={source.id}>
            {source.displayName}
            {source.isDefault ? ' (default)' : ''}
          </NativeSelectOption>
        ))}
      </NativeSelect>
    </Field>
  );
}
