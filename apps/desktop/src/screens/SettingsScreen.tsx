import { useEffect, useMemo, useState } from 'react';
import type {
  AppSettings,
  AppSettingsLimits,
  AppSettingsStorageCleanupPreview,
} from '@daydream/shared';
import { RotateCcw, Save } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  estimateOneHourRecordingBytes,
  formatFps,
  formatGiB,
  formatSceneDimensions,
  formatStorageSize,
  gibToBytes,
} from '@/features/settings/settings-format';
import { useSettingsStore } from '@/features/settings/settings-store';

type CadenceMode = 'fps' | 'seconds';
type CleanupConfirmationAction = 'reset' | 'save';

interface CleanupConfirmation {
  action: CleanupConfirmationAction;
  preview: AppSettingsStorageCleanupPreview;
}

export function SettingsScreen() {
  const draft = useSettingsStore((state) => state.draft);
  const error = useSettingsStore((state) => state.error);
  const isDirty = useSettingsStore((state) => state.isDirty);
  const load = useSettingsStore((state) => state.load);
  const loadStatus = useSettingsStore((state) => state.loadStatus);
  const mutationStatus = useSettingsStore((state) => state.mutationStatus);
  const previewStorageCleanup = useSettingsStore(
    (state) => state.previewStorageCleanup,
  );
  const reset = useSettingsStore((state) => state.reset);
  const save = useSettingsStore((state) => state.save);
  const savedScene = useSettingsStore((state) => state.savedScene);
  const setDraft = useSettingsStore((state) => state.setDraft);
  const settingsState = useSettingsStore((state) => state.settingsState);
  const successMessage = useSettingsStore((state) => state.successMessage);
  const [cadenceMode, setCadenceMode] = useState<CadenceMode | null>(null);
  const [cleanupConfirmation, setCleanupConfirmation] =
    useState<CleanupConfirmation | null>(null);
  const [cleanupPreviewing, setCleanupPreviewing] = useState(false);
  const [resetOpen, setResetOpen] = useState(false);

  useEffect(() => {
    if (loadStatus === 'idle') {
      void load();
    }
  }, [load, loadStatus]);

  const validationError = useMemo(
    () =>
      draft && settingsState
        ? validateDraftSettings(draft, settingsState.limits)
        : null,
    [draft, settingsState],
  );
  const saving = mutationStatus === 'loading';
  const busy = saving || cleanupPreviewing;
  const canSave = Boolean(draft && isDirty && !validationError && !busy);

  function updateDraft(updater: (draft: AppSettings) => AppSettings) {
    if (!draft) {
      return;
    }

    setDraft(updater(draft));
  }

  async function requestSave() {
    if (!draft) {
      return;
    }

    await runWithCleanupConfirmation('save', draft);
  }

  async function requestReset() {
    if (!settingsState) {
      return;
    }

    await runWithCleanupConfirmation('reset', settingsState.defaults);
  }

  async function runWithCleanupConfirmation(
    action: CleanupConfirmationAction,
    settings: AppSettings,
  ) {
    setCleanupPreviewing(true);
    try {
      const preview = await previewStorageCleanup(settings);
      if (!preview) {
        return;
      }
      if (preview.recordingCount > 0) {
        if (action === 'reset') {
          setResetOpen(false);
        }
        setCleanupConfirmation({ action, preview });
        return;
      }

      if (action === 'save') {
        await save();
      } else {
        await confirmReset();
      }
    } finally {
      setCleanupPreviewing(false);
    }
  }

  async function confirmReset() {
    await reset();
    setCadenceMode(null);
    setResetOpen(false);
  }

  async function confirmCleanup() {
    const action = cleanupConfirmation?.action;
    if (!action) {
      return;
    }

    setCleanupConfirmation(null);
    if (action === 'save') {
      await save();
    } else {
      await confirmReset();
    }
  }

  return (
    <section className="mx-auto flex max-w-5xl flex-col gap-6 p-6">
      <div className="flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-normal">Settings</h1>
          <p className="text-muted-foreground mt-2 max-w-3xl text-sm leading-6">
            Capture and library defaults stored locally in Daydream.
          </p>
        </div>

        <div className="flex flex-wrap gap-2 md:justify-end">
          <Dialog open={resetOpen} onOpenChange={setResetOpen}>
            <DialogTrigger asChild>
              <Button disabled={busy || !draft} variant="outline">
                <RotateCcw className="size-3.5" />
                Reset
              </Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Reset settings?</DialogTitle>
                <DialogDescription>
                  This restores capture and storage settings to defaults. It
                  does not delete recordings or scenes.
                </DialogDescription>
              </DialogHeader>
              <DialogFooter>
                <DialogClose asChild>
                  <Button disabled={busy} variant="outline">
                    Cancel
                  </Button>
                </DialogClose>
                <Button
                  disabled={busy}
                  onClick={() => void requestReset()}
                  variant="destructive"
                >
                  Reset settings
                </Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>
          <Button disabled={!canSave} onClick={() => void requestSave()}>
            <Save className="size-3.5" />
            Save
          </Button>
        </div>
      </div>

      <Dialog
        open={Boolean(cleanupConfirmation)}
        onOpenChange={(open) => {
          if (!open && !saving) {
            setCleanupConfirmation(null);
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete old recordings?</DialogTitle>
            <DialogDescription>
              This would delete{' '}
              {cleanupConfirmation
                ? formatSessionCount(cleanupConfirmation.preview.recordingCount)
                : '0 sessions'}{' '}
              ({formatStorageSize(cleanupConfirmation?.preview.sizeBytes ?? 0)})
              from the local library.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              disabled={saving}
              onClick={() => setCleanupConfirmation(null)}
              variant="outline"
            >
              Cancel
            </Button>
            <Button
              disabled={saving}
              onClick={() => void confirmCleanup()}
              variant="destructive"
            >
              {cleanupConfirmation?.action === 'reset'
                ? 'Delete and reset'
                : 'Delete and save'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {error ? (
        <Alert variant="destructive">
          <AlertTitle>Settings failed</AlertTitle>
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      ) : null}
      {successMessage ? (
        <p aria-live="polite" className="text-muted-foreground text-xs">
          {successMessage}
        </p>
      ) : null}
      {validationError ? (
        <p className="text-destructive text-xs" role="alert">
          {validationError}
        </p>
      ) : null}

      {!draft || !settingsState ? (
        <div className="text-muted-foreground border p-4 text-sm">
          {loadStatus === 'error'
            ? 'Settings are unavailable.'
            : 'Loading settings.'}
        </div>
      ) : (
        <Tabs defaultValue="record">
          <TabsList aria-label="Settings sections">
            <TabsTrigger value="record">Record</TabsTrigger>
            <TabsTrigger value="storage">Storage/Library</TabsTrigger>
          </TabsList>

          <TabsContent value="record">
            <RecordSettingsTab
              cadenceMode={cadenceMode ?? cadenceModeForFps(draft.record.fps)}
              draft={draft}
              limits={settingsState.limits}
              sceneDimensions={formatSceneDimensions(savedScene)}
              oneHourEstimate={formatStorageSize(
                estimateOneHourRecordingBytes(draft, savedScene),
              )}
              onChangeCadenceMode={setCadenceMode}
              onUpdateDraft={updateDraft}
            />
          </TabsContent>

          <TabsContent value="storage">
            <StorageSettingsTab
              draft={draft}
              limits={settingsState.limits}
              onUpdateDraft={updateDraft}
            />
          </TabsContent>
        </Tabs>
      )}
    </section>
  );
}

interface RecordSettingsTabProps {
  cadenceMode: CadenceMode;
  draft: AppSettings;
  limits: AppSettingsLimits;
  oneHourEstimate: string;
  onChangeCadenceMode: (mode: CadenceMode) => void;
  onUpdateDraft: (updater: (draft: AppSettings) => AppSettings) => void;
  sceneDimensions: string;
}

function RecordSettingsTab({
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
            <FieldLabel>Frame cadence</FieldLabel>
            <div className="flex flex-wrap gap-2">
              <Button
                aria-pressed={cadenceMode === 'seconds'}
                data-variant={
                  cadenceMode === 'seconds' ? 'secondary' : 'outline'
                }
                onClick={() => onChangeCadenceMode('seconds')}
                type="button"
                variant={cadenceMode === 'seconds' ? 'secondary' : 'outline'}
              >
                One frame every
              </Button>
              <Button
                aria-pressed={cadenceMode === 'fps'}
                data-variant={cadenceMode === 'fps' ? 'secondary' : 'outline'}
                onClick={() => onChangeCadenceMode('fps')}
                type="button"
                variant={cadenceMode === 'fps' ? 'secondary' : 'outline'}
              >
                Frames per second
              </Button>
            </div>
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

interface StorageSettingsTabProps {
  draft: AppSettings;
  limits: AppSettingsLimits;
  onUpdateDraft: (updater: (draft: AppSettings) => AppSettings) => void;
}

function StorageSettingsTab({
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

function SummaryMetric({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-muted-foreground text-xs">{label}</p>
      <p className="mt-1 text-sm font-medium">{value}</p>
    </div>
  );
}

function validateDraftSettings(
  draft: AppSettings,
  limits: AppSettingsLimits,
): string | null {
  if (
    !Number.isFinite(draft.record.fps) ||
    draft.record.fps < limits.record.fpsMin ||
    draft.record.fps > limits.record.fpsMax
  ) {
    return 'Frame cadence must be between one frame every 60 seconds and 5 fps.';
  }
  if (
    draft.record.videoBitrateKbps < limits.record.videoBitrateKbpsMin ||
    draft.record.videoBitrateKbps > limits.record.videoBitrateKbpsMax
  ) {
    return 'Video bitrate must be between 250 and 10000 kbps.';
  }
  if (
    draft.storage.retentionDays < limits.storage.retentionDaysMin ||
    draft.storage.retentionDays > limits.storage.retentionDaysMax
  ) {
    return 'Retention must be between 1 and 365 days.';
  }
  if (draft.storage.sizeLimitBytes < limits.storage.sizeLimitBytesMin) {
    return 'Storage size limit must be at least 1 GiB.';
  }

  return null;
}

function formatInputNumber(value: number): string {
  if (!Number.isFinite(value)) {
    return '';
  }

  return Number.isInteger(value) ? value.toString() : value.toFixed(2);
}

function formatSessionCount(count: number): string {
  return `${count} session${count === 1 ? '' : 's'}`;
}

function cadenceModeForFps(fps: number): CadenceMode {
  return fps < 1 ? 'seconds' : 'fps';
}
