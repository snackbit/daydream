import type {
  AppSettings,
  ModelCapability,
  AiModelRegistry,
} from '@daydream/shared';
import { Field, FieldDescription, FieldLabel } from '@/components/ui/field';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import type { UpdateSettingsDraft } from '../settings-draft-updaters';
import {
  emptyRegistry,
  selectableModels,
  updateRegistry,
} from './model-registry';

export function ModelSelector({
  draft,
  feature,
  capability,
  label,
  onUpdateDraft,
}: {
  draft: AppSettings;
  feature: keyof AiModelRegistry['bindings'];
  capability: ModelCapability;
  label: string;
  onUpdateDraft: UpdateSettingsDraft;
}) {
  const registry = draft.ai.modelRegistry ?? emptyRegistry();
  const models = selectableModels(registry, capability);
  function change(value: string) {
    onUpdateDraft((current) =>
      updateRegistry(current, (registry) => ({
        ...registry,
        bindings: {
          ...registry.bindings,
          [feature]: value === 'none' ? null : value,
        },
      })),
    );
  }
  return (
    <Field>
      <FieldLabel htmlFor={`model-for-${feature}`}>{label}</FieldLabel>
      <Select
        value={registry.bindings[feature] ?? 'none'}
        onValueChange={change}
      >
        <SelectTrigger id={`model-for-${feature}`} className="w-full">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="none">Choose a model</SelectItem>
          {models.map((model) => (
            <SelectItem key={model.id} value={model.id}>
              {model.name} ·{' '}
              {
                registry.connections.find(
                  (connection) => connection.id === model.connectionId,
                )?.name
              }
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <FieldDescription>
        {models.length
          ? 'Connections and model tests are managed in Models.'
          : 'Add a model with this capability in Models first.'}
      </FieldDescription>
    </Field>
  );
}
