import { useCallback, type ChangeEvent } from 'react';
import type { AiOcrEngine, AiVisionProviderSettings } from '@daydream/shared';
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
  updateAiVision,
} from '../../settings-draft-updaters';
import { CheckboxField } from '../ai-settings-fields';
import { VisionProviderTest } from './vision-provider-test';

export interface VisionEndpointSettingsSectionModel {
  ocrEnabled: boolean;
  ocrEngine: AiOcrEngine;
  settings: AiVisionProviderSettings;
  visionKeyDraft: string;
  sourceCount: number;
  samplesPerMinute: number;
}
export interface VisionEndpointSettingsSectionActions {
  onChangeVisionKey: (value: string) => void;
  onUpdateDraft: UpdateSettingsDraft;
}
export interface VisionEndpointSettingsSectionProps {
  actions: VisionEndpointSettingsSectionActions;
  model: VisionEndpointSettingsSectionModel;
}

export function VisionEndpointSettingsSection({
  actions,
  model,
}: VisionEndpointSettingsSectionProps) {
  const { settings } = model;
  const { onChangeVisionKey } = actions;
  const update = useCallback(
    (changes: Partial<AiVisionProviderSettings>) =>
      updateAiVision(actions.onUpdateDraft, (current) => ({
        ...current,
        ...changes,
      })),
    [actions.onUpdateDraft],
  );
  function setProvider(value: string) {
    if (value === 'disabled' || value === 'openai-compatible')
      update({ provider: value });
  }
  function setAuth(value: string) {
    if (value === 'none' || value === 'api-key') update({ authMode: value });
  }
  const changeObservations = useCallback(
    (observationsEnabled: boolean) => update({ observationsEnabled }),
    [update],
  );
  const changeBaseUrl = useCallback(
    (event: ChangeEvent<HTMLInputElement>) =>
      update({ baseUrl: event.target.value || null }),
    [update],
  );
  const changeModel = useCallback(
    (event: ChangeEvent<HTMLInputElement>) =>
      update({ model: event.target.value || null }),
    [update],
  );
  const changeKey = useCallback(
    (event: ChangeEvent<HTMLInputElement>) =>
      onChangeVisionKey(event.target.value),
    [onChangeVisionKey],
  );
  return (
    <FieldGroup className="gap-3">
      <CheckboxField
        checked={settings.observationsEnabled}
        label="Generate screen observations"
        description="Describe activity in sampled screen frames."
        onCheckedChange={changeObservations}
      />
      <Field>
        <FieldLabel htmlFor="ai-vision-provider">Vision provider</FieldLabel>
        <Select value={settings.provider} onValueChange={setProvider}>
          <SelectTrigger id="ai-vision-provider">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="disabled">Disabled</SelectItem>
            <SelectItem value="openai-compatible">OpenAI-compatible</SelectItem>
          </SelectContent>
        </Select>
      </Field>
      {settings.provider === 'openai-compatible' ? (
        <>
          <div className="grid gap-3 lg:grid-cols-2">
            <Field>
              <FieldLabel htmlFor="ai-vision-base-url">Base URL</FieldLabel>
              <Input
                id="ai-vision-base-url"
                value={settings.baseUrl ?? ''}
                placeholder="http://localhost:8080/v1"
                onChange={changeBaseUrl}
              />
            </Field>
            <Field>
              <FieldLabel htmlFor="ai-vision-model">Vision model</FieldLabel>
              <Input
                id="ai-vision-model"
                value={settings.model ?? ''}
                placeholder="Model name"
                onChange={changeModel}
              />
            </Field>
            <Field>
              <FieldLabel htmlFor="ai-vision-auth">Authentication</FieldLabel>
              <Select
                value={settings.authMode ?? 'api-key'}
                onValueChange={setAuth}
              >
                <SelectTrigger id="ai-vision-auth">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="api-key">API key</SelectItem>
                  <SelectItem value="none">No authentication</SelectItem>
                </SelectContent>
              </Select>
            </Field>
            {settings.authMode !== 'none' ? (
              <Field>
                <FieldLabel htmlFor="ai-vision-api-key">API key</FieldLabel>
                <Input
                  id="ai-vision-api-key"
                  autoComplete="off"
                  type="password"
                  value={model.visionKeyDraft}
                  placeholder={settings.apiKeyRef ? 'Saved key' : 'API key'}
                  onChange={changeKey}
                />
                <FieldDescription>
                  {model.visionKeyDraft.trim()
                    ? 'New key, not saved.'
                    : settings.apiKeyRef
                      ? 'Key saved.'
                      : 'Enter an API key.'}
                </FieldDescription>
              </Field>
            ) : null}
          </div>
          <VisionProviderTest
            vision={settings}
            keyDraft={model.visionKeyDraft}
            ocr={model.ocrEnabled && model.ocrEngine === 'vision'}
            observations={settings.observationsEnabled}
          />
          <details className="text-muted-foreground text-xs">
            <summary className="cursor-pointer">Data and usage</summary>
            <p className="mt-1">
              Vision sends sampled screen frames to this endpoint. Screen
              observations can also include recognized text. API keys are
              encrypted locally.
            </p>
            <p className="mt-1">
              {model.sourceCount} video{' '}
              {model.sourceCount === 1 ? 'source' : 'sources'} at up to{' '}
              {Number(model.samplesPerMinute.toFixed(2))} samples per minute
              each:{' '}
              {Number((model.sourceCount * model.samplesPerMinute).toFixed(2))}{' '}
              frames per minute in total. Each enabled Vision task can make a
              separate request per frame. Cached OCR frames may reduce calls.
              Provider charges vary.
            </p>
          </details>
        </>
      ) : null}
    </FieldGroup>
  );
}
