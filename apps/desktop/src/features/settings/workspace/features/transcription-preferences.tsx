import type { ChangeEvent } from 'react';
import type { AiTranscriptionProviderSettings } from '@daydream/shared';
import { SharedDisclosure } from '@/components/shared-disclosure';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import type {
  SettingsScreenTabsModel,
  SettingsScreenTabsActions,
} from '../../screen/settings-screen-tabs';
import { NumberPreference } from '../controls/number-preference';
import { SettingRow } from '../controls/setting-row';

export function TranscriptionPreferences({
  model,
  actions,
}: {
  model: SettingsScreenTabsModel;
  actions: SettingsScreenTabsActions;
}) {
  const settings = model.draft.ai.transcription;
  function update(changes: Partial<AiTranscriptionProviderSettings>) {
    actions.updateDraft((current) => ({
      ...current,
      ai: {
        ...current.ai,
        transcription: { ...current.ai.transcription, ...changes },
      },
    }));
  }
  function provider(value: string) {
    if (value === 'local-whisper-cpp' || value === 'openai-compatible')
      update({ provider: value });
  }
  function localModel(localModelId: string) {
    update({ localModelId });
  }
  function language(language: string) {
    update({ language });
  }
  function backend(value: string) {
    if (value === 'cpu' || value === 'vulkan' || value === 'cuda')
      update({ localBackend: value });
  }
  function baseUrl(event: ChangeEvent<HTMLInputElement>) {
    update({ remoteBaseUrl: event.target.value || null });
  }
  function remoteModel(event: ChangeEvent<HTMLInputElement>) {
    update({ remoteModel: event.target.value || null });
  }
  function key(event: ChangeEvent<HTMLInputElement>) {
    actions.changeTranscriptionKey(event.target.value);
  }
  function words(wordTimestampsEnabled: boolean) {
    update({ wordTimestampsEnabled });
  }
  function silence(silenceGateEnabled: boolean) {
    update({ silenceGateEnabled });
  }
  function threshold(silenceThresholdDb: number) {
    update({ silenceThresholdDb });
  }
  function minSpeech(minSpeechMs: number) {
    update({ minSpeechMs });
  }
  function merge(mergeGapMs: number) {
    update({ mergeGapMs });
  }
  function timestamp(maxInitialTimestampSeconds: number) {
    update({ maxInitialTimestampSeconds });
  }
  const languages =
    model.aiProviderStatus?.localTranscription.availableLanguages ?? [];
  const limits = model.limits.ai.transcription;
  return (
    <div className="grid gap-6">
      <SettingRow title="Processing">
        <Select value={settings.provider} onValueChange={provider}>
          <SelectTrigger aria-label="Transcription processing" className="w-44">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="local-whisper-cpp">Local · Whisper</SelectItem>
            <SelectItem value="openai-compatible">Remote · API</SelectItem>
          </SelectContent>
        </Select>
      </SettingRow>
      {settings.provider === 'local-whisper-cpp' ? (
        <div className="grid gap-2">
          <label className="text-sm" htmlFor="transcription-local-model">
            Whisper model
          </label>
          <Select value={settings.localModelId} onValueChange={localModel}>
            <SelectTrigger id="transcription-local-model" className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {model.aiModels
                .filter((asset) => asset.provider === 'local-whisper-cpp')
                .map((asset) => (
                  <SelectItem key={asset.id} value={asset.id}>
                    {asset.label}
                    {asset.downloadStatus === 'available'
                      ? ''
                      : ' · Download needed'}
                  </SelectItem>
                ))}
            </SelectContent>
          </Select>
          <p className="text-muted-foreground text-xs">
            Manage downloads in Models & accounts.
          </p>
        </div>
      ) : (
        <div className="grid gap-4">
          <div className="grid gap-2">
            <label htmlFor="transcription-url" className="text-sm">
              API URL
            </label>
            <Input
              id="transcription-url"
              type="url"
              value={settings.remoteBaseUrl ?? ''}
              onChange={baseUrl}
              placeholder="https://api.openai.com/v1"
            />
          </div>
          <div className="grid gap-2">
            <label htmlFor="transcription-remote-model" className="text-sm">
              Model
            </label>
            <Input
              id="transcription-remote-model"
              value={settings.remoteModel ?? ''}
              onChange={remoteModel}
            />
          </div>
          <div className="grid gap-2">
            <label htmlFor="transcription-key" className="text-sm">
              API key
            </label>
            <Input
              id="transcription-key"
              type="password"
              autoComplete="off"
              value={model.transcriptionKeyDraft}
              placeholder={
                settings.remoteApiKeyRef
                  ? 'Saved · enter a key to replace'
                  : 'Enter API key'
              }
              onChange={key}
            />
          </div>
          <p className="text-muted-foreground text-sm">
            Recorded audio is sent to this API. ChatGPT sign-in does not cover
            transcription.
          </p>
        </div>
      )}
      <SettingRow title="Language">
        <Select value={settings.language} onValueChange={language}>
          <SelectTrigger aria-label="Transcription language" className="w-44">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="auto">Auto detect</SelectItem>
            {languages
              .filter((item) => item.id !== 'auto')
              .map((item) => (
                <SelectItem key={item.id} value={item.id}>
                  {item.label}
                </SelectItem>
              ))}
          </SelectContent>
        </Select>
      </SettingRow>
      {model.draft.expert?.enabled ? (
        <SharedDisclosure label="Advanced">
          <div className="grid gap-2">
            <SettingRow title="Local backend">
              <Select value={settings.localBackend} onValueChange={backend}>
                <SelectTrigger aria-label="Local backend" className="w-36">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {(['cpu', 'vulkan', 'cuda'] as const).map((value) => (
                    <SelectItem
                      key={value}
                      value={value}
                      disabled={
                        model.aiProviderStatus?.localTranscription.availableBackends.find(
                          (item) => item.id === value,
                        )?.available === false
                      }
                    >
                      {value.toUpperCase()}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </SettingRow>
            <SettingRow title="Word timestamps">
              <Switch
                aria-label="Word timestamps"
                checked={settings.wordTimestampsEnabled}
                onCheckedChange={words}
              />
            </SettingRow>
            <SettingRow title="Skip silence">
              <Switch
                aria-label="Skip silence"
                checked={settings.silenceGateEnabled}
                onCheckedChange={silence}
              />
            </SettingRow>
            <SettingRow title="Silence threshold">
              <NumberPreference
                label="Silence threshold"
                value={settings.silenceThresholdDb}
                min={limits.silenceThresholdDbMin}
                max={limits.silenceThresholdDbMax}
                unit="dB"
                onCommit={threshold}
              />
            </SettingRow>
            <SettingRow title="Minimum speech">
              <NumberPreference
                label="Minimum speech"
                value={settings.minSpeechMs}
                min={limits.minSpeechMsMin}
                max={limits.minSpeechMsMax}
                unit="ms"
                onCommit={minSpeech}
              />
            </SettingRow>
            <SettingRow title="Merge gap">
              <NumberPreference
                label="Merge gap"
                value={settings.mergeGapMs}
                min={limits.mergeGapMsMin}
                max={limits.mergeGapMsMax}
                unit="ms"
                onCommit={merge}
              />
            </SettingRow>
            <SettingRow title="Initial timestamp limit">
              <NumberPreference
                label="Initial timestamp limit"
                value={settings.maxInitialTimestampSeconds}
                min={limits.maxInitialTimestampSecondsMin}
                max={limits.maxInitialTimestampSecondsMax}
                unit="seconds"
                onCommit={timestamp}
              />
            </SettingRow>
          </div>
        </SharedDisclosure>
      ) : null}
    </div>
  );
}
