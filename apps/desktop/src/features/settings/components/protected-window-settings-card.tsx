import { useEffect, useState } from 'react';
import type {
  AppSettings,
  AppSettingsLimits,
  ProtectedWindowAction,
  ProtectedWindowRule,
  ProtectedWindowRuleAction,
  ProtectedWindowTitleMatcherKind,
  ProtectableWindow,
  ProtectableWindowList,
  WindowProtectionStatus,
} from '@daydream/shared';
import { Plus, RefreshCw, Trash2 } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  getWindowProtectionStatus,
  listProtectableWindows,
} from '@/features/recorder/recorder-client';

interface ProtectedWindowSettingsCardProps {
  draft: AppSettings;
  limits: AppSettingsLimits;
  onUpdateDraft: (updater: (draft: AppSettings) => AppSettings) => void;
}

export function ProtectedWindowSettingsCard({
  draft,
  limits,
  onUpdateDraft,
}: ProtectedWindowSettingsCardProps) {
  const [pickerOpen, setPickerOpen] = useState(false);
  const [picker, setPicker] = useState<ProtectableWindowList | null>(null);
  const [pickerLoading, setPickerLoading] = useState(false);
  const [providerStatus, setProviderStatus] =
    useState<WindowProtectionStatus | null>(null);
  const settings = draft.record.windowProtection;
  const hasEnabledRules = settings.rules.some((rule) => rule.enabled);
  const protectionUnavailable =
    settings.enabled &&
    hasEnabledRules &&
    providerStatus !== null &&
    providerStatus.status !== 'available';

  async function refreshPicker() {
    setPickerLoading(true);
    try {
      const [nextPicker, nextStatus] = await Promise.all([
        listProtectableWindows(),
        getWindowProtectionStatus(),
      ]);
      setPicker(nextPicker);
      setProviderStatus(nextStatus);
    } finally {
      setPickerLoading(false);
    }
  }

  useEffect(() => {
    let cancelled = false;
    void getWindowProtectionStatus().then((status) => {
      if (!cancelled) {
        setProviderStatus(status);
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);

  function updateWindowProtection(
    updater: (
      current: AppSettings['record']['windowProtection'],
    ) => AppSettings['record']['windowProtection'],
  ) {
    onUpdateDraft((current) => ({
      ...current,
      record: {
        ...current.record,
        windowProtection: updater(current.record.windowProtection),
      },
    }));
  }

  function addRule(rule: ProtectedWindowRule) {
    updateWindowProtection((current) => ({
      ...current,
      rules: [...current.rules, rule],
    }));
  }

  function updateRule(
    ruleId: string,
    updater: (rule: ProtectedWindowRule) => ProtectedWindowRule,
  ) {
    updateWindowProtection((current) => ({
      ...current,
      rules: current.rules.map((rule) =>
        rule.id === ruleId ? updater(rule) : rule,
      ),
    }));
  }

  return (
    <FieldGroup>
      <Field orientation="horizontal">
        <FieldContent>
          <FieldTitle>Protected windows</FieldTitle>
          <FieldDescription>
            Pause or redact when matching app windows are focused.
          </FieldDescription>
        </FieldContent>
        <Checkbox
          aria-label="Enable protected windows"
          checked={settings.enabled}
          onCheckedChange={(checked) => {
            updateWindowProtection((current) => ({
              ...current,
              enabled: checked === true,
            }));
          }}
        />
      </Field>

      {protectionUnavailable ? (
        <Alert variant="destructive">
          <AlertTitle>Protected-window automation unavailable</AlertTitle>
          <AlertDescription>{providerStatus.message}</AlertDescription>
        </Alert>
      ) : null}

      <Field>
        <FieldLabel id="protected-window-default-action">
          Default behavior
        </FieldLabel>
        <RadioGroup
          aria-labelledby="protected-window-default-action"
          className="max-w-sm gap-2"
          onValueChange={(value) => {
            updateWindowProtection((current) => ({
              ...current,
              defaultAction: value as ProtectedWindowAction,
            }));
          }}
          value={settings.defaultAction}
        >
          <FieldLabel htmlFor="protected-window-default-pause">
            <Field orientation="horizontal">
              <FieldContent>
                <FieldTitle>Pause capture</FieldTitle>
                <FieldDescription>
                  Do not write protected spans after the dwell threshold.
                </FieldDescription>
              </FieldContent>
              <RadioGroupItem
                id="protected-window-default-pause"
                value="pause"
              />
            </Field>
          </FieldLabel>
          <FieldLabel htmlFor="protected-window-default-redact">
            <Field orientation="horizontal">
              <FieldContent>
                <FieldTitle>Record then redact</FieldTitle>
                <FieldDescription>
                  Keep capture continuous and create all-track redactions.
                </FieldDescription>
              </FieldContent>
              <RadioGroupItem
                id="protected-window-default-redact"
                value="redact"
              />
            </Field>
          </FieldLabel>
        </RadioGroup>
      </Field>

      <div className="grid gap-3 md:grid-cols-2">
        <NumberSetting
          label="Focus dwell"
          max={limits.record.windowProtectionDwellMsMax}
          min={limits.record.windowProtectionDwellMsMin}
          onChange={(dwellMs) => {
            updateWindowProtection((current) => ({ ...current, dwellMs }));
          }}
          value={settings.dwellMs}
        />
        <NumberSetting
          label="Merge gap"
          max={limits.record.windowProtectionMergeGapMsMax}
          min={limits.record.windowProtectionMergeGapMsMin}
          onChange={(mergeGapMs) => {
            updateWindowProtection((current) => ({ ...current, mergeGapMs }));
          }}
          value={settings.mergeGapMs}
        />
      </div>

      <div className="grid gap-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <p className="text-sm font-medium">Rules</p>
          <div className="flex flex-wrap gap-2">
            <Button
              onClick={() => addRule(blankRule())}
              size="sm"
              type="button"
              variant="outline"
            >
              <Plus className="size-3.5" />
              Add rule
            </Button>
            <Dialog
              open={pickerOpen}
              onOpenChange={(open) => {
                setPickerOpen(open);
                if (open) {
                  void refreshPicker();
                }
              }}
            >
              <DialogTrigger asChild>
                <Button size="sm" type="button" variant="outline">
                  Pick window
                </Button>
              </DialogTrigger>
              <DialogContent>
                <DialogHeader>
                  <DialogTitle>Pick a window</DialogTitle>
                  <DialogDescription>
                    Add a rule from currently open app windows.
                  </DialogDescription>
                </DialogHeader>
                <WindowPickerContent
                  loading={pickerLoading}
                  onPick={(window) => {
                    addRule(ruleFromWindow(window));
                    setPickerOpen(false);
                  }}
                  onRefresh={() => void refreshPicker()}
                  picker={picker}
                  status={providerStatus}
                />
              </DialogContent>
            </Dialog>
          </div>
        </div>

        {settings.rules.length === 0 ? (
          <p className="text-muted-foreground border p-3 text-sm">
            No protected-window rules configured.
          </p>
        ) : (
          settings.rules.map((rule) => (
            <ProtectedWindowRuleEditor
              key={rule.id}
              maxTextChars={limits.record.windowProtectionTextMaxChars}
              onDelete={() => {
                updateWindowProtection((current) => ({
                  ...current,
                  rules: current.rules.filter((item) => item.id !== rule.id),
                }));
              }}
              onUpdate={(updater) => updateRule(rule.id, updater)}
              rule={rule}
            />
          ))
        )}
      </div>
    </FieldGroup>
  );
}

function ProtectedWindowRuleEditor({
  maxTextChars,
  onDelete,
  onUpdate,
  rule,
}: {
  maxTextChars: number;
  onDelete: () => void;
  onUpdate: (
    updater: (rule: ProtectedWindowRule) => ProtectedWindowRule,
  ) => void;
  rule: ProtectedWindowRule;
}) {
  return (
    <div className="grid gap-3 border p-3">
      <div className="flex items-center justify-between gap-3">
        <label className="flex items-center gap-2 text-sm font-medium">
          <Checkbox
            aria-label={`Enable ${rule.label || 'protected window rule'}`}
            checked={rule.enabled}
            onCheckedChange={(checked) =>
              onUpdate((current) => ({ ...current, enabled: checked === true }))
            }
          />
          Enabled
        </label>
        <Button
          aria-label={`Delete ${rule.label || 'protected window rule'}`}
          onClick={onDelete}
          size="icon"
          type="button"
          variant="ghost"
        >
          <Trash2 className="size-3.5" />
        </Button>
      </div>

      <div className="grid gap-3 md:grid-cols-2">
        <TextSetting
          label="Label"
          maxLength={maxTextChars}
          onChange={(label) => onUpdate((current) => ({ ...current, label }))}
          value={rule.label}
        />
        <SelectSetting
          label="Action"
          onChange={(action) =>
            onUpdate((current) => ({
              ...current,
              action: action as ProtectedWindowRuleAction,
            }))
          }
          options={[
            ['inherit', 'Use default'],
            ['pause', 'Pause'],
            ['redact', 'Redact'],
          ]}
          value={rule.action}
        />
        <TextSetting
          label="App identity"
          maxLength={maxTextChars}
          onChange={(appIdentity) =>
            onUpdate((current) => ({
              ...current,
              appIdentity: emptyToNull(appIdentity),
            }))
          }
          value={rule.appIdentity ?? ''}
        />
        <TextSetting
          label="Process"
          maxLength={maxTextChars}
          onChange={(processName) =>
            onUpdate((current) => ({
              ...current,
              processName: emptyToNull(processName),
            }))
          }
          value={rule.processName ?? ''}
        />
        <SelectSetting
          label="Title matcher"
          onChange={(kind) =>
            onUpdate((current) => ({
              ...current,
              titleMatcher: {
                caseSensitive: current.titleMatcher?.caseSensitive ?? false,
                kind: kind as ProtectedWindowTitleMatcherKind,
                value: current.titleMatcher?.value ?? '',
              },
            }))
          }
          options={[
            ['contains', 'Contains'],
            ['regex', 'Regex'],
          ]}
          value={rule.titleMatcher?.kind ?? 'contains'}
        />
        <TextSetting
          label="Title value"
          maxLength={maxTextChars}
          onChange={(value) =>
            onUpdate((current) => ({
              ...current,
              titleMatcher:
                value.trim().length === 0
                  ? null
                  : {
                      caseSensitive:
                        current.titleMatcher?.caseSensitive ?? false,
                      kind: current.titleMatcher?.kind ?? 'contains',
                      value,
                    },
            }))
          }
          value={rule.titleMatcher?.value ?? ''}
        />
      </div>
    </div>
  );
}

function WindowPickerContent({
  loading,
  onPick,
  onRefresh,
  picker,
  status,
}: {
  loading: boolean;
  onPick: (window: ProtectableWindow) => void;
  onRefresh: () => void;
  picker: ProtectableWindowList | null;
  status: WindowProtectionStatus | null;
}) {
  if (!picker || loading) {
    return (
      <div className="grid gap-3">
        <p className="text-muted-foreground text-sm">Loading windows.</p>
        <Button onClick={onRefresh} type="button" variant="outline">
          <RefreshCw className="size-3.5" />
          Refresh
        </Button>
      </div>
    );
  }

  if (picker.status !== 'available') {
    return (
      <div className="grid gap-3">
        <Alert variant="destructive">
          <AlertTitle>Window picker unavailable</AlertTitle>
          <AlertDescription>
            {status?.message ?? unavailablePickerMessage(picker.status)}
          </AlertDescription>
        </Alert>
        <Button onClick={onRefresh} type="button" variant="outline">
          <RefreshCw className="size-3.5" />
          Refresh
        </Button>
      </div>
    );
  }

  return (
    <div className="grid max-h-80 gap-2 overflow-auto">
      {picker.windows.map((window) => (
        <Button
          className="h-auto justify-start whitespace-normal"
          key={window.id}
          onClick={() => onPick(window)}
          type="button"
          variant="outline"
        >
          {window.processName ?? window.appIdentity ?? 'Unknown app'}
          {window.title ? ` - ${window.title}` : ''}
        </Button>
      ))}
    </div>
  );
}

function unavailablePickerMessage(status: ProtectableWindowList['status']) {
  switch (status) {
    case 'permission-denied':
      return 'Focused-window detection needs desktop permission before windows can be listed.';
    case 'provider-unavailable':
      return 'Focused-window detection is not available in this desktop session.';
    case 'session-unsupported':
      return 'Focused-window detection is not supported in this desktop session.';
    case 'unsupported-platform':
      return 'Focused-window detection is not implemented on this platform.';
    case 'available':
      return 'Focused-window detection is available.';
  }
}

function NumberSetting({
  label,
  max,
  min,
  onChange,
  value,
}: {
  label: string;
  max: number;
  min: number;
  onChange: (value: number) => void;
  value: number;
}) {
  return (
    <Field>
      <FieldLabel>{label}</FieldLabel>
      <div className="flex items-center gap-2">
        <Input
          max={max}
          min={min}
          onChange={(event) => onChange(Math.round(Number(event.target.value)))}
          step={250}
          type="number"
          value={value}
        />
        <span className="text-muted-foreground shrink-0 text-xs">ms</span>
      </div>
    </Field>
  );
}

function TextSetting({
  label,
  maxLength,
  onChange,
  value,
}: {
  label: string;
  maxLength: number;
  onChange: (value: string) => void;
  value: string;
}) {
  return (
    <Field>
      <FieldLabel>{label}</FieldLabel>
      <Input
        maxLength={maxLength}
        onChange={(event) => onChange(event.target.value)}
        value={value}
      />
    </Field>
  );
}

function SelectSetting({
  label,
  onChange,
  options,
  value,
}: {
  label: string;
  onChange: (value: string) => void;
  options: Array<[string, string]>;
  value: string;
}) {
  return (
    <Field>
      <FieldLabel>{label}</FieldLabel>
      <Select onValueChange={onChange} value={value}>
        <SelectTrigger>
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {options.map(([optionValue, optionLabel]) => (
            <SelectItem key={optionValue} value={optionValue}>
              {optionLabel}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </Field>
  );
}

function blankRule(): ProtectedWindowRule {
  return {
    action: 'inherit',
    appIdentity: null,
    enabled: true,
    id: crypto.randomUUID(),
    label: 'Protected window',
    processName: null,
    titleMatcher: null,
  };
}

function ruleFromWindow(window: ProtectableWindow): ProtectedWindowRule {
  return {
    action: 'inherit',
    appIdentity: window.appIdentity,
    enabled: true,
    id: crypto.randomUUID(),
    label: window.processName ?? window.appIdentity ?? 'Protected window',
    processName: window.processName,
    titleMatcher: window.title
      ? {
          caseSensitive: false,
          kind: 'contains',
          value: window.title,
        }
      : null,
  };
}

function emptyToNull(value: string): string | null {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}
