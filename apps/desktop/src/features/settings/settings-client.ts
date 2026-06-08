import { invoke } from '@tauri-apps/api/core';
import type {
  AppSettingsState,
  AppSettingsStorageCleanupPreview,
  PreviewAppSettingsStorageCleanupInput,
  SaveAppSettingsInput,
} from '@daydream/shared';

export async function getAppSettings(): Promise<AppSettingsState> {
  return invoke<AppSettingsState>('getAppSettings');
}

export async function saveAppSettings(
  input: SaveAppSettingsInput,
): Promise<AppSettingsState> {
  return invoke<AppSettingsState>('saveAppSettings', { input });
}

export async function previewAppSettingsStorageCleanup(
  input: PreviewAppSettingsStorageCleanupInput,
): Promise<AppSettingsStorageCleanupPreview> {
  return invoke<AppSettingsStorageCleanupPreview>(
    'previewAppSettingsStorageCleanup',
    { input },
  );
}

export async function resetAppSettings(): Promise<AppSettingsState> {
  return invoke<AppSettingsState>('resetAppSettings');
}
