import { invoke } from '@tauri-apps/api/core';
import type { DaydreamDiagnostics } from '@daydream/shared';

export async function getDiagnostics(): Promise<DaydreamDiagnostics> {
  return invoke<DaydreamDiagnostics>('get_diagnostics');
}
