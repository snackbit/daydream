import type { AppSettings, AiOcrStatus } from '@daydream/shared';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { ModelSelector } from '../../models/model-selector';
import type { UpdateSettingsDraft } from '../../settings-draft-updaters';
import { SettingRow } from '../controls/setting-row';

export function ScreenPreferences({
  kind,
  draft,
  onUpdate,
  ocrStatus,
}: {
  kind: 'ocr' | 'observations';
  draft: AppSettings;
  onUpdate: UpdateSettingsDraft;
  ocrStatus?: AiOcrStatus | undefined;
}) {
  function ocr(value: string) {
    if (value === 'off' || value === 'tesseract' || value === 'vision')
      onUpdate((current) => ({
        ...current,
        ai: {
          ...current.ai,
          ocr: {
            ...current.ai.ocr,
            enabled: value !== 'off',
            engine: value === 'off' ? current.ai.ocr.engine : value,
          },
        },
      }));
  }
  function observations(observationsEnabled: boolean) {
    onUpdate((current) => ({
      ...current,
      ai: {
        ...current.ai,
        vision: { ...current.ai.vision, observationsEnabled },
      },
    }));
  }
  function remote(enabled: boolean) {
    onUpdate((current) => ({
      ...current,
      ai: {
        ...current.ai,
        vision: {
          ...current.ai.vision,
          provider: enabled ? 'openai-compatible' : 'disabled',
        },
      },
    }));
  }
  const needsModel =
    kind === 'observations' ||
    (draft.ai.ocr.enabled && draft.ai.ocr.engine === 'vision');
  return (
    <div className="grid gap-6">
      {kind === 'ocr' ? (
        <SettingRow title="Recognition method">
          <Select
            value={draft.ai.ocr.enabled ? draft.ai.ocr.engine : 'off'}
            onValueChange={ocr}
          >
            <SelectTrigger
              aria-label="Recognition method"
              className="w-full sm:w-56"
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="tesseract">Local · Tesseract</SelectItem>
              <SelectItem value="vision">Remote · Screen model</SelectItem>
              <SelectItem value="off">Off</SelectItem>
            </SelectContent>
          </Select>
        </SettingRow>
      ) : (
        <SettingRow
          title="Screen observations"
          description="Describe activity from sampled screen images."
        >
          <Switch
            aria-label="Enable screen observations"
            checked={draft.ai.vision.observationsEnabled}
            onCheckedChange={observations}
          />
        </SettingRow>
      )}
      {needsModel ? (
        <>
          <ModelSelector
            draft={draft}
            feature="vision"
            capability="vision"
            label="Screen model"
            onUpdateDraft={onUpdate}
          />
          <SettingRow
            title="Allow remote screen processing"
            description="Send sampled screen images and recognized text. This permission is shared by text recognition and screen observations."
          >
            <Switch
              aria-label="Allow remote screen processing"
              checked={draft.ai.vision.provider === 'openai-compatible'}
              onCheckedChange={remote}
            />
          </SettingRow>
        </>
      ) : draft.ai.ocr.enabled ? (
        <p className="text-muted-foreground text-sm">
          {ocrStatus?.runtimeSource === 'system' ||
          ocrStatus?.runtimeSource === 'managed'
            ? `Tesseract is installed${ocrStatus.languages.length ? ` with ${ocrStatus.languages.join(', ')} language data` : ''}.`
            : 'Tesseract needs a local runtime and language data.'}{' '}
          Manage language downloads in Models & accounts. The runtime can use
          your system Tesseract installation.
        </p>
      ) : null}
    </div>
  );
}
