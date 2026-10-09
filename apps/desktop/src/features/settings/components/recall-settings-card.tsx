import type { ChangeEvent } from 'react';
import type {
  AppSettings,
  AiModelAsset,
  RecallExternalProfile,
} from '@daydream/shared';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Field, FieldDescription, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import type { ModelTransferRowProps } from '@/features/ai/models/model-transfer-row';
import { ModelCacheRow } from './ai-model-cache-card';
import { CheckboxField } from './ai-settings-fields';
import { RecallProfileFields } from './recall-profile-fields';
import { RecallReadiness } from './recall-readiness';

export interface RecallSettingsCardProps {
  draft: AppSettings;
  models: AiModelAsset[];
  modelActionErrors?: Readonly<Record<string, string>> | undefined;
  onTransferAction?: ModelTransferRowProps['onTransferAction'];
  embeddingKeyDraft: string;
  generationKeyDraft: string;
  onEmbeddingKeyChange: (value: string) => void;
  onGenerationKeyChange: (value: string) => void;
  onUpdateDraft: (updater: (draft: AppSettings) => AppSettings) => void;
  onCancelModelDownload: (modelId: string) => void;
  onDeleteModel: (modelId: string) => void;
  onDownloadModel: (modelId: string) => void;
}

export function RecallSettingsCard(props: RecallSettingsCardProps) {
  const { draft, models, onUpdateDraft } = props;
  const recall = draft.ai.recall;
  function handleProvider(value: string) {
    if (value === 'local' || value === 'openai-compatible')
      onUpdateDraft((current) => ({
        ...current,
        ai: {
          ...current.ai,
          recall: {
            ...current.ai.recall,
            embedding: { ...current.ai.recall.embedding, provider: value },
          },
        },
      }));
  }
  function handleModel(value: string) {
    if (
      value === 'multilingual-e5-small-q8_0' ||
      value === 'bge-small-en-v1.5-q8_0'
    )
      onUpdateDraft((current) => ({
        ...current,
        ai: {
          ...current.ai,
          recall: {
            ...current.ai.recall,
            embedding: { ...current.ai.recall.embedding, localModelId: value },
          },
        },
      }));
  }
  function handleBackend(value: string) {
    if (value === 'cpu' || value === 'auto' || value === 'vulkan')
      onUpdateDraft((current) => ({
        ...current,
        ai: {
          ...current.ai,
          recall: {
            ...current.ai.recall,
            embedding: { ...current.ai.recall.embedding, localBackend: value },
          },
        },
      }));
  }
  function handleChooseCpu() {
    handleBackend('cpu');
  }
  function handleEmbedding(profile: RecallExternalProfile) {
    onUpdateDraft((current) => ({
      ...current,
      ai: {
        ...current.ai,
        recall: {
          ...current.ai.recall,
          embedding: { ...current.ai.recall.embedding, ...profile },
        },
      },
    }));
  }
  function handleDimensions(event: ChangeEvent<HTMLInputElement>) {
    const dimensions = Number(event.target.value);
    onUpdateDraft((current) => ({
      ...current,
      ai: {
        ...current.ai,
        recall: {
          ...current.ai.recall,
          embedding: { ...current.ai.recall.embedding, dimensions },
        },
      },
    }));
  }
  function handleConvention(value: string) {
    if (value === 'e5' || value === 'bge' || value === 'plain')
      onUpdateDraft((current) => ({
        ...current,
        ai: {
          ...current.ai,
          recall: {
            ...current.ai.recall,
            embedding: { ...current.ai.recall.embedding, convention: value },
          },
        },
      }));
  }
  function handleGeneration(profile: RecallExternalProfile) {
    onUpdateDraft((current) => ({
      ...current,
      ai: {
        ...current.ai,
        recall: { ...current.ai.recall, generation: profile },
      },
    }));
  }
  function handleAutoGenerate(autoGenerate: boolean) {
    onUpdateDraft((current) => ({
      ...current,
      ai: { ...current.ai, recall: { ...current.ai.recall, autoGenerate } },
    }));
  }
  return (
    <div className="grid gap-6">
      <RecallReadiness
        settings={recall}
        models={models}
        onDownloadModel={props.onDownloadModel}
        onChooseCpu={handleChooseCpu}
      />
      <Card id="ai-recall-embedding" tabIndex={-1}>
        <CardHeader>
          <CardTitle>Recall and semantic search</CardTitle>
          <CardDescription>
            Index eligible OCR and both transcript tracks. Local embeddings keep
            captured content on this device.
          </CardDescription>
        </CardHeader>
        <CardContent className="grid gap-6">
          <Field>
            <FieldLabel htmlFor="recall-provider">
              Embedding provider
            </FieldLabel>
            <Select
              value={recall.embedding.provider}
              onValueChange={handleProvider}
            >
              <SelectTrigger
                id="recall-provider"
                className="w-full sm:max-w-sm"
              >
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="local">App-managed local</SelectItem>
                <SelectItem value="openai-compatible">
                  OpenAI-compatible API
                </SelectItem>
              </SelectContent>
            </Select>
          </Field>
          {recall.embedding.provider === 'local' ? (
            <>
              <div className="grid gap-5 sm:grid-cols-2">
                <Field>
                  <FieldLabel htmlFor="recall-local-model">
                    Embedding model
                  </FieldLabel>
                  <Select
                    value={recall.embedding.localModelId}
                    onValueChange={handleModel}
                  >
                    <SelectTrigger id="recall-local-model" className="w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="multilingual-e5-small-q8_0">
                        Multilingual E5 Small Q8_0
                      </SelectItem>
                      <SelectItem value="bge-small-en-v1.5-q8_0">
                        BGE Small English v1.5 Q8_0
                      </SelectItem>
                    </SelectContent>
                  </Select>
                  <FieldDescription>
                    E5 supports multilingual recall; BGE targets English.
                  </FieldDescription>
                </Field>
                <Field>
                  <FieldLabel htmlFor="recall-backend">
                    Embedding backend
                  </FieldLabel>
                  <Select
                    value={recall.embedding.localBackend}
                    onValueChange={handleBackend}
                  >
                    <SelectTrigger id="recall-backend" className="w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="cpu">CPU</SelectItem>
                      <SelectItem value="auto">Auto</SelectItem>
                      <SelectItem value="vulkan">Vulkan</SelectItem>
                    </SelectContent>
                  </Select>
                  <FieldDescription>
                    Auto tries Vulkan then CPU. Vulkan requires a compatible
                    build and device.
                  </FieldDescription>
                </Field>
              </div>
              {models
                .filter((model) => model.provider === 'local-embedding')
                .map((model) => (
                  <ModelCacheRow
                    key={model.id}
                    model={model}
                    actionError={props.modelActionErrors?.[model.id]}
                    onTransferAction={props.onTransferAction}
                    onCancelModelDownload={props.onCancelModelDownload}
                    onDeleteModel={props.onDeleteModel}
                    onDownloadModel={props.onDownloadModel}
                  />
                ))}
              <p className="text-muted-foreground text-xs">
                Model downloads contact the model host. Captured content stays
                local.
              </p>
            </>
          ) : (
            <>
              <div className="grid gap-5 sm:grid-cols-2">
                <Field>
                  <FieldLabel htmlFor="recall-dimensions">
                    Embedding dimensions
                  </FieldLabel>
                  <Input
                    id="recall-dimensions"
                    type="number"
                    min={1}
                    max={8192}
                    value={recall.embedding.dimensions}
                    onChange={handleDimensions}
                  />
                  <FieldDescription>
                    Must match the provider model output.
                  </FieldDescription>
                </Field>
                <Field>
                  <FieldLabel htmlFor="recall-convention">
                    Embedding convention
                  </FieldLabel>
                  <Select
                    value={recall.embedding.convention}
                    onValueChange={handleConvention}
                  >
                    <SelectTrigger id="recall-convention" className="w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="plain">Plain</SelectItem>
                      <SelectItem value="e5">
                        E5 query/passage prefixes
                      </SelectItem>
                      <SelectItem value="bge">BGE query instruction</SelectItem>
                    </SelectContent>
                  </Select>
                </Field>
              </div>
              <RecallProfileFields
                kind="embedding"
                settings={recall}
                keyDraft={props.embeddingKeyDraft}
                onKeyChange={props.onEmbeddingKeyChange}
                onChange={handleEmbedding}
              />
            </>
          )}
        </CardContent>
      </Card>
      <Card id="ai-recall-generation" tabIndex={-1}>
        <CardHeader>
          <CardTitle>Day summaries</CardTitle>
          <CardDescription>
            A separately enabled text provider creates grounded summaries and
            suggestions. Tasks and feedback stay local.
          </CardDescription>
        </CardHeader>
        <CardContent className="grid gap-6">
          <RecallProfileFields
            kind="generation"
            settings={recall}
            keyDraft={props.generationKeyDraft}
            onKeyChange={props.onGenerationKeyChange}
            onChange={handleGeneration}
          />
          <CheckboxField
            checked={recall.autoGenerate}
            label="Generate automatically"
            description="Generate after five quiet minutes. While recording, each Day is generated at most once every fifteen minutes."
            onCheckedChange={handleAutoGenerate}
          />
        </CardContent>
      </Card>
    </div>
  );
}
