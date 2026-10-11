import type { AiModelAsset, AppSettings } from '@daydream/shared';
import {
  ChevronRight,
  FileText,
  Search,
  AudioLines,
  ScanText,
  Image,
  Settings2,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import type { UpdateSettingsDraft } from '../../settings-draft-updaters';
import type { SettingsDetail } from '../../settings-location';
import { SettingRow } from '../controls/setting-row';

export const featureNames = {
  summary: 'Day summaries',
  search: 'Search',
  transcription: 'Transcription',
  ocr: 'Text recognition',
  observations: 'Screen observations',
  processing: 'Processing',
  protection: 'Protected windows',
} satisfies Record<SettingsDetail, string>;
function selectedModel(
  draft: AppSettings,
  binding: 'summary' | 'embedding' | 'vision',
) {
  const registry = draft.ai.modelRegistry;
  return (
    registry?.models.find((model) => model.id === registry.bindings[binding])
      ?.name ?? null
  );
}
function featureRows(draft: AppSettings, models: AiModelAsset[]) {
  const { ai } = draft;
  const localLabel = (id: string) => {
    const model = models.find((model) => model.id === id);
    return `${model?.label ?? id}${model?.downloadStatus === 'available' ? '' : ' · Download needed'}`;
  };
  return [
    {
      id: 'summary',
      icon: FileText,
      description: 'Review your day.',
      value: selectedModel(draft, 'summary') ?? 'Choose a model',
      mode: ai.recall.generation.enabled ? 'Remote' : 'Off',
    },
    {
      id: 'search',
      icon: Search,
      description: 'Find related moments.',
      value:
        ai.recall.embedding.provider === 'local'
          ? localLabel(ai.recall.embedding.localModelId)
          : (selectedModel(draft, 'embedding') ?? 'Choose a model'),
      mode:
        ai.recall.embedding.provider === 'local'
          ? 'Local'
          : ai.recall.embedding.enabled
            ? 'Remote'
            : 'Off',
    },
    {
      id: 'transcription',
      icon: AudioLines,
      description: 'Turn audio into text.',
      value:
        ai.transcription.provider === 'local-whisper-cpp'
          ? localLabel(ai.transcription.localModelId)
          : (ai.transcription.remoteModel ?? 'Set up an API'),
      mode:
        ai.transcription.provider === 'local-whisper-cpp' ? 'Local' : 'Remote',
    },
    {
      id: 'ocr',
      icon: ScanText,
      description: 'Read text on screen.',
      value:
        ai.ocr.engine === 'tesseract'
          ? 'Tesseract'
          : (selectedModel(draft, 'vision') ?? 'Choose a screen model'),
      mode: !ai.ocr.enabled
        ? 'Off'
        : ai.ocr.engine === 'tesseract'
          ? 'Local'
          : ai.vision.provider === 'openai-compatible'
            ? 'Remote'
            : 'Off',
    },
    {
      id: 'observations',
      icon: Image,
      description: 'Describe screen activity.',
      value: selectedModel(draft, 'vision') ?? 'Choose a screen model',
      mode:
        ai.vision.observationsEnabled &&
        ai.vision.provider === 'openai-compatible'
          ? 'Remote'
          : 'Off',
    },
  ] satisfies {
    id: SettingsDetail;
    icon: typeof FileText;
    description: string;
    value: string;
    mode: string;
  }[];
}
function FeatureRow({
  item,
  onOpen,
}: {
  item: ReturnType<typeof featureRows>[number];
  onOpen: (detail: SettingsDetail) => void;
}) {
  function open() {
    onOpen(item.id);
  }
  const Icon = item.icon;
  return (
    <button
      type="button"
      onClick={open}
      aria-label={`Configure ${featureNames[item.id]}`}
      className="group hover:bg-muted/40 focus-visible:outline-ring flex w-full items-center gap-4 border-b px-1 py-5 text-left transition-colors last:border-b-0 focus-visible:rounded-lg focus-visible:outline-2"
    >
      <div className="bg-muted flex size-9 shrink-0 items-center justify-center rounded-xl">
        <Icon aria-hidden="true" className="size-4" />
      </div>
      <div className="min-w-0 flex-1">
        <div className="text-sm font-medium">{featureNames[item.id]}</div>
        <div className="text-muted-foreground mt-1 text-xs">
          {item.description}
        </div>
        <div className="text-muted-foreground mt-1 truncate text-xs sm:hidden">
          {item.value}
        </div>
      </div>
      <span className="text-muted-foreground hidden max-w-56 truncate text-xs sm:block">
        {item.value}
      </span>
      <span className="text-muted-foreground shrink-0 text-xs">
        {item.mode}
      </span>
      <ChevronRight
        aria-hidden="true"
        className="text-muted-foreground size-4 shrink-0"
      />
    </button>
  );
}
export interface FeatureOverviewProps {
  draft: AppSettings;
  models: AiModelAsset[];
  onOpen: (detail: SettingsDetail) => void;
  onUpdate: UpdateSettingsDraft;
}
export function FeatureOverview({
  draft,
  models,
  onOpen,
  onUpdate,
}: FeatureOverviewProps) {
  function timing(value: string) {
    if (
      value === 'manual' ||
      value === 'after-stop' ||
      value === 'during-recording'
    )
      onUpdate((current) => ({
        ...current,
        ai: {
          ...current.ai,
          processing: {
            ...current.ai.processing,
            autoStartAfterRecording: value !== 'manual',
            ...(value !== 'manual' ? { timing: value } : {}),
          },
        },
      }));
  }
  function resources(value: string) {
    if (value === 'background' || value === 'fast')
      onUpdate((current) => ({
        ...current,
        ai: {
          ...current.ai,
          processing: { ...current.ai.processing, resourceMode: value },
        },
      }));
  }
  function advanced() {
    onOpen('processing');
  }
  return (
    <div>
      <div>
        {featureRows(draft, models).map((item) => (
          <FeatureRow key={item.id} item={item} onOpen={onOpen} />
        ))}
      </div>
      <h3 className="mt-10 mb-1 text-sm font-medium">Processing</h3>
      <SettingRow title="When to process">
        <Select
          value={
            draft.ai.processing.autoStartAfterRecording
              ? (draft.ai.processing.timing ?? 'after-stop')
              : 'manual'
          }
          onValueChange={timing}
        >
          <SelectTrigger aria-label="Processing timing" className="w-44">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="after-stop">After recording</SelectItem>
            <SelectItem value="during-recording">During recording</SelectItem>
            <SelectItem value="manual">Manually</SelectItem>
          </SelectContent>
        </Select>
      </SettingRow>
      <SettingRow title="Resource use">
        <Select
          value={draft.ai.processing.resourceMode ?? 'background'}
          onValueChange={resources}
        >
          <SelectTrigger aria-label="Processing resource use" className="w-44">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="background">Background</SelectItem>
            <SelectItem value="fast">Fast</SelectItem>
          </SelectContent>
        </Select>
      </SettingRow>
      {draft.expert?.enabled ? (
        <Button
          type="button"
          variant="ghost"
          onClick={advanced}
          className="mt-4 gap-2 px-0"
        >
          <Settings2 aria-hidden="true" className="size-4" />
          Advanced processing
        </Button>
      ) : null}
    </div>
  );
}
