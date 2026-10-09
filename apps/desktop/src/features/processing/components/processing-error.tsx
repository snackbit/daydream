import { Link } from '@tanstack/react-router';
import { SharedDisclosure } from '@/components/shared-disclosure';
import { Button } from '@/components/ui/button';
import { useDebugEnabled } from '@/features/settings/use-debug-enabled';
import { describeProcessingError } from '../processing-errors';

export interface ProcessingErrorProps {
  errorCode: string;
  technicalDetails?: boolean;
}
export function ProcessingError({
  errorCode,
  technicalDetails = true,
}: ProcessingErrorProps) {
  const description = describeProcessingError(errorCode);
  const debugEnabled = useDebugEnabled();
  return (
    <div className="space-y-1 text-xs">
      <p className="text-destructive" role="alert">
        {description.summary}
      </p>
      {description.settingsTab ? (
        <Button asChild className="h-auto p-0 text-xs" variant="link">
          <Link
            to="/settings"
            search={{
              tab: description.settingsTab,
              focus: description.settingsSection,
            }}
          >
            {description.settingsLabel}
          </Link>
        </Button>
      ) : null}
      {technicalDetails && debugEnabled ? (
        <SharedDisclosure label="Technical details">
          <code className="break-all">{errorCode}</code>
        </SharedDisclosure>
      ) : null}
    </div>
  );
}
