export type ProtectableWindowProviderStatus =
  | 'available'
  | 'permission-denied'
  | 'provider-unavailable'
  | 'session-unsupported'
  | 'unsupported-platform';

export interface ProtectableWindow {
  /** Opaque local icon handle; absent on older native builds. */
  iconKey?: string | null;
  appIdentity: string | null;
  id: string;
  platform: string;
  processId: number | null;
  processName: string | null;
  title: string | null;
}

export interface GetWindowApplicationIconInput {
  iconKey: string;
}
export interface WindowApplicationIcon {
  iconKey: string;
  pngBase64: string | null;
}

export interface ProtectableWindowList {
  provider: string | null;
  status: ProtectableWindowProviderStatus;
  windows: ProtectableWindow[];
}

export interface WindowProtectionStatus {
  message: string;
  platform: string;
  provider: string | null;
  status: ProtectableWindowProviderStatus;
}
