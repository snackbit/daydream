import { ChevronRight } from 'lucide-react';
import { SharedDisclosure } from '@/components/shared-disclosure';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { InputActivityStatusLine } from '../../components/input-activity-status-line';
import type {
  SettingsScreenTabsActions,
  SettingsScreenTabsModel,
} from '../../screen/settings-screen-tabs';
import { NumberPreference } from '../controls/number-preference';
import { SettingRow } from '../controls/setting-row';

export interface PrivacyPreferencesProps {
  model: SettingsScreenTabsModel;
  actions: SettingsScreenTabsActions;
  onOpenProtection: () => void;
}
export function PrivacyPreferences({
  model,
  actions,
  onOpenProtection,
}: PrivacyPreferencesProps) {
  const { draft, limits } = model;
  function windows(enabled: boolean) {
    actions.updateDraft((current) => ({
      ...current,
      ai: {
        ...current.ai,
        activity: {
          ...current.ai.activity,
          windows: { ...current.ai.activity.windows, enabled },
        },
      },
    }));
  }
  function keyboard(enabled: boolean) {
    actions.updateDraft((current) => ({
      ...current,
      ai: {
        ...current.ai,
        activity: {
          ...current.ai.activity,
          keyboard: { ...current.ai.activity.keyboard, enabled },
        },
      },
    }));
  }
  function mouse(enabled: boolean) {
    actions.updateDraft((current) => ({
      ...current,
      ai: {
        ...current.ai,
        activity: {
          ...current.ai.activity,
          mouse: { ...current.ai.activity.mouse, enabled },
        },
      },
    }));
  }
  function keyboardWindow(value: number) {
    actions.updateDraft((current) => ({
      ...current,
      ai: {
        ...current.ai,
        activity: {
          ...current.ai.activity,
          keyboard: { ...current.ai.activity.keyboard, windowMs: value * 1000 },
        },
      },
    }));
  }
  function mouseWindow(value: number) {
    actions.updateDraft((current) => ({
      ...current,
      ai: {
        ...current.ai,
        activity: {
          ...current.ai.activity,
          mouse: { ...current.ai.activity.mouse, windowMs: value * 1000 },
        },
      },
    }));
  }
  const ruleCount = draft.record.windowProtection.rules.filter(
    (rule) => rule.enabled,
  ).length;
  return (
    <div>
      <SettingRow
        title="Protected windows"
        description="Pause or redact recordings when a matching window is focused."
      >
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={onOpenProtection}
        >
          {draft.record.windowProtection.enabled
            ? `${ruleCount} active rules`
            : 'Set up protection'}
          <ChevronRight aria-hidden="true" className="size-3.5" />
        </Button>
      </SettingRow>
      <h3 className="mt-9 text-sm font-medium">Activity</h3>
      <SettingRow
        title="App and window activity"
        description="Record app identities and window titles."
      >
        <Switch
          aria-label="Use window activity"
          checked={draft.ai.activity.windows.enabled}
          onCheckedChange={windows}
        />
      </SettingRow>
      <SettingRow
        title="Keyboard activity"
        description="Timing only. Typed text and key values are not stored."
      >
        <Switch
          aria-label="Use keyboard activity"
          checked={draft.ai.activity.keyboard.enabled}
          onCheckedChange={keyboard}
        />
      </SettingRow>
      <SettingRow
        title="Mouse activity"
        description="Timing only. Pointer positions and clicked content are not stored."
      >
        <Switch
          aria-label="Use mouse activity"
          checked={draft.ai.activity.mouse.enabled}
          onCheckedChange={mouse}
        />
      </SettingRow>
      {draft.ai.activity.keyboard.enabled || draft.ai.activity.mouse.enabled ? (
        <div className="pt-5">
          <InputActivityStatusLine
            onRefresh={actions.refreshInputActivityStatus}
            onRepair={actions.repairInputActivity}
            repairMessage={model.inputActivityRepairMessage}
            repairing={model.inputActivityRepairing}
            status={model.inputActivityStatus}
          />
        </div>
      ) : null}
      {draft.expert?.enabled ? (
        <div className="pt-6">
          <SharedDisclosure label="Advanced">
            <SettingRow title="Keyboard activity window">
              <NumberPreference
                label="Keyboard activity window"
                value={draft.ai.activity.keyboard.windowMs / 1000}
                min={limits.ai.activity.keyboardWindowMsMin / 1000}
                max={limits.ai.activity.keyboardWindowMsMax / 1000}
                unit="seconds"
                onCommit={keyboardWindow}
              />
            </SettingRow>
            <SettingRow title="Mouse activity window">
              <NumberPreference
                label="Mouse activity window"
                value={draft.ai.activity.mouse.windowMs / 1000}
                min={limits.ai.activity.mouseWindowMsMin / 1000}
                max={limits.ai.activity.mouseWindowMsMax / 1000}
                unit="seconds"
                onCommit={mouseWindow}
              />
            </SettingRow>
          </SharedDisclosure>
        </div>
      ) : null}
    </div>
  );
}
