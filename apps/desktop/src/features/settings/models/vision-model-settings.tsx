import type { AppSettings } from '@daydream/shared';
import { CheckboxField } from '../components/ai-settings-fields';
import type { UpdateSettingsDraft } from '../settings-draft-updaters';
import { updateAiVision } from '../settings-draft-updaters';
import { ModelSelector } from './model-selector';

export function VisionModelSettings({
  draft,
  onUpdateDraft,
}: {
  draft: AppSettings;
  onUpdateDraft: UpdateSettingsDraft;
}) {
  return (
    <div className="grid gap-4">
      <ModelSelector
        draft={draft}
        feature="vision"
        capability="vision"
        label="OCR and Vision model"
        onUpdateDraft={onUpdateDraft}
      />
      <CheckboxField
        checked={draft.ai.vision.provider === 'openai-compatible'}
        label="Allow remote screen processing"
        description="When Vision OCR or screen observations are enabled, sampled screen images and recognized text are sent to the selected model. This consent is separate from summaries and embeddings."
        onCheckedChange={(enabled) =>
          updateAiVision(onUpdateDraft, (current) => ({
            ...current,
            provider: enabled ? 'openai-compatible' : 'disabled',
          }))
        }
      />
      <CheckboxField
        checked={draft.ai.vision.observationsEnabled}
        label="Generate screen observations"
        description="Describe activity in sampled screen frames using the selected Vision model."
        onCheckedChange={(observationsEnabled) =>
          updateAiVision(onUpdateDraft, (current) => ({
            ...current,
            observationsEnabled,
          }))
        }
      />
      <p className="text-muted-foreground text-xs">
        Model setup and synthetic tests are in Models. Tesseract and audio
        transcription remain local unless their own remote options are selected.
      </p>
    </div>
  );
}
