export type AppStartupPhase = 'error' | 'loading' | 'ready';

export type AppStartupStageStatus =
  | 'error'
  | 'loading'
  | 'pending'
  | 'ready'
  | 'warning';

export interface AppStartupIssue {
  code: string;
  message: string;
  stageId: string;
}

export interface AppStartupStage {
  description: string;
  id: string;
  label: string;
  status: AppStartupStageStatus;
}

export interface AppStartupStatus {
  activeStageId: string | null;
  error: AppStartupIssue | null;
  phase: AppStartupPhase;
  stages: AppStartupStage[];
  warnings: AppStartupIssue[];
}
