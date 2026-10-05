import type { AppSettings } from '@daydream/shared';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
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
    <Card size="sm">
      <CardHeader>
        <CardTitle>Workspace preferences</CardTitle>
        <CardDescription>
          These options affect this Daydream installation only.
        </CardDescription>
      </CardHeader>
      <CardContent className="divide-y">
        <div className="py-3 first:pt-0">
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
        </div>
        <div className="py-3 last:pb-0">
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
      </CardContent>
    </Card>
  );
}
