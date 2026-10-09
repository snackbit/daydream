import { useEffect, useState } from 'react';
import { localDayId } from './recall-format';

/** Keep Today correct across local midnight, DST, and returning to the window. */
export function useLocalDayId(): string {
  const [dayId, setDayId] = useState(localDayId);
  useEffect(() => {
    let timer: number | undefined;
    function refresh() {
      window.clearTimeout(timer);
      const now = new Date();
      setDayId(localDayId(now));
      const midnight = new Date(now);
      midnight.setHours(24, 0, 0, 0);
      timer = window.setTimeout(
        refresh,
        midnight.getTime() - now.getTime() + 10,
      );
    }
    function resume() {
      if (document.visibilityState === 'visible') refresh();
    }
    refresh();
    window.addEventListener('focus', refresh);
    document.addEventListener('visibilitychange', resume);
    return () => {
      window.clearTimeout(timer);
      window.removeEventListener('focus', refresh);
      document.removeEventListener('visibilitychange', resume);
    };
  }, []);
  return dayId;
}
