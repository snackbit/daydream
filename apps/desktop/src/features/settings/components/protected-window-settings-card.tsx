import { useCallback, useEffect, useState } from 'react';
import type {
  AppSettings,
  AppSettingsLimits,
  ProtectedWindowRule,
  ProtectableWindow,
  WindowProtectionStatus,
} from '@daydream/shared';
import { Plus } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import {
  Field,
  FieldContent,
  FieldDescription,
  FieldGroup,
  FieldLabel,
  FieldTitle,
} from '@/components/ui/field';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { getWindowProtectionStatus } from '@/features/recorder/recorder-client';
import { NumberSetting } from './protected-windows/number-setting';
import { ProtectedWindowPicker } from './protected-windows/protected-window-picker';
import { ProtectedWindowRuleEditor } from './protected-windows/protected-window-rule-editor';

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
  const [providerStatus, setProviderStatus] =
    useState<WindowProtectionStatus | null>(null);
  const [providerError, setProviderError] = useState(false);
  const settings = draft.record.windowProtection;
  const hasEnabledRules = settings.rules.some((rule) => rule.enabled);
  const protectionUnavailable =
    settings.enabled &&
    hasEnabledRules &&
    providerStatus !== null &&
    providerStatus.status !== 'available';

  useEffect(() => {
    let cancelled = false;
    void getWindowProtectionStatus()
      .then((status) => {
        if (!cancelled) {
          setProviderStatus(status);
        }
      })
      .catch(() => {
        if (!cancelled) setProviderError(true);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const updateWindowProtection = useCallback(
    (
      updater: (
        current: AppSettings['record']['windowProtection'],
      ) => AppSettings['record']['windowProtection'],
    ) => {
      onUpdateDraft((current) => ({
        ...current,
        record: {
          ...current.record,
          windowProtection: updater(current.record.windowProtection),
        },
      }));
    },
    [onUpdateDraft],
  );

  const addRule = useCallback(
    (rule: ProtectedWindowRule) => {
      updateWindowProtection((current) => ({
        ...current,
        rules: [...current.rules, rule],
      }));
    },
    [updateWindowProtection],
  );

  const handlePickWindow = useCallback(
    (window: ProtectableWindow) => addRule(ruleFromWindow(window)),
    [addRule],
  );
  const handleProviderStatus = useCallback((status: WindowProtectionStatus) => {
    setProviderStatus(status);
    setProviderError(false);
  }, []);

  const updateRule = useCallback(
    (
      ruleId: string,
      updater: (rule: ProtectedWindowRule) => ProtectedWindowRule,
    ) => {
      updateWindowProtection((current) => ({
        ...current,
        rules: current.rules.map((rule) =>
          rule.id === ruleId ? updater(rule) : rule,
        ),
      }));
    },
    [updateWindowProtection],
  );
  const handleDeleteRule = useCallback(
    (ruleId: string) =>
      updateWindowProtection((current) => ({
        ...current,
        rules: current.rules.filter((item) => item.id !== ruleId),
      })),
    [updateWindowProtection],
  );
  const handleAddRule = useCallback(() => addRule(blankRule()), [addRule]);
  const handleEnabled = useCallback(
    (checked: boolean | 'indeterminate') =>
      updateWindowProtection((current) => ({
        ...current,
        enabled: checked === true,
      })),
    [updateWindowProtection],
  );
  const handleDefaultAction = useCallback(
    (defaultAction: string) => {
      if (defaultAction === 'pause' || defaultAction === 'redact')
        updateWindowProtection((current) => ({ ...current, defaultAction }));
    },
    [updateWindowProtection],
  );
  const handleDwell = useCallback(
    (dwellMs: number) =>
      updateWindowProtection((current) => ({ ...current, dwellMs })),
    [updateWindowProtection],
  );
  const handleMergeGap = useCallback(
    (mergeGapMs: number) =>
      updateWindowProtection((current) => ({ ...current, mergeGapMs })),
    [updateWindowProtection],
  );

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
          onCheckedChange={handleEnabled}
        />
      </Field>

      {protectionUnavailable ? (
        <Alert variant="destructive">
          <AlertTitle>Protected-window automation unavailable</AlertTitle>
          <AlertDescription>{providerStatus.message}</AlertDescription>
        </Alert>
      ) : null}
      {providerError && settings.enabled && hasEnabledRules ? (
        <Alert variant="destructive">
          <AlertTitle>
            Protected-window availability could not be checked
          </AlertTitle>
          <AlertDescription>
            Recheck in the window picker before recording.
          </AlertDescription>
        </Alert>
      ) : null}

      <Field>
        <FieldLabel id="protected-window-default-action">
          Default behavior
        </FieldLabel>
        <RadioGroup
          aria-labelledby="protected-window-default-action"
          className="max-w-sm gap-2"
          onValueChange={handleDefaultAction}
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
          onChange={handleDwell}
          value={settings.dwellMs}
        />
        <NumberSetting
          label="Merge gap"
          max={limits.record.windowProtectionMergeGapMsMax}
          min={limits.record.windowProtectionMergeGapMsMin}
          onChange={handleMergeGap}
          value={settings.mergeGapMs}
        />
      </div>

      <div className="grid gap-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <p className="text-sm font-medium">Rules</p>
          <div className="flex flex-wrap gap-2">
            <Button
              onClick={handleAddRule}
              size="sm"
              type="button"
              variant="outline"
            >
              <Plus className="size-3.5" />
              Add rule
            </Button>
            <ProtectedWindowPicker
              onPick={handlePickWindow}
              onStatusLoaded={handleProviderStatus}
            />
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
              onDelete={handleDeleteRule}
              onUpdate={updateRule}
              rule={rule}
            />
          ))
        )}
      </div>
    </FieldGroup>
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
