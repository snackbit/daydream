import type {
  AiModelAsset,
  AiProviderStatus,
  AiTranscriptionBackend,
  AppSettings,
  AppSettingsLimits,
} from '@daydream/shared';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
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

  return (
    <div className="grid gap-4">
      <Alert>
        <AlertTitle>Local baseline</AlertTitle>
        <AlertDescription>
          Local Whisper and Tesseract remain the default. Remote providers are
          optional and send private captured content only in later processing
          gates.
        </AlertDescription>
      </Alert>

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

            {remoteTranscription ? (
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

      <Card>
        <CardHeader>
          <CardTitle>Activity and Processing</CardTitle>
          <CardDescription>
            Controls for automatic processing and activity inference.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <FieldGroup>
            <CheckboxField
              checked={draft.ai.processing.autoStartAfterRecording}
              description="Process eligible, non-redacted recordings after capture ends."
              label="Start processing after recording"
              onCheckedChange={(checked) => {
                onUpdateDraft((current) => ({
                  ...current,
                  ai: {
                    ...current.ai,
                    processing: {
                      ...current.ai.processing,
                      autoStartAfterRecording: checked,
                    },
                  },
                }));
              }}
            />
            <CheckboxField
              checked={draft.ai.processing.frameSimilarityEnabled}
              description="Skip visually similar frame regions between periodic full-frame passes."
              label="Use frame similarity cache"
              onCheckedChange={(checked) => {
                onUpdateDraft((current) => ({
                  ...current,
                  ai: {
                    ...current.ai,
                    processing: {
                      ...current.ai.processing,
                      frameSimilarityEnabled: checked,
                    },
                  },
                }));
              }}
            />
            <div className="grid gap-4 md:grid-cols-3">
              <SliderField
                label="Parallel jobs"
                max={limits.ai.processing.maxParallelJobsMax}
                min={limits.ai.processing.maxParallelJobsMin}
                onChange={(value) => {
                  onUpdateDraft((current) => ({
                    ...current,
                    ai: {
                      ...current.ai,
                      processing: {
                        ...current.ai.processing,
                        maxParallelJobs: value,
                      },
                    },
                  }));
                }}
                value={draft.ai.processing.maxParallelJobs}
              />
              <SliderField
                label="Chunks per job"
                max={limits.ai.processing.maxParallelChunksPerJobMax}
                min={limits.ai.processing.maxParallelChunksPerJobMin}
                onChange={(value) => {
                  onUpdateDraft((current) => ({
                    ...current,
                    ai: {
                      ...current.ai,
                      processing: {
                        ...current.ai.processing,
                        maxParallelChunksPerJob: value,
                      },
                    },
                  }));
                }}
                value={draft.ai.processing.maxParallelChunksPerJob}
              />
              <SliderField
                label="Full-frame interval"
                max={limits.ai.processing.periodicFullFrameIntervalMsMax}
                min={limits.ai.processing.periodicFullFrameIntervalMsMin}
                onChange={(value) => {
                  onUpdateDraft((current) => ({
                    ...current,
                    ai: {
                      ...current.ai,
                      processing: {
                        ...current.ai.processing,
                        periodicFullFrameIntervalMs: value,
                      },
                    },
                  }));
                }}
                step={500}
                suffix="ms"
                value={draft.ai.processing.periodicFullFrameIntervalMs}
              />
            </div>
            <CheckboxField
              checked={draft.ai.activity.keyboard.enabled}
              description="Requires platform support in later gates."
              label="Use keyboard activity"
              onCheckedChange={(checked) => {
                onUpdateDraft((current) => ({
                  ...current,
                  ai: {
                    ...current.ai,
                    activity: {
                      ...current.ai.activity,
                      keyboard: {
                        ...current.ai.activity.keyboard,
                        enabled: checked,
                      },
                    },
                  },
                }));
              }}
            />
            <SliderField
              label="Keyboard activity window"
              max={limits.ai.activity.keyboardWindowMsMax}
              min={limits.ai.activity.keyboardWindowMsMin}
              onChange={(value) => {
                onUpdateDraft((current) => ({
                  ...current,
                  ai: {
                    ...current.ai,
                    activity: {
                      ...current.ai.activity,
                      keyboard: {
                        ...current.ai.activity.keyboard,
                        windowMs: value,
                      },
                    },
                  },
                }));
              }}
              step={1_000}
              suffix="ms"
              value={draft.ai.activity.keyboard.windowMs}
            />
            <CheckboxField
              checked={draft.ai.activity.mouse.enabled}
              description="Requires platform support in later gates."
              label="Use mouse activity"
              onCheckedChange={(checked) => {
                onUpdateDraft((current) => ({
                  ...current,
                  ai: {
                    ...current.ai,
                    activity: {
                      ...current.ai.activity,
                      mouse: {
                        ...current.ai.activity.mouse,
                        enabled: checked,
                      },
                    },
                  },
                }));
              }}
            />
            <SliderField
              label="Mouse activity window"
              max={limits.ai.activity.mouseWindowMsMax}
              min={limits.ai.activity.mouseWindowMsMin}
              onChange={(value) => {
                onUpdateDraft((current) => ({
                  ...current,
                  ai: {
                    ...current.ai,
                    activity: {
                      ...current.ai.activity,
                      mouse: {
                        ...current.ai.activity.mouse,
                        windowMs: value,
                      },
                    },
                  },
                }));
              }}
              step={1_000}
              suffix="ms"
              value={draft.ai.activity.mouse.windowMs}
            />
          </FieldGroup>
        </CardContent>
      </Card>

      {providerStatus ? (
        <div className="text-muted-foreground grid gap-2 border p-3 text-xs md:grid-cols-3">
          <span>
            Local transcription:{' '}
            {providerStatus.localTranscription.modelDownloadStatus}; backend{' '}
            {providerStatus.localTranscription.selectedBackend}
          </span>
          <span>
            Remote transcription: {providerStatus.remoteTranscription}
          </span>
          <span>Vision: {providerStatus.vision}</span>
        </div>
      ) : null}
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
