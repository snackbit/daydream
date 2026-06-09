import type { AppSettings, AppSettingsLimits } from '@daydream/shared';
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
import { formatGiB, gibToBytes } from '@/features/settings/settings-format';

interface StorageSettingsTabProps {
  draft: AppSettings;
  limits: AppSettingsLimits;
  onUpdateDraft: (updater: (draft: AppSettings) => AppSettings) => void;
}

export function StorageSettingsTab({
  draft,
  limits,
  onUpdateDraft,
}: StorageSettingsTabProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Storage/Library</CardTitle>
        <CardDescription>
          Local retention and storage cap for the recording library.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <FieldGroup>
          <Field>
            <FieldLabel htmlFor="settings-retention-days">
              Recording retention
            </FieldLabel>
            <div className="flex max-w-sm items-center gap-2">
              <Input
                id="settings-retention-days"
                max={limits.storage.retentionDaysMax}
                min={limits.storage.retentionDaysMin}
                onChange={(event) => {
                  const value = Math.round(Number(event.target.value));
                  onUpdateDraft((current) => ({
                    ...current,
                    storage: {
                      ...current.storage,
                      retentionDays: value,
                    },
                  }));
                }}
                step={1}
                type="number"
                value={draft.storage.retentionDays}
              />
              <span className="text-muted-foreground shrink-0 text-xs">
                days
              </span>
            </div>
            <FieldDescription>
              Range: {limits.storage.retentionDaysMin}-
              {limits.storage.retentionDaysMax} days.
            </FieldDescription>
          </Field>

          <Field>
            <FieldLabel htmlFor="settings-storage-limit">
              Storage size limit
            </FieldLabel>
            <div className="flex max-w-sm items-center gap-2">
              <Input
                id="settings-storage-limit"
                min={formatGiB(limits.storage.sizeLimitBytesMin)}
                onChange={(event) => {
                  const value = gibToBytes(Number(event.target.value));
                  onUpdateDraft((current) => ({
                    ...current,
                    storage: {
                      ...current.storage,
                      sizeLimitBytes: value,
                    },
                  }));
                }}
                step={1}
                type="number"
                value={formatGiB(draft.storage.sizeLimitBytes)}
              />
              <span className="text-muted-foreground shrink-0 text-xs">
                GiB
              </span>
            </div>
            <FieldDescription>
              Minimum 1 GiB. No fixed maximum is enforced.
            </FieldDescription>
          </Field>
        </FieldGroup>
      </CardContent>
    </Card>
  );
}
