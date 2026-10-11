import type {
  AcknowledgeMaintenanceThemeResetInput,
  ExecuteMaintenanceInput,
  MaintenancePreview,
  MaintenanceStatus,
  PreviewMaintenanceInput,
} from '@daydream/shared';
import { invoke } from '@/lib/diagnostics/invoke';

export function previewMaintenance(
  input: PreviewMaintenanceInput,
): Promise<MaintenancePreview> {
  return invoke('previewMaintenance', { input });
}
export function executeMaintenance(
  input: ExecuteMaintenanceInput,
): Promise<MaintenanceStatus> {
  return invoke('executeMaintenance', { input });
}
export function getMaintenanceStatus(): Promise<MaintenanceStatus | null> {
  return invoke('getMaintenanceStatus');
}
export function acknowledgeMaintenanceThemeReset(
  input: AcknowledgeMaintenanceThemeResetInput,
): Promise<MaintenanceStatus> {
  return invoke('acknowledgeMaintenanceThemeReset', { input });
}
