import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import type {
  LibraryOverviewActions,
  LibraryOverviewModel,
} from './use-library-overview-controller';
import { RECORDING_TITLE_MAX_CHARS } from './use-library-overview-controller';

interface LibraryOverviewDialogsProps {
  actions: Pick<
    LibraryOverviewActions,
    | 'closeDeleteDialog'
    | 'closeRenameDialog'
    | 'confirmDelete'
    | 'confirmRename'
    | 'setRenameTitle'
  >;
  model: Pick<
    LibraryOverviewModel,
    'deleteTarget' | 'renameSaving' | 'renameTarget' | 'renameTitle'
  >;
}

export function LibraryOverviewDialogs({
  actions,
  model,
}: LibraryOverviewDialogsProps) {
  return (
    <>
      <Dialog
        open={model.deleteTarget !== null}
        onOpenChange={(open) => {
          if (!open) {
            actions.closeDeleteDialog();
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete recording</DialogTitle>
            <DialogDescription>
              This removes{' '}
              {model.deleteTarget?.label ?? 'the selected recordings'}
              and any recording files still present in Daydream storage.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={actions.closeDeleteDialog}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={() => void actions.confirmDelete()}
            >
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={model.renameTarget !== null}
        onOpenChange={(open) => {
          if (!open) {
            actions.closeRenameDialog();
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Rename recording</DialogTitle>
            <DialogDescription>
              Set a display title for{' '}
              {model.renameTarget?.fallbackTitle ?? 'this recording'}. Leave it
              blank to use the recording timestamp.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-2">
            <Label htmlFor="recording-title">Title</Label>
            <Input
              id="recording-title"
              maxLength={RECORDING_TITLE_MAX_CHARS + 1}
              value={model.renameTitle}
              onChange={(event) => actions.setRenameTitle(event.target.value)}
            />
            <div className="text-muted-foreground text-xs">
              {model.renameTitle.trim().length}/{RECORDING_TITLE_MAX_CHARS}
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={actions.closeRenameDialog}>
              Cancel
            </Button>
            <Button
              disabled={
                model.renameSaving ||
                model.renameTitle.trim().length > RECORDING_TITLE_MAX_CHARS
              }
              onClick={() => void actions.confirmRename()}
            >
              Save
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
