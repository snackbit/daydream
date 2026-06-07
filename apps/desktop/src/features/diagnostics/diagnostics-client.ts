import { invoke } from '@tauri-apps/api/core';
import type { DaydreamDiagnostics, DebugLogBundle } from '@daydream/shared';

export async function getDiagnostics(): Promise<DaydreamDiagnostics> {
  return invoke<DaydreamDiagnostics>('get_diagnostics');
}

export async function getDebugLogBundle(): Promise<DebugLogBundle> {
  return invoke<DebugLogBundle>('get_debug_log_bundle');
}
