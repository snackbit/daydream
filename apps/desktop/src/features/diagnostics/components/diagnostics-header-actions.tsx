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
    <div className="flex flex-col items-end gap-2">
      <div className="flex gap-2">
        <Button
          className="gap-2"
          disabled={!model.diagnostics}
          onClick={() => void actions.copyDebugLogBundle()}
          variant="outline"
        >
          <Clipboard className="size-3.5" />
          Copy Debug Bundle
        </Button>
        <Button
          className="gap-2"
          disabled={!model.diagnostics}
          onClick={() => void actions.copyRawDiagnosticsJson()}
          variant="outline"
        >
          <Clipboard className="size-3.5" />
          Copy JSON
        </Button>
        <Button className="gap-2" onClick={actions.refresh} variant="outline">
          <RefreshCw className="size-3.5" />
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
