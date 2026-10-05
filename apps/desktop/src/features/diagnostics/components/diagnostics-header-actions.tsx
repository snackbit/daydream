import { Clipboard, RefreshCw } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type {
  DiagnosticsScreenActions,
  DiagnosticsScreenModel,
} from '../use-diagnostics-screen-controller';

interface DiagnosticsHeaderActionsProps {
  actions: DiagnosticsScreenActions;
  model: DiagnosticsScreenModel;
}

export function DiagnosticsHeaderActions({
  actions,
  model,
}: DiagnosticsHeaderActionsProps) {
  return (
    <div className="grid justify-items-start gap-1">
      <div className="flex flex-wrap gap-1">
        <Button
          className="gap-1.5"
          disabled={!model.diagnostics}
          onClick={() => void actions.copyDebugLogBundle()}
          size="sm"
          variant="outline"
        >
          <Clipboard aria-hidden="true" className="size-3" />
          Copy bundle
        </Button>
        <Button
          className="gap-1.5"
          disabled={!model.diagnostics}
          onClick={() => void actions.copyRawDiagnosticsJson()}
          size="sm"
          variant="outline"
        >
          <Clipboard aria-hidden="true" className="size-3" />
          Copy JSON
        </Button>
        <Button
          className="gap-1.5"
          onClick={actions.refresh}
          size="sm"
          variant="outline"
        >
          <RefreshCw aria-hidden="true" className="size-3" />
          Refresh
        </Button>
      </div>
      <p aria-live="polite" className="text-muted-foreground min-h-5 text-xs">
        {model.copyStatus === 'copied' ? 'Diagnostics JSON copied.' : null}
        {model.copyStatus === 'failed'
          ? 'Unable to copy diagnostics JSON.'
          : null}
        {model.bundleCopyStatus === 'copied' ? 'Debug bundle copied.' : null}
        {model.bundleCopyStatus === 'failed'
          ? 'Unable to copy debug bundle.'
          : null}
      </p>
    </div>
  );
}
