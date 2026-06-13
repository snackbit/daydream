import { invoke } from '@tauri-apps/api/core';
import type {
  AppSettingsState,
  AppSettingsStorageCleanupPreview,
  InputActivityRepairResult,
  InputActivityStatus,
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

export async function getInputActivityStatus(): Promise<InputActivityStatus> {
  return invoke<InputActivityStatus>('getInputActivityStatus');
}

export async function repairInputActivityPermissions(): Promise<InputActivityRepairResult> {
  return invoke<InputActivityRepairResult>('repairInputActivityPermissions');
}
