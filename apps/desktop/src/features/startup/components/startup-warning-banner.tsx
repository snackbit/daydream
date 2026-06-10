import { AlertTriangle } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { useStartupStore } from '../startup-store';

export function StartupWarningBanner() {
  const warnings = useStartupStore((state) => state.status?.warnings ?? []);

  if (warnings.length === 0) {
    return null;
  }

  return (
    <Alert className="rounded-none border-x-0 border-t-0">
      <AlertTriangle className="size-4" />
      <AlertTitle>Some startup work needs attention</AlertTitle>
      <AlertDescription>
        {warnings.map((warning) => warning.message).join(' ')}
      </AlertDescription>
    </Alert>
  );
}
