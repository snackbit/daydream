import { vi } from 'vitest';

export const settings = {
  getAppSettings: vi.fn(),
  saveAppSettings: vi.fn(),
  resetAppSettings: vi.fn(),
  previewAppSettingsStorageCleanup: vi.fn(),
  getInputActivityStatus: vi.fn(),
  repairInputActivityPermissions: vi.fn(),
};
export const recorder = {
  getRecorderScene: vi.fn(),
  getWindowProtectionStatus: vi.fn(),
  listProtectableWindows: vi.fn(),
};
export const ai = {
  listAiModels: vi.fn(),
  getAiProviderStatus: vi.fn(),
  listenToAiModelsChanged: vi.fn(),
  downloadAiModel: vi.fn(),
  cancelAiModelDownload: vi.fn(),
  deleteAiModel: vi.fn(),
  deleteAiProviderSecret: vi.fn(),
};
export const transfer = {
  pauseAiModelDownload: vi.fn(),
  resumeAiModelDownload: vi.fn(),
  removeAiModelPartial: vi.fn(),
};
export const recall = {
  getRecallStatus: vi.fn(),
  listenToRecallStateChanged: vi.fn(),
  testRecallProvider: vi.fn(),
};
export const maintenance = {
  getMaintenanceStatus: vi.fn(),
  previewMaintenance: vi.fn(),
  executeMaintenance: vi.fn(),
  acknowledgeMaintenanceThemeReset: vi.fn(),
};
export const accounts = {
  getChatGptConnection: vi.fn(),
  prepareChatGptConnection: vi.fn(),
  signInChatGpt: vi.fn(),
  signOutChatGpt: vi.fn(),
  cancelModelRequest: vi.fn(),
  testConfiguredModel: vi.fn(),
  modelActionError: (error: unknown) =>
    typeof error === 'string' ? error : 'Connection failed. Try again.',
};
export const blocker: {
  status: 'idle' | 'blocked';
  proceed: ReturnType<typeof vi.fn>;
  reset: ReturnType<typeof vi.fn>;
} = { status: 'idle', proceed: vi.fn(), reset: vi.fn() };
