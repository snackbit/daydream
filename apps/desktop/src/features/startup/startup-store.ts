import type { AppStartupStatus } from '@daydream/shared';
import { create } from 'zustand';
import { unknownErrorMessage } from '@/lib/async-store-action';
import {
  getAppStartupStatus,
  listenToAppStartupStatusChanged,
  retryAppStartup,
} from './startup-client';

type StartupSubscriptionStatus = 'idle' | 'listening' | 'error';

interface StartupState {
  error: string | null;
  initialize: () => Promise<void>;
  retry: () => Promise<void>;
  status: AppStartupStatus | null;
  subscriptionStatus: StartupSubscriptionStatus;
}

let startupUnlisten: (() => void) | null = null;

export const useStartupStore = create<StartupState>((set, get) => ({
  error: null,
  async initialize() {
    if (get().subscriptionStatus === 'listening') {
      return;
    }

    try {
      let receivedStartupEvent = false;
      const unlisten = await listenToAppStartupStatusChanged((nextStatus) => {
        receivedStartupEvent = true;
        set({
          error: null,
          status: nextStatus,
        });
      });
      startupUnlisten?.();
      startupUnlisten = unlisten;
      set({ subscriptionStatus: 'listening' });

      try {
        const status = await getAppStartupStatus();
        set((state) => ({
          error: null,
          status: receivedStartupEvent ? state.status : status,
          subscriptionStatus: 'listening',
        }));
      } catch (error) {
        set({
          error: unknownErrorMessage(error),
          subscriptionStatus: 'listening',
        });
      }
    } catch (error) {
      set({
        error: unknownErrorMessage(error),
        subscriptionStatus: 'error',
      });
    }
  },
  async retry() {
    try {
      const status = await retryAppStartup();
      set({
        error: null,
        status,
      });
    } catch (error) {
      set({ error: unknownErrorMessage(error) });
    }
  },
  status: null,
  subscriptionStatus: 'idle',
}));
