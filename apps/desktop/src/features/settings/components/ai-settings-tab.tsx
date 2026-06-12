import type {
  AiModelAsset,
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
import { AiActivityProcessingCard } from './ai-activity-processing-card';
import { AiModelActionButton, AiModelCacheCard } from './ai-model-cache-card';
import {
  CheckboxField,
  RemoteProviderFields,
  SliderField,
} from './ai-settings-fields';

interface AiSettingsTabProps {
  draft: AppSettings;
  limits: AppSettingsLimits;
  models: AiModelAsset[];
  onCancelModelDownload: (modelId: string) => void;
  onChangeTranscriptionKey: (value: string) => void;
  onChangeVisionKey: (value: string) => void;
  onDeleteModel: (modelId: string) => void;
  onDownloadModel: (modelId: string) => void;
  onUpdateDraft: (updater: (draft: AppSettings) => AppSettings) => void;
  providerStatus: AiProviderStatus | null;
  transcriptionKeyDraft: string;
  visionKeyDraft: string;
}

export function AiSettingsTab({
  draft,
  limits,
  models,
  onChangeTranscriptionKey,
  onChangeVisionKey,
  onDeleteModel,
  onDownloadModel,
  onCancelModelDownload,
  onUpdateDraft,
  providerStatus,
  transcriptionKeyDraft,
  visionKeyDraft,
}: AiSettingsTabProps) {
  const remoteTranscription =
    draft.ai.transcription.provider === 'openai-compatible';
  const remoteVision = draft.ai.vision.provider === 'openai-compatible';
  const transcriptionLanguages = [
    { id: 'auto', label: 'Auto detect' },
    ...(providerStatus?.localTranscription.availableLanguages ?? []),
  ];

  return (
    <div className="grid gap-4">
      <Card>
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
                  const provider = value as
                    | 'local-whisper-cpp'
                    | 'openai-compatible';
                  onUpdateDraft((current) => ({
                    ...current,
                    ai: {
                      ...current.ai,
                      transcription: {
                        ...current.ai.transcription,
                        provider,
                        remoteApiKeyRef:
                          provider === 'openai-compatible'
                            ? current.ai.transcription.remoteApiKeyRef
                            : null,
                      },
                    },
                  }));
                }}
                value={draft.ai.transcription.provider}
              >
                <SelectTrigger id="ai-transcription-provider">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="local-whisper-cpp">
                    Local Whisper
                  </SelectItem>
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
                  onValueChange={(value) => {
                    onUpdateDraft((current) => ({
                      ...current,
                      ai: {
                        ...current.ai,
                        transcription: {
                          ...current.ai.transcription,
                          localModelId: value,
                        },
                      },
                    }));
                  }}
                  value={draft.ai.transcription.localModelId}
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
                    (model) => model.id === draft.ai.transcription.localModelId,
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
                  onUpdateDraft((current) => ({
                    ...current,
                    ai: {
                      ...current.ai,
                      transcription: {
                        ...current.ai.transcription,
                        localBackend,
                      },
                    },
                  }));
                }}
                value={draft.ai.transcription.localBackend}
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
                  onUpdateDraft((current) => ({
                    ...current,
                    ai: {
                      ...current.ai,
                      transcription: {
                        ...current.ai.transcription,
                        language,
                      },
                    },
                  }));
                }}
                value={draft.ai.transcription.language}
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
                Auto-detect is best for recordings that may switch languages.
                Choose a language to force transcription in that language.
              </FieldDescription>
            </Field>

            <CheckboxField
              checked={draft.ai.transcription.wordTimestampsEnabled}
              description="Providers may omit word timestamps; Daydream keeps segment timestamps either way."
              label="Request word timestamps"
              onCheckedChange={(checked) => {
                onUpdateDraft((current) => ({
                  ...current,
                  ai: {
                    ...current.ai,
                    transcription: {
                      ...current.ai.transcription,
                      wordTimestampsEnabled: checked,
                    },
                  },
                }));
              }}
            />

            {draft.expert?.enabled ? (
              <>
                <CheckboxField
                  checked={draft.ai.transcription.silenceGateEnabled}
                  description="Skip transcription for WAV ranges below the configured RMS threshold."
                  label="Enable silence gate"
                  onCheckedChange={(checked) => {
                    onUpdateDraft((current) => ({
                      ...current,
                      ai: {
                        ...current.ai,
                        transcription: {
                          ...current.ai.transcription,
                          silenceGateEnabled: checked,
                        },
                      },
                    }));
                  }}
                />
                <SliderField
                  label="Silence threshold"
                  max={limits.ai.transcription.silenceThresholdDbMax}
                  min={limits.ai.transcription.silenceThresholdDbMin}
                  onChange={(silenceThresholdDb) => {
                    onUpdateDraft((current) => ({
                      ...current,
                      ai: {
                        ...current.ai,
                        transcription: {
                          ...current.ai.transcription,
                          silenceThresholdDb,
                        },
                      },
                    }));
                  }}
                  suffix="dBFS"
                  value={draft.ai.transcription.silenceThresholdDb}
                />
                <SliderField
                  label="Minimum speech"
                  max={limits.ai.transcription.minSpeechMsMax}
                  min={limits.ai.transcription.minSpeechMsMin}
                  onChange={(minSpeechMs) => {
                    onUpdateDraft((current) => ({
                      ...current,
                      ai: {
                        ...current.ai,
                        transcription: {
                          ...current.ai.transcription,
                          minSpeechMs,
                        },
                      },
                    }));
                  }}
                  suffix="ms"
                  value={draft.ai.transcription.minSpeechMs}
                />
                <SliderField
                  label="Merge close speech gaps"
                  max={limits.ai.transcription.mergeGapMsMax}
                  min={limits.ai.transcription.mergeGapMsMin}
                  onChange={(mergeGapMs) => {
                    onUpdateDraft((current) => ({
                      ...current,
                      ai: {
                        ...current.ai,
                        transcription: {
                          ...current.ai.transcription,
                          mergeGapMs,
                        },
                      },
                    }));
                  }}
                  suffix="ms"
                  value={draft.ai.transcription.mergeGapMs}
                />
                <SliderField
                  label="Whisper initial timestamp cap"
                  max={limits.ai.transcription.maxInitialTimestampSecondsMax}
                  min={limits.ai.transcription.maxInitialTimestampSecondsMin}
                  onChange={(maxInitialTimestampSeconds) => {
                    onUpdateDraft((current) => ({
                      ...current,
                      ai: {
                        ...current.ai,
                        transcription: {
                          ...current.ai.transcription,
                          maxInitialTimestampSeconds,
                        },
                      },
                    }));
                  }}
                  step={0.5}
                  suffix="s"
                  value={draft.ai.transcription.maxInitialTimestampSeconds}
                />
              </>
            ) : null}

            {remoteTranscription ? (
              <>
                <RemoteProviderFields
                  baseUrl={draft.ai.transcription.remoteBaseUrl ?? ''}
                  hasSavedKey={Boolean(draft.ai.transcription.remoteApiKeyRef)}
                  keyDraft={transcriptionKeyDraft}
                  onBaseUrlChange={(value) => {
                    onUpdateDraft((current) => ({
                      ...current,
                      ai: {
                        ...current.ai,
                        transcription: {
                          ...current.ai.transcription,
                          remoteBaseUrl: value || null,
                        },
                      },
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
                      onUpdateDraft((current) => ({
                        ...current,
                        ai: {
                          ...current.ai,
                          transcription: {
                            ...current.ai.transcription,
                            remoteModel: value || null,
                          },
                        },
                      }));
                    }}
                    placeholder="whisper-1"
                    value={draft.ai.transcription.remoteModel ?? ''}
                  />
                </Field>
              </>
            ) : null}
          </FieldGroup>
        </CardContent>
      </Card>

      <AiModelCacheCard
        models={models}
        onCancelModelDownload={onCancelModelDownload}
        onDeleteModel={onDeleteModel}
        onDownloadModel={onDownloadModel}
      />

      <Card>
        <CardHeader>
          <CardTitle>OCR and Vision</CardTitle>
          <CardDescription>
            OCR is local. VLM observations require an explicitly configured
            remote provider.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <FieldGroup>
            <CheckboxField
              checked={draft.ai.ocr.enabled}
              description="Tesseract is the only OCR engine in M2."
              label="Run OCR on retained frames"
              onCheckedChange={(checked) => {
                onUpdateDraft((current) => ({
                  ...current,
                  ai: {
                    ...current.ai,
                    ocr: {
                      ...current.ai.ocr,
                      enabled: checked,
                    },
                  },
                }));
              }}
            />
            <Field>
              <FieldLabel htmlFor="ai-vision-provider">
                Vision provider
              </FieldLabel>
              <Select
                onValueChange={(value) => {
                  const provider = value as 'disabled' | 'openai-compatible';
                  onUpdateDraft((current) => ({
                    ...current,
                    ai: {
                      ...current.ai,
                      vision: {
                        ...current.ai.vision,
                        apiKeyRef:
                          provider === 'openai-compatible'
                            ? current.ai.vision.apiKeyRef
                            : null,
                        provider,
                      },
                    },
                  }));
                }}
                value={draft.ai.vision.provider}
              >
                <SelectTrigger id="ai-vision-provider">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="disabled">Disabled</SelectItem>
                  <SelectItem value="openai-compatible">
                    OpenAI-compatible
                  </SelectItem>
                </SelectContent>
              </Select>
            </Field>
            {remoteVision ? (
              <>
                <RemoteProviderFields
                  baseUrl={draft.ai.vision.baseUrl ?? ''}
                  hasSavedKey={Boolean(draft.ai.vision.apiKeyRef)}
                  keyDraft={visionKeyDraft}
                  onBaseUrlChange={(value) => {
                    onUpdateDraft((current) => ({
                      ...current,
                      ai: {
                        ...current.ai,
                        vision: {
                          ...current.ai.vision,
                          baseUrl: value || null,
                        },
                      },
                    }));
                  }}
                  onKeyChange={onChangeVisionKey}
                  prefix="vision"
                />
                <Field>
                  <FieldLabel htmlFor="ai-vision-model">
                    Vision model
                  </FieldLabel>
                  <Input
                    id="ai-vision-model"
                    onChange={(event) => {
                      onUpdateDraft((current) => ({
                        ...current,
                        ai: {
                          ...current.ai,
                          vision: {
                            ...current.ai.vision,
                            model: event.target.value || null,
                          },
                        },
                      }));
                    }}
                    placeholder="Model name"
                    value={draft.ai.vision.model ?? ''}
                  />
                </Field>
              </>
            ) : null}
          </FieldGroup>
        </CardContent>
      </Card>

      <AiActivityProcessingCard
        draft={draft}
        limits={limits}
        onUpdateDraft={onUpdateDraft}
      />
    </div>
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
