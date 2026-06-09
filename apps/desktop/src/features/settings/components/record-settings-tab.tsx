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
  FieldContent,
  FieldDescription,
  FieldGroup,
  FieldLabel,
  FieldTitle,
} from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { formatFps } from '@/features/settings/settings-format';

export type CadenceMode = 'fps' | 'seconds';

interface RecordSettingsTabProps {
  cadenceMode: CadenceMode;
  draft: AppSettings;
  limits: AppSettingsLimits;
  oneHourEstimate: string;
  onChangeCadenceMode: (mode: CadenceMode) => void;
  onUpdateDraft: (updater: (draft: AppSettings) => AppSettings) => void;
  sceneDimensions: string;
}

export function RecordSettingsTab({
  cadenceMode,
  draft,
  limits,
  oneHourEstimate,
  onChangeCadenceMode,
  onUpdateDraft,
  sceneDimensions,
}: RecordSettingsTabProps) {
  const cadenceValue =
    cadenceMode === 'seconds' ? 1 / draft.record.fps : draft.record.fps;

  return (
    <Card>
      <CardHeader>
        <CardTitle>Record</CardTitle>
        <CardDescription>
          Frame cadence and video bitrate for new recordings.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <FieldGroup>
          <Field>
            <FieldLabel id="settings-frame-cadence-label">
              Frame cadence
            </FieldLabel>
            <RadioGroup
              aria-labelledby="settings-frame-cadence-label"
              className="max-w-sm gap-2"
              onValueChange={(value) =>
                onChangeCadenceMode(value as CadenceMode)
              }
              value={cadenceMode}
            >
              <FieldLabel htmlFor="settings-frame-cadence-seconds">
                <Field orientation="horizontal">
                  <FieldContent>
                    <FieldTitle>One frame every</FieldTitle>
                    <FieldDescription>
                      Enter the number of seconds between captured frames.
                    </FieldDescription>
                  </FieldContent>
                  <RadioGroupItem
                    id="settings-frame-cadence-seconds"
                    value="seconds"
                  />
                </Field>
              </FieldLabel>
              <FieldLabel htmlFor="settings-frame-cadence-fps">
                <Field orientation="horizontal">
                  <FieldContent>
                    <FieldTitle>Frames per second</FieldTitle>
                    <FieldDescription>
                      Enter the number of frames captured each second.
                    </FieldDescription>
                  </FieldContent>
                  <RadioGroupItem id="settings-frame-cadence-fps" value="fps" />
                </Field>
              </FieldLabel>
            </RadioGroup>
            <div className="flex max-w-sm items-center gap-2">
              <Input
                aria-label={
                  cadenceMode === 'seconds'
                    ? 'Seconds per frame'
                    : 'Frames per second'
                }
                max={cadenceMode === 'seconds' ? 60 : limits.record.fpsMax}
                min={cadenceMode === 'seconds' ? 1 : 1}
                onChange={(event) => {
                  const value = Number(event.target.value);
                  onUpdateDraft((current) => ({
                    ...current,
                    record: {
                      ...current.record,
                      fps: cadenceMode === 'seconds' ? 1 / value : value,
                    },
                  }));
                }}
                step={cadenceMode === 'seconds' ? 1 : 0.1}
                type="number"
                value={formatInputNumber(cadenceValue)}
              />
              <span className="text-muted-foreground shrink-0 text-xs">
                {cadenceMode === 'seconds' ? 'seconds' : 'fps'}
              </span>
            </div>
            <FieldDescription>
              Current value: {formatFps(draft.record.fps)}.
            </FieldDescription>
          </Field>

          <Field>
            <FieldLabel htmlFor="settings-video-bitrate">
              Video bitrate
            </FieldLabel>
            <div className="flex max-w-sm items-center gap-2">
              <Input
                id="settings-video-bitrate"
                max={limits.record.videoBitrateKbpsMax}
                min={limits.record.videoBitrateKbpsMin}
                onChange={(event) => {
                  const value = Math.round(Number(event.target.value));
                  onUpdateDraft((current) => ({
                    ...current,
                    record: {
                      ...current.record,
                      videoBitrateKbps: value,
                    },
                  }));
                }}
                step={50}
                type="number"
                value={draft.record.videoBitrateKbps}
              />
              <span className="text-muted-foreground shrink-0 text-xs">
                kbps
              </span>
            </div>
            <FieldDescription>
              Higher bitrate keeps more visual detail but writes larger files.
            </FieldDescription>
          </Field>

          <div className="grid gap-3 border p-4 text-sm md:grid-cols-3">
            <SummaryMetric label="Scene" value={sceneDimensions} />
            <SummaryMetric
              label="Configured cadence"
              value={formatFps(draft.record.fps)}
            />
            <SummaryMetric label="One hour estimate" value={oneHourEstimate} />
          </div>
        </FieldGroup>
      </CardContent>
    </Card>
  );
}

export function cadenceModeForFps(fps: number): CadenceMode {
  return fps < 1 ? 'seconds' : 'fps';
}

function SummaryMetric({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-muted-foreground text-xs">{label}</p>
      <p className="mt-1 text-sm font-medium">{value}</p>
    </div>
  );
}

function formatInputNumber(value: number): string {
  if (!Number.isFinite(value)) {
    return '';
  }

  return Number.isInteger(value) ? value.toString() : value.toFixed(2);
}
