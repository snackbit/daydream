import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogDescription,
  AlertDialogFooter,
} from '@/components/ui/alert-dialog';
import { Button } from '@/components/ui/button';
import type { RemoteConsent } from './features/remote-consent';

export function RemoteConsentDialog({
  changes,
  onCancel,
  onConfirm,
}: {
  changes: RemoteConsent[];
  onCancel: () => void;
  onConfirm: () => void;
}) {
  function openChange(open: boolean) {
    if (!open) onCancel();
  }
  return (
    <AlertDialog open={changes.length > 0} onOpenChange={openChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Allow remote processing?</AlertDialogTitle>
          <AlertDialogDescription>
            Review where your data will be sent. Other feature permissions
            remain separate.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <div className="grid gap-5">
          {changes.map((change) => (
            <div key={change.feature} className="grid gap-1.5 text-sm">
              <p className="font-medium">{change.feature}</p>
              <p className="break-words">{change.destination}</p>
              <p className="text-muted-foreground">{change.data}</p>
            </div>
          ))}
        </div>
        <AlertDialogFooter>
          <Button variant="outline" onClick={onCancel}>
            Cancel
          </Button>
          <Button onClick={onConfirm}>Allow and save</Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
