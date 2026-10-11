import type { AppSettings, AiModelAsset } from '@daydream/shared';
import { SharedDisclosure } from '@/components/shared-disclosure';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { RecallReadiness } from '../../components/recall-readiness';
import { ModelSelector } from '../../models/model-selector';
import type { UpdateSettingsDraft } from '../../settings-draft-updaters';
import { SettingRow } from '../controls/setting-row';

export function RecallPreferences({
  kind,
  draft,
  models,
  onUpdate,
  onDownloadModel,
}: {
  kind: 'summary' | 'search';
  draft: AppSettings;
  models: AiModelAsset[];
  onUpdate: UpdateSettingsDraft;
  onDownloadModel: (modelId: string) => void;
}) {
  const recall = draft.ai.recall;
  function generation(enabled: boolean) {
    onUpdate((current) => ({
      ...current,
      ai: {
        ...current.ai,
        recall: {
          ...current.ai.recall,
          generation: { ...current.ai.recall.generation, enabled },
        },
      },
    }));
  }
  function automatic(autoGenerate: boolean) {
    onUpdate((current) => ({
      ...current,
      ai: { ...current.ai, recall: { ...current.ai.recall, autoGenerate } },
    }));
  }
  function searchMode(value: string) {
    if (value === 'local' || value === 'remote' || value === 'off')
      onUpdate((current) => ({
        ...current,
        ai: {
          ...current.ai,
          ...(value === 'local' && current.ai.modelRegistry
            ? {
                modelRegistry: {
                  ...current.ai.modelRegistry,
                  bindings: {
                    ...current.ai.modelRegistry.bindings,
                    embedding: null,
                  },
                },
              }
            : {}),
          recall: {
            ...current.ai.recall,
            embedding: {
              ...current.ai.recall.embedding,
              provider: value === 'local' ? 'local' : 'openai-compatible',
              enabled: value !== 'off',
            },
          },
        },
      }));
  }
  function localModel(localModelId: string) {
    if (
      localModelId === 'multilingual-e5-small-q8_0' ||
      localModelId === 'bge-small-en-v1.5-q8_0'
    )
      onUpdate((current) => ({
        ...current,
        ai: {
          ...current.ai,
          recall: {
            ...current.ai.recall,
            embedding: { ...current.ai.recall.embedding, localModelId },
          },
        },
      }));
  }
  function backend(value: string) {
    if (value === 'cpu' || value === 'auto' || value === 'vulkan')
      onUpdate((current) => ({
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
  if (kind === 'summary')
    return (
      <div className="grid gap-6">
        <ModelSelector
          draft={draft}
          feature="summary"
          capability="text"
          label="Summary model"
          onUpdateDraft={onUpdate}
        />
        <SettingRow
          title="Day summaries"
          description="Send recognized text and transcripts to this model. Screen images stay local."
        >
          <Switch
            aria-label="Enable day summaries"
            checked={recall.generation.enabled}
            onCheckedChange={generation}
          />
        </SettingRow>
        <SettingRow
          title="Generate automatically"
          description="After five quiet minutes; at most every fifteen minutes while recording."
        >
          <Switch
            aria-label="Generate automatically"
            checked={recall.autoGenerate}
            onCheckedChange={automatic}
          />
        </SettingRow>
      </div>
    );
  const localModels = models.filter(
    (model) => model.provider === 'local-embedding',
  );
  return (
    <div className="grid gap-6">
      <SettingRow
        title="Semantic search"
        description="Keyword search remains available with semantic search off."
      >
        <Select
          value={
            !recall.embedding.enabled && recall.embedding.provider !== 'local'
              ? 'off'
              : recall.embedding.provider === 'local'
                ? 'local'
                : 'remote'
          }
          onValueChange={searchMode}
        >
          <SelectTrigger aria-label="Search processing" className="w-40">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="local">Local</SelectItem>
            <SelectItem value="remote">Remote</SelectItem>
            <SelectItem value="off">Off</SelectItem>
          </SelectContent>
        </Select>
      </SettingRow>
      {recall.embedding.provider === 'local' ? (
        <div className="grid gap-2">
          <label className="text-sm" htmlFor="search-local-model">
            Local embedding model
          </label>
          <Select
            value={recall.embedding.localModelId}
            onValueChange={localModel}
          >
            <SelectTrigger id="search-local-model" className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {localModels.map((model) => (
                <SelectItem key={model.id} value={model.id}>
                  {model.label}
                  {model.downloadStatus === 'available'
                    ? ''
                    : ' · Download needed'}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <p className="text-muted-foreground text-xs">
            Manage downloads in Models & accounts.
          </p>
          {draft.expert?.enabled ? (
            <SharedDisclosure label="Advanced">
              <SettingRow title="Embedding backend">
                <Select
                  value={recall.embedding.localBackend}
                  onValueChange={backend}
                >
                  <SelectTrigger
                    aria-label="Embedding backend"
                    className="w-36"
                  >
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="cpu">CPU</SelectItem>
                    <SelectItem value="auto">Auto</SelectItem>
                    <SelectItem value="vulkan">Vulkan</SelectItem>
                  </SelectContent>
                </Select>
              </SettingRow>
            </SharedDisclosure>
          ) : null}
        </div>
      ) : recall.embedding.enabled ? (
        <>
          <ModelSelector
            draft={draft}
            feature="embedding"
            capability="embedding"
            label="Embedding model"
            onUpdateDraft={onUpdate}
          />
          <p className="text-muted-foreground text-sm">
            Recognized text, transcripts and search queries are sent to this
            model.
          </p>
        </>
      ) : null}
      <SharedDisclosure label="Saved configuration">
        <RecallReadiness
          settings={recall}
          models={models}
          onDownloadModel={onDownloadModel}
          onChooseCpu={() => backend('cpu')}
        />
      </SharedDisclosure>
    </div>
  );
}
