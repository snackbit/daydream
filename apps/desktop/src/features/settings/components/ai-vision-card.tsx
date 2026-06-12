import type { AiVisionProviderKind, AppSettings } from '@daydream/shared';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Field, FieldGroup, FieldLabel } from '@/components/ui/field';
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
  updateAiOcr,
  updateAiVision,
} from '@/features/settings/settings-draft-updaters';
import { CheckboxField, RemoteProviderFields } from './ai-settings-fields';

interface AiVisionCardProps {
  draft: AppSettings;
  onChangeVisionKey: (value: string) => void;
  onUpdateDraft: UpdateSettingsDraft;
  visionKeyDraft: string;
}

export function AiVisionCard({
  draft,
  onChangeVisionKey,
  onUpdateDraft,
  visionKeyDraft,
}: AiVisionCardProps) {
  const { ocr, vision } = draft.ai;
  const remoteVision = vision.provider === 'openai-compatible';

  return (
    <Card>
      <CardHeader>
        <CardTitle>OCR and Vision</CardTitle>
        <CardDescription>
          OCR is local. VLM observations require an explicitly configured remote
          provider.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <FieldGroup>
          <CheckboxField
            checked={ocr.enabled}
            description="Tesseract is the only OCR engine in M2."
            label="Run OCR on retained frames"
            onCheckedChange={(enabled) => {
              updateAiOcr(onUpdateDraft, (current) => ({
                ...current,
                enabled,
              }));
            }}
          />
          <Field>
            <FieldLabel htmlFor="ai-vision-provider">
              Vision provider
            </FieldLabel>
            <Select
              onValueChange={(value) => {
                const provider = value as AiVisionProviderKind;
                updateAiVision(onUpdateDraft, (current) => ({
                  ...current,
                  apiKeyRef:
                    provider === 'openai-compatible' ? current.apiKeyRef : null,
                  provider,
                }));
              }}
              value={vision.provider}
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
                baseUrl={vision.baseUrl ?? ''}
                hasSavedKey={Boolean(vision.apiKeyRef)}
                keyDraft={visionKeyDraft}
                onBaseUrlChange={(value) => {
                  updateAiVision(onUpdateDraft, (current) => ({
                    ...current,
                    baseUrl: value || null,
                  }));
                }}
                onKeyChange={onChangeVisionKey}
                prefix="vision"
              />
              <Field>
                <FieldLabel htmlFor="ai-vision-model">Vision model</FieldLabel>
                <Input
                  id="ai-vision-model"
                  onChange={(event) => {
                    updateAiVision(onUpdateDraft, (current) => ({
                      ...current,
                      model: event.target.value || null,
                    }));
                  }}
                  placeholder="Model name"
                  value={vision.model ?? ''}
                />
              </Field>
            </>
          ) : null}
        </FieldGroup>
      </CardContent>
    </Card>
  );
}
