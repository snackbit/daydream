import { useEffect, useState } from 'react';
import type { RecorderScene } from '@daydream/shared';
import { errorMessage } from './recorder-error-utils';
import { useRecorderWorkspaceStore } from './recorder-workspace-store';
import { getCaptureSetup, saveCaptureSetup } from './source-capture-client';

export function useSourceCaptureSetup() {
  const [defaults, setDefaults] = useState<{
    fps: number;
    videoBitrateKbps: number;
  } | null>(null);
  const [savedDefaults, setSavedDefaults] = useState<{
    fps: number;
    videoBitrateKbps: number;
  } | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [reloadVersion, setReloadVersion] = useState(0);
  useEffect(() => {
    let active = true;
    void getCaptureSetup()
      .then((setup) => {
        if (active) {
          setError(null);
          setDefaults({
            fps: setup.fps,
            videoBitrateKbps: setup.videoBitrateKbps,
          });
          setSavedDefaults({
            fps: setup.fps,
            videoBitrateKbps: setup.videoBitrateKbps,
          });
        }
      })
      .catch((error) => {
        if (active) setError(errorMessage(error));
      });
    return () => {
      active = false;
    };
  }, [reloadVersion]);
  async function save(scene: RecorderScene) {
    if (!defaults || busy) return;
    const before = useRecorderWorkspaceStore.getState();
    setBusy(true);
    setError(null);
    try {
      const setup = await saveCaptureSetup({ ...defaults, scene });
      const after = useRecorderWorkspaceStore.getState();
      if (
        after.scene === before.scene &&
        after.desktopAudioSourceId === before.desktopAudioSourceId &&
        after.microphoneAudioSourceId === before.microphoneAudioSourceId
      ) {
        useRecorderWorkspaceStore.setState({
          scene: setup.scene,
          isDirty: false,
          saveStatus: 'ready',
        });
      }
      setSavedDefaults({
        fps: setup.fps,
        videoBitrateKbps: setup.videoBitrateKbps,
      });
    } catch (error) {
      setError(errorMessage(error));
    } finally {
      setBusy(false);
    }
  }
  return {
    reload: () => setReloadVersion((value) => value + 1),
    defaults,
    setDefaults,
    busy,
    error,
    save,
    dirty:
      defaults !== null &&
      savedDefaults !== null &&
      (defaults.fps !== savedDefaults.fps ||
        defaults.videoBitrateKbps !== savedDefaults.videoBitrateKbps),
  };
}
