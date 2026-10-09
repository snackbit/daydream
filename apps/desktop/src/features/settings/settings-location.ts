import type { SearchSchemaInput } from '@tanstack/react-router';

export type SettingsTab = 'general' | 'record' | 'storage' | 'ai';
export type SettingsFocus =
  | 'transcription'
  | 'vision'
  | 'ocr'
  | 'models'
  | 'processing'
  | 'recall'
  | 'recall-generation'
  | 'recall-embedding';
export interface SettingsLocation {
  tab: SettingsTab;
  focus: SettingsFocus | null;
}
export function validateSettingsLocation(
  value: { tab?: unknown; focus?: unknown } & SearchSchemaInput,
): SettingsLocation {
  return parseSettingsLocation(value);
}
export function settingsTab(value: unknown): SettingsTab {
  return value === 'record' || value === 'storage' || value === 'ai'
    ? value
    : 'general';
}
export function parseSettingsLocation(value: unknown): SettingsLocation {
  const tab =
    typeof value === 'object' && value !== null && 'tab' in value
      ? settingsTab(value.tab)
      : 'general';
  const focus =
    typeof value === 'object' && value !== null && 'focus' in value
      ? value.focus
      : null;
  return {
    tab,
    focus:
      focus === 'transcription' ||
      focus === 'vision' ||
      focus === 'ocr' ||
      focus === 'models' ||
      focus === 'recall' ||
      focus === 'recall-generation' ||
      focus === 'recall-embedding' ||
      focus === 'processing'
        ? focus
        : null,
  };
}
