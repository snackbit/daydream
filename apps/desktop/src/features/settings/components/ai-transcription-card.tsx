import type {
  AiModelAsset,
  AiProviderKind,
  AiProviderStatus,
  AiTranscriptionBackend,
  AppSettings,
  AppSettingsLimits,
} from '@daydream/shared';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  type UpdateSettingsDraft,
  updateAiTranscription,
} from '@/features/settings/settings-draft-updaters';
import { AiModelActionButton } from './ai-model-cache-card';
import {
  CheckboxField,
  RemoteProviderFields,
  SliderField,
} from './ai-settings-fields';

interface AiTranscriptionCardProps {
  draft: AppSettings;
  limits: AppSettingsLimits;
  models: AiModelAsset[];
  onCancelModelDownload: (modelId: string) => void;
  onChangeTranscriptionKey: (value: string) => void;
  onDeleteModel: (modelId: string) => void;
  onDownloadModel: (modelId: string) => void;
  onUpdateDraft: UpdateSettingsDraft;
  providerStatus: AiProviderStatus | null;
  transcriptionKeyDraft: string;
}

export function AiTranscriptionCard({
  draft,
  limits,
  models,
  onCancelModelDownload,
  onChangeTranscriptionKey,
  onDeleteModel,
  onDownloadModel,
  onUpdateDraft,
  providerStatus,
  transcriptionKeyDraft,
}: AiTranscriptionCardProps) {
  const transcription = draft.ai.transcription;
  const remoteTranscription = transcription.provider === 'openai-compatible';
  const transcriptionLanguages = [
    { id: 'auto', label: 'Auto detect' },
    ...(providerStatus?.localTranscription.availableLanguages ?? []),
  ];

  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>Transcription</CardTitle>
        <CardDescription>
          Local Whisper is the default transcription provider.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <FieldGroup>
          <Field>
            <FieldLabel htmlFor="ai-transcription-provider">
              Provider
            </FieldLabel>
            <Select
              onValueChange={(value) => {
                const provider = value as AiProviderKind;
                updateAiTranscription(onUpdateDraft, (current) => ({
                  ...current,
                  provider,
                  remoteApiKeyRef:
                    provider === 'openai-compatible'
                      ? current.remoteApiKeyRef
                      : null,
                }));
              }}
              value={transcription.provider}
            >
              <SelectTrigger id="ai-transcription-provider">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="local-whisper-cpp">Local Whisper</SelectItem>
                <SelectItem value="openai-compatible">
                  OpenAI-compatible
                </SelectItem>
              </SelectContent>
            </Select>
          </Field>

          <Field>
            <FieldLabel htmlFor="ai-local-model">Local model</FieldLabel>
            <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
              <Select
                onValueChange={(localModelId) => {
                  updateAiTranscription(onUpdateDraft, (current) => ({
                    ...current,
                    localModelId,
                  }));
                }}
                value={transcription.localModelId}
              >
                <SelectTrigger className="sm:flex-1" id="ai-local-model">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {models.map((model) => (
                    <SelectItem key={model.id} value={model.id}>
                      {model.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <AiModelActionButton
                model={models.find(
                  (model) => model.id === transcription.localModelId,
                )}
                onCancelModelDownload={onCancelModelDownload}
                onDeleteModel={onDeleteModel}
                onDownloadModel={onDownloadModel}
              />
            </div>
          </Field>

          <Field>
            <FieldLabel htmlFor="ai-local-backend">Local backend</FieldLabel>
            <Select
              onValueChange={(value) => {
                const localBackend = value as AiTranscriptionBackend;
                updateAiTranscription(onUpdateDraft, (current) => ({
                  ...current,
                  localBackend,
                }));
              }}
              value={transcription.localBackend}
            >
              <SelectTrigger id="ai-local-backend">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {(['cpu', 'vulkan', 'cuda'] as const).map((backend) => {
                  const status =
                    providerStatus?.localTranscription.availableBackends.find(
                      (candidate) => candidate.id === backend,
                    );
                  const disabled = status?.available === false;

                  return (
                    <SelectItem
                      disabled={disabled}
                      key={backend}
                      value={backend}
                    >
                      {backendLabel(backend)}
                      {disabled ? ' unavailable' : ''}
                    </SelectItem>
                  );
                })}
              </SelectContent>
            </Select>
            <FieldDescription>
              Vulkan and CUDA backends may require additional setup and have
              specific hardware and OS requirements.
            </FieldDescription>
          </Field>

          <Field>
            <FieldLabel htmlFor="ai-transcription-language">
              Language
            </FieldLabel>
            <Select
              onValueChange={(language) => {
                updateAiTranscription(onUpdateDraft, (current) => ({
                  ...current,
                  language,
                }));
              }}
              value={transcription.language}
            >
              <SelectTrigger id="ai-transcription-language">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {transcriptionLanguages.map((language) => (
                  <SelectItem key={language.id} value={language.id}>
                    {language.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <FieldDescription>
              Local Whisper remembers confident language detections during
              nearby speech and detects again after a gap longer than 15
              seconds. Choose a language to force transcription in that
              language.
            </FieldDescription>
          </Field>

          <CheckboxField
            checked={transcription.wordTimestampsEnabled}
            description="Providers may omit word timestamps; Daydream keeps segment timestamps either way."
            label="Request word timestamps"
            onCheckedChange={(wordTimestampsEnabled) => {
              updateAiTranscription(onUpdateDraft, (current) => ({
                ...current,
                wordTimestampsEnabled,
              }));
            }}
          />

          {draft.expert?.enabled ? (
            <ExpertTranscriptionFields
              limits={limits}
              onUpdateDraft={onUpdateDraft}
              transcription={transcription}
            />
          ) : null}

          {remoteTranscription ? (
            <>
              <RemoteProviderFields
                baseUrl={transcription.remoteBaseUrl ?? ''}
                dataSentLabel="Recorded audio for transcription"
                hasSavedKey={Boolean(transcription.remoteApiKeyRef)}
                keyDraft={transcriptionKeyDraft}
                localAlternativeDescription="Choose Local Whisper to keep speech-to-text on this device."
                onBaseUrlChange={(value) => {
                  updateAiTranscription(onUpdateDraft, (current) => ({
                    ...current,
                    remoteBaseUrl: value || null,
                  }));
                }}
                onKeyChange={onChangeTranscriptionKey}
                prefix="transcription"
              />
              <Field>
                <FieldLabel htmlFor="ai-transcription-remote-model">
                  Remote model
                </FieldLabel>
                <Input
                  id="ai-transcription-remote-model"
                  onChange={(event) => {
                    const value = event.currentTarget.value;
                    updateAiTranscription(onUpdateDraft, (current) => ({
                      ...current,
                      remoteModel: value || null,
                    }));
                  }}
                  placeholder="whisper-1"
                  value={transcription.remoteModel ?? ''}
                />
              </Field>
            </>
          ) : null}
        </FieldGroup>
      </CardContent>
    </Card>
  );
}

function ExpertTranscriptionFields({
  limits,
  onUpdateDraft,
  transcription,
}: {
  limits: AppSettingsLimits;
  onUpdateDraft: UpdateSettingsDraft;
  transcription: AppSettings['ai']['transcription'];
}) {
  return (
    <>
      <CheckboxField
        checked={transcription.silenceGateEnabled}
        description="Skip transcription for WAV ranges below the configured RMS threshold."
        label="Enable silence gate"
        onCheckedChange={(silenceGateEnabled) => {
          updateAiTranscription(onUpdateDraft, (current) => ({
            ...current,
            silenceGateEnabled,
          }));
        }}
      />
      <SliderField
        label="Silence threshold"
        max={limits.ai.transcription.silenceThresholdDbMax}
        min={limits.ai.transcription.silenceThresholdDbMin}
        onChange={(silenceThresholdDb) => {
          updateAiTranscription(onUpdateDraft, (current) => ({
            ...current,
            silenceThresholdDb,
          }));
        }}
        suffix="dBFS"
        value={transcription.silenceThresholdDb}
      />
      <SliderField
        label="Minimum speech"
        max={limits.ai.transcription.minSpeechMsMax}
        min={limits.ai.transcription.minSpeechMsMin}
        onChange={(minSpeechMs) => {
          updateAiTranscription(onUpdateDraft, (current) => ({
            ...current,
            minSpeechMs,
          }));
        }}
        suffix="ms"
        value={transcription.minSpeechMs}
      />
      <SliderField
        label="Merge close speech gaps"
        max={limits.ai.transcription.mergeGapMsMax}
        min={limits.ai.transcription.mergeGapMsMin}
        onChange={(mergeGapMs) => {
          updateAiTranscription(onUpdateDraft, (current) => ({
            ...current,
            mergeGapMs,
          }));
        }}
        suffix="ms"
        value={transcription.mergeGapMs}
      />
      <SliderField
        label="Whisper initial timestamp cap"
        max={limits.ai.transcription.maxInitialTimestampSecondsMax}
        min={limits.ai.transcription.maxInitialTimestampSecondsMin}
        onChange={(maxInitialTimestampSeconds) => {
          updateAiTranscription(onUpdateDraft, (current) => ({
            ...current,
            maxInitialTimestampSeconds,
          }));
        }}
        step={0.5}
        suffix="s"
        value={transcription.maxInitialTimestampSeconds}
      />
    </>
  );
}

function backendLabel(backend: AiTranscriptionBackend): string {
  if (backend === 'cpu') {
    return 'CPU';
  }
  if (backend === 'vulkan') {
    return 'Vulkan';
  }

  return 'CUDA';
}
