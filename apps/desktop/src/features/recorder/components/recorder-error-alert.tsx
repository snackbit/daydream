import { AlertCircle } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';

export interface RecorderErrorAlertProps {
  message: string;
}

export function RecorderErrorAlert({ message }: RecorderErrorAlertProps) {
  return (
    <div className="p-4">
      <Alert variant="destructive">
        <AlertCircle />
        <AlertTitle>Recorder error</AlertTitle>
        <AlertDescription>{message}</AlertDescription>
      </Alert>
    </div>
  );
}
