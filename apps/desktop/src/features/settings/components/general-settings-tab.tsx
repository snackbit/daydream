import type { AppSettings } from '@daydream/shared';
import { CheckboxField } from './ai-settings-fields';

interface GeneralSettingsTabProps {
  draft: AppSettings;
  onUpdateDraft: (updater: (draft: AppSettings) => AppSettings) => void;
}

export function GeneralSettingsTab({
  draft,
  onUpdateDraft,
}: GeneralSettingsTabProps) {
  return (
    <div className="grid gap-4 py-4">
      <CheckboxField
        checked={draft.expert?.enabled ?? false}
        description="Show advanced tuning controls for local processing."
        label="Enable expert mode"
        onCheckedChange={(enabled) =>
          onUpdateDraft((current) => ({
            ...current,
            expert: {
              ...current.expert,
              enabled,
            },
          }))
        }
      />
      <CheckboxField
        checked={draft.debug?.enabled ?? false}
        description="Show diagnostics routes and per-recording raw debug data."
        label="Enable debug tools"
        onCheckedChange={(enabled) =>
          onUpdateDraft((current) => ({
            ...current,
            debug: {
              ...current.debug,
              enabled,
            },
          }))
        }
      />
    </div>
  );
}
