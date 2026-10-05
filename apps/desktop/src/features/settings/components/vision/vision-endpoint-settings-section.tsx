import { useCallback, type ChangeEvent } from 'react';
import type {
  AiOcrEngine,
  AiVisionProviderKind,
  AiVisionProviderSettings,
} from '@daydream/shared';
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
} from '@/features/settings/settings-draft-updaters';
import { CheckboxField, RemoteProviderFields } from '../ai-settings-fields';
import { visionRemoteDisclosure } from './vision-settings-format';

export interface VisionEndpointSettingsSectionModel {
  ocrEnabled: boolean;
  ocrEngine: AiOcrEngine;
  settings: AiVisionProviderSettings;
  visionKeyDraft: string;
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
  const remoteVision = settings.provider === 'openai-compatible';
  const handleObservationsChange = useCallback(
    (observationsEnabled: boolean) => {
      updateAiVision(actions.onUpdateDraft, (current) => ({
        ...current,
        observationsEnabled,
      }));
    },
    [actions.onUpdateDraft],
  );
  const handleVisionProviderChange = useCallback(
    (value: string) => {
      const provider = parseVisionProvider(value);
      if (!provider) {
        return;
      }
      updateAiVision(actions.onUpdateDraft, (current) => ({
        ...current,
        apiKeyRef: provider === 'openai-compatible' ? current.apiKeyRef : null,
        provider,
      }));
    },
    [actions.onUpdateDraft],
  );
  const handleBaseUrlChange = useCallback(
    (value: string) => {
      updateAiVision(actions.onUpdateDraft, (current) => ({
        ...current,
        baseUrl: value || null,
      }));
    },
    [actions.onUpdateDraft],
  );
  const handleModelChange = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      const modelName = event.target.value;
      updateAiVision(actions.onUpdateDraft, (current) => ({
        ...current,
        model: modelName || null,
      }));
    },
    [actions.onUpdateDraft],
  );

  return (
    <FieldGroup>
      <CheckboxField
        checked={settings.observationsEnabled}
        description="Use the shared Vision endpoint for screen observations. Turning this off keeps the endpoint available for Vision OCR."
        label="Generate screen observations"
        onCheckedChange={handleObservationsChange}
      />
      <Field>
        <FieldLabel htmlFor="ai-vision-provider">Vision provider</FieldLabel>
        <Select
          onValueChange={handleVisionProviderChange}
          value={settings.provider}
        >
          <SelectTrigger id="ai-vision-provider">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="disabled">Disabled</SelectItem>
            <SelectItem value="openai-compatible">OpenAI-compatible</SelectItem>
          </SelectContent>
        </Select>
        <FieldDescription>
          One endpoint, model and encrypted key are shared by Vision OCR and
          screen observations.
        </FieldDescription>
      </Field>
      {remoteVision ? (
        <>
          <RemoteProviderFields
            baseUrl={settings.baseUrl ?? ''}
            customDisclosure={visionRemoteDisclosure({
              ocrEnabled: model.ocrEnabled,
              ocrEngine: model.ocrEngine,
              observationsEnabled: settings.observationsEnabled,
            })}
            dataSentLabel="Retained screen frames and OCR text"
            hasSavedKey={Boolean(settings.apiKeyRef)}
            keyDraft={model.visionKeyDraft}
            localAlternativeDescription="Tesseract is the local OCR option. Disabling screen observations does not disable Vision OCR."
            onBaseUrlChange={handleBaseUrlChange}
            onKeyChange={actions.onChangeVisionKey}
            prefix="vision"
          />
          <Field>
            <FieldLabel htmlFor="ai-vision-model">Vision model</FieldLabel>
            <Input
              id="ai-vision-model"
              onChange={handleModelChange}
              placeholder="Model name"
              value={settings.model ?? ''}
            />
          </Field>
        </>
      ) : null}
    </FieldGroup>
  );
}

function parseVisionProvider(value: string): AiVisionProviderKind | null {
  return value === 'disabled' || value === 'openai-compatible' ? value : null;
}
