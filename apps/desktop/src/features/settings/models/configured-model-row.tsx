import type {
  AiConfiguredModel,
  AiModelConnection,
  AiModelRegistry,
  ModelCapability,
} from '@daydream/shared';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from '@/components/ui/select';
import { selectedFeatures } from './model-registry';
import { ModelTest } from './model-test';

export function ConfiguredModelRow({
  model,
  connection,
  registry,
  keyDraft,
  onUpdate,
  onRemove,
}: {
  model: AiConfiguredModel;
  connection: AiModelConnection;
  registry: AiModelRegistry;
  keyDraft: string;
  onUpdate: (update: Partial<AiConfiguredModel>) => void;
  onRemove: () => void;
}) {
  const uses = selectedFeatures(registry, model.id);
  function capability(capability: ModelCapability, checked: boolean) {
    let capabilities = checked
      ? [...model.capabilities, capability]
      : model.capabilities.filter((value) => value !== capability);
    if (capability === 'vision' && checked && !capabilities.includes('text'))
      capabilities.push('text');
    if (capability === 'text' && !checked)
      capabilities = capabilities.filter((value) => value !== 'vision');
    onUpdate({ capabilities });
  }
  return (
    <section
      className="border-border grid gap-4 rounded-lg border p-4"
      aria-label={`Model ${model.name}`}
    >
      <div className="grid gap-4 sm:grid-cols-2">
        <Field>
          <FieldLabel htmlFor={`model-name-${model.id}`}>
            Model label
          </FieldLabel>
          <Input
            id={`model-name-${model.id}`}
            value={model.name}
            onChange={(event) => onUpdate({ name: event.target.value })}
          />
        </Field>
        <Field>
          <FieldLabel htmlFor={`model-id-${model.id}`}>
            Model identifier
          </FieldLabel>
          <Input
            id={`model-id-${model.id}`}
            value={model.model}
            placeholder="Model name from the provider"
            onChange={(event) => onUpdate({ model: event.target.value })}
          />
        </Field>
      </div>
      <div className="flex flex-wrap gap-5" aria-label="Model capabilities">
        {(['text', 'vision', 'embedding'] as const).map((value) => {
          const selected = uses.some((feature) =>
            feature === 'summary'
              ? value === 'text'
              : feature === 'vision'
                ? value === 'vision' || value === 'text'
                : feature === value,
          );
          return (
            <Field key={value} orientation="horizontal">
              <Checkbox
                id={`cap-${model.id}-${value}`}
                checked={model.capabilities.includes(value)}
                disabled={
                  selected ||
                  (connection.authMode === 'chatgpt' && value === 'embedding')
                }
                onCheckedChange={(checked) =>
                  capability(value, checked === true)
                }
              />
              <FieldLabel htmlFor={`cap-${model.id}-${value}`}>
                {value === 'text'
                  ? 'Text generation'
                  : value === 'vision'
                    ? 'Vision'
                    : 'Embeddings'}
              </FieldLabel>
            </Field>
          );
        })}
      </div>
      {connection.authMode === 'chatgpt' ? (
        <p className="text-muted-foreground text-xs">
          Embeddings require a local model or an API connection.
        </p>
      ) : null}
      {model.capabilities.includes('embedding') ? (
        <div className="grid gap-4 sm:grid-cols-2">
          <Field>
            <FieldLabel htmlFor={`dimensions-${model.id}`}>
              Embedding dimensions
            </FieldLabel>
            <Input
              id={`dimensions-${model.id}`}
              type="number"
              min={1}
              max={8192}
              value={model.embeddingDimensions ?? ''}
              placeholder="Detected by test"
              onChange={(event) =>
                onUpdate({
                  embeddingDimensions: event.target.value
                    ? Number(event.target.value)
                    : null,
                })
              }
            />
          </Field>
          <Field>
            <FieldLabel htmlFor={`convention-${model.id}`}>
              Embedding input convention
            </FieldLabel>
            <Select
              value={model.embeddingConvention}
              onValueChange={(value) => {
                if (value === 'plain' || value === 'e5' || value === 'bge')
                  onUpdate({ embeddingConvention: value });
              }}
            >
              <SelectTrigger id={`convention-${model.id}`}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="plain">Plain</SelectItem>
                <SelectItem value="e5">E5 query/passage prefixes</SelectItem>
                <SelectItem value="bge">BGE query instruction</SelectItem>
              </SelectContent>
            </Select>
          </Field>
        </div>
      ) : null}
      <ModelTest
        connection={connection}
        model={model}
        keyDraft={keyDraft}
        onDimensions={(dimensions) =>
          onUpdate({ embeddingDimensions: dimensions })
        }
      />
      <div className="flex flex-wrap items-center gap-3">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          disabled={uses.length > 0}
          onClick={onRemove}
        >
          Remove model
        </Button>
        {uses.length ? (
          <span className="text-muted-foreground text-xs">
            Used by {uses.join(', ')}. Choose another model there before
            removing this one.
          </span>
        ) : null}
      </div>
    </section>
  );
}
