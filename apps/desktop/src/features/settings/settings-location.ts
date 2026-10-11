import type { SearchSchemaInput } from '@tanstack/react-router';

export type SettingsTab = 'general' | 'record' | 'storage' | 'ai';
export type SettingsSection =
  'general' | 'recording' | 'privacy' | 'ai' | 'models' | 'storage';
export type SettingsDetail =
  | 'summary'
  | 'search'
  | 'transcription'
  | 'ocr'
  | 'observations'
  | 'processing'
  | 'protection';
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
  section: SettingsSection;
  detail: SettingsDetail | null;
}
export function validateSettingsLocation(
  value: {
    tab?: unknown;
    focus?: unknown;
    section?: unknown;
    detail?: unknown;
  } & SearchSchemaInput,
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
  const legacyFocus =
    focus === 'transcription' ||
    focus === 'vision' ||
    focus === 'ocr' ||
    focus === 'models' ||
    focus === 'recall' ||
    focus === 'recall-generation' ||
    focus === 'recall-embedding' ||
    focus === 'processing'
      ? focus
      : null;
  const explicitSection =
    typeof value === 'object' && value !== null && 'section' in value
      ? settingsSection(value.section)
      : null;
  const explicitDetail =
    typeof value === 'object' && value !== null && 'detail' in value
      ? settingsDetail(value.detail)
      : null;
  return {
    tab,
    focus: legacyFocus,
    section:
      explicitSection ??
      (legacyFocus === 'models'
        ? 'models'
        : tab === 'record'
          ? 'recording'
          : tab === 'ai'
            ? 'ai'
            : tab),
    detail:
      explicitDetail ??
      (legacyFocus === 'recall-generation'
        ? 'summary'
        : legacyFocus === 'recall-embedding'
          ? 'search'
          : legacyFocus === 'vision'
            ? 'observations'
            : legacyFocus === 'transcription' ||
                legacyFocus === 'ocr' ||
                legacyFocus === 'processing'
              ? legacyFocus
              : null),
  };
}

export function settingsSection(value: unknown): SettingsSection | null {
  return value === 'general' ||
    value === 'recording' ||
    value === 'privacy' ||
    value === 'ai' ||
    value === 'models' ||
    value === 'storage'
    ? value
    : null;
}
export function settingsDetail(value: unknown): SettingsDetail | null {
  return value === 'summary' ||
    value === 'search' ||
    value === 'transcription' ||
    value === 'ocr' ||
    value === 'observations' ||
    value === 'processing' ||
    value === 'protection'
    ? value
    : null;
}
