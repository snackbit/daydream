export type MaintenanceAction =
  'reset-preferences' | 'clear-library' | 'factory-reset';
export interface MaintenancePreview {
  action: MaintenanceAction;
  token: string;
  recordingCount: number;
  libraryRowCount: number;
  modelCount: number;
  fileCount: number;
  sizeBytes: number;
}
export interface PreviewMaintenanceInput {
  action: MaintenanceAction;
}
export interface ExecuteMaintenanceInput {
  action: MaintenanceAction;
  previewToken: string;
  confirmation: string;
}
export interface MaintenanceStatus {
  operationId: string;
  action: MaintenanceAction;
  phase: string;
  error: string | null;
}
export interface AcknowledgeMaintenanceThemeResetInput {
  operationId: string;
}
