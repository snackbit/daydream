import { useCallback } from 'react';
import type { ProtectedWindowRule } from '@daydream/shared';
import { Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { SelectSetting } from './select-setting';
import { TextSetting } from './text-setting';

export interface ProtectedWindowRuleEditorProps {
  maxTextChars: number;
  onDelete: (ruleId: string) => void;
  onUpdate: (
    ruleId: string,
    updater: (rule: ProtectedWindowRule) => ProtectedWindowRule,
  ) => void;
  rule: ProtectedWindowRule;
}
export function ProtectedWindowRuleEditor({
  maxTextChars,
  onDelete,
  onUpdate,
  rule,
}: ProtectedWindowRuleEditorProps) {
  const update = useCallback(
    (updater: (rule: ProtectedWindowRule) => ProtectedWindowRule) =>
      onUpdate(rule.id, updater),
    [onUpdate, rule.id],
  );
  const handleDelete = useCallback(
    () => onDelete(rule.id),
    [onDelete, rule.id],
  );
  const handleEnabled = useCallback(
    (checked: boolean | 'indeterminate') =>
      update((current) => ({ ...current, enabled: checked === true })),
    [update],
  );
  const handleLabel = useCallback(
    (label: string) => update((current) => ({ ...current, label })),
    [update],
  );
  const handleAction = useCallback(
    (action: string) => {
      if (action === 'inherit' || action === 'pause' || action === 'redact')
        update((current) => ({ ...current, action }));
    },
    [update],
  );
  const handleIdentity = useCallback(
    (appIdentity: string) =>
      update((current) => ({
        ...current,
        appIdentity: emptyToNull(appIdentity),
      })),
    [update],
  );
  const handleProcess = useCallback(
    (processName: string) =>
      update((current) => ({
        ...current,
        processName: emptyToNull(processName),
      })),
    [update],
  );
  const handleTitleKind = useCallback(
    (kind: string) => {
      if (kind === 'contains' || kind === 'regex')
        update((current) => ({
          ...current,
          titleMatcher: {
            caseSensitive: current.titleMatcher?.caseSensitive ?? false,
            kind,
            value: current.titleMatcher?.value ?? '',
          },
        }));
    },
    [update],
  );
  const handleTitleValue = useCallback(
    (value: string) =>
      update((current) => ({
        ...current,
        titleMatcher:
          value.trim().length === 0
            ? null
            : {
                caseSensitive: current.titleMatcher?.caseSensitive ?? false,
                kind: current.titleMatcher?.kind ?? 'contains',
                value,
              },
      })),
    [update],
  );
  return (
    <div className="rounded-panel grid gap-4 border p-5">
      <div className="flex items-center justify-between gap-3">
        <label className="text-control flex items-center gap-2 font-medium">
          <Checkbox
            aria-label={`Enable ${rule.label || 'protected window rule'}`}
            checked={rule.enabled}
            onCheckedChange={handleEnabled}
          />
          Enabled
        </label>
        <Button
          aria-label={`Delete ${rule.label || 'protected window rule'}`}
          onClick={handleDelete}
          size="icon"
          type="button"
          variant="ghost"
        >
          <Trash2 aria-hidden="true" className="size-4" />
        </Button>
      </div>
      <div className="grid gap-4 md:grid-cols-2">
        <TextSetting
          label="Label"
          maxLength={maxTextChars}
          onChange={handleLabel}
          value={rule.label}
        />
        <SelectSetting
          label="Action"
          onChange={handleAction}
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
          onChange={handleIdentity}
          value={rule.appIdentity ?? ''}
        />
        <TextSetting
          label="Process"
          maxLength={maxTextChars}
          onChange={handleProcess}
          value={rule.processName ?? ''}
        />
        <SelectSetting
          label="Title matcher"
          onChange={handleTitleKind}
          options={[
            ['contains', 'Contains'],
            ['regex', 'Regex'],
          ]}
          value={rule.titleMatcher?.kind ?? 'contains'}
        />
        <TextSetting
          label="Title value"
          maxLength={maxTextChars}
          onChange={handleTitleValue}
          value={rule.titleMatcher?.value ?? ''}
        />
      </div>
    </div>
  );
}
function emptyToNull(value: string): string | null {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}
