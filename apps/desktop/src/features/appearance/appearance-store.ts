import { create } from 'zustand';

export type AppearanceMode = 'dark' | 'light';
export type AppearancePersistence = 'saved' | 'session-only';

interface AppearanceState {
  mode: AppearanceMode;
  persistence: AppearancePersistence;
  setMode: (mode: AppearanceMode) => void;
}

const appearanceStorageKey = 'daydream.appearance';

/** Destructive maintenance acknowledges this only after browser persistence clears. */
export function resetAppearancePreference(): void {
  window.localStorage.removeItem(appearanceStorageKey);
  document.cookie = `${appearanceStorageKey}=; Max-Age=0; Path=/; SameSite=Lax`;
  applyAppearance('light');
  useAppearanceStore.setState({ mode: 'light', persistence: 'saved' });
}

interface LoadedAppearancePreference {
  mode: AppearanceMode;
  persistence: AppearancePersistence;
}

export function parseAppearancePreference(
  value: string | null,
): AppearanceMode {
  return value === 'dark' ? 'dark' : 'light';
}

function loadAppearancePreference(): LoadedAppearancePreference {
  try {
    return {
      mode: parseAppearancePreference(
        window.localStorage.getItem(appearanceStorageKey),
      ),
      persistence: 'saved',
    };
  } catch {
    return { mode: 'light', persistence: 'session-only' };
  }
}

function saveAppearancePreference(mode: AppearanceMode): AppearancePersistence {
  try {
    window.localStorage.setItem(appearanceStorageKey, mode);
    return 'saved';
  } catch {
    return 'session-only';
  }
}

function applyAppearance(mode: AppearanceMode): void {
  if (typeof document !== 'undefined') {
    document.documentElement.dataset.theme = mode;
  }
}

const initialPreference = loadAppearancePreference();
applyAppearance(initialPreference.mode);

export const useAppearanceStore = create<AppearanceState>((set, get) => ({
  mode: initialPreference.mode,
  persistence: initialPreference.persistence,
  setMode(mode) {
    if (mode === get().mode) {
      return;
    }

    applyAppearance(mode);
    set({
      mode,
      persistence: saveAppearancePreference(mode),
    });
  },
}));
