import { ChevronRight, Plus } from 'lucide-react';
import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogDescription,
  AlertDialogFooter,
} from '@/components/ui/alert-dialog';
import { Button } from '@/components/ui/button';
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetDescription,
  SheetFooter,
} from '@/components/ui/sheet';
import { ConfiguredModelRow } from '../../models/configured-model-row';
import { ConnectionEditor } from '../../models/connection-editor';
import { RemoteConsentDialog } from '../remote-consent-dialog';
import {
  useConnectionManagement,
  type ConnectionManagementProps,
} from './use-connection-management';

export function ConnectionManagement(props: ConnectionManagementProps) {
  const { initial, connectionId } = props;
  const {
    registry,
    connection,
    models,
    model,
    used,
    keyDraft,
    saving,
    error,
    discarding,
    consent,
    navigation,
    setKeyDraft,
    setSelected,
    setDiscarding,
    updateConnection,
    updateModel,
    addModel,
    addBlankModel,
    removeModel,
    connected,
    registration,
    dismiss,
    cancel,
    keepEditing,
    back,
    cancelConsent,
    done,
    confirmConsent,
    removeConnection,
  } = useConnectionManagement(props);
  if (!connection) return null;
  return (
    <>
      <Sheet open onOpenChange={dismiss}>
        <SheetContent
          className="gap-0 data-[side=right]:w-full data-[side=right]:sm:max-w-[560px]"
          showCloseButton={!saving}
        >
          <SheetHeader className="border-b px-6 py-5">
            <SheetTitle className="text-xl">
              {model ? model.name || 'Add model' : connection.name}
            </SheetTitle>
            <SheetDescription>
              {model
                ? 'Choose its capabilities and test with synthetic data.'
                : 'Account setup does not enable remote processing.'}
            </SheetDescription>
          </SheetHeader>
          <fieldset
            disabled={saving}
            className="min-h-0 flex-1 overflow-y-auto px-6 py-6"
          >
            {model ? (
              <div className="grid gap-5">
                <Button
                  variant="ghost"
                  size="sm"
                  className="justify-self-start"
                  onClick={back}
                >
                  Back to connection
                </Button>
                <ConfiguredModelRow
                  registry={registry}
                  connection={connection}
                  model={model}
                  keyDraft={keyDraft}
                  onUpdate={updateModel}
                  onRemove={removeModel}
                />
              </div>
            ) : (
              <div className="grid gap-8">
                <ConnectionEditor
                  fixedType
                  connection={connection}
                  keyDraft={keyDraft}
                  onKeyChange={setKeyDraft}
                  onUpdate={updateConnection}
                  onConnected={connected}
                  onRegistered={registration}
                  onAddModel={addModel}
                />
                <div>
                  <div className="mb-3 flex items-center justify-between">
                    <h3 className="text-sm font-medium">Models</h3>
                    <Button variant="ghost" size="sm" onClick={addBlankModel}>
                      <Plus aria-hidden="true" className="size-3.5" />
                      Add model
                    </Button>
                  </div>
                  {models.length ? (
                    models.map((model) => (
                      <button
                        type="button"
                        key={model.id}
                        onClick={() => setSelected(model.id)}
                        aria-label={`Edit ${model.name || 'new model'}`}
                        className="hover:bg-muted/40 flex w-full items-center gap-3 border-t py-4 text-left"
                      >
                        <div className="min-w-0 flex-1">
                          <p className="truncate text-sm font-medium">
                            {model.name || 'New model'}
                          </p>
                          <p className="text-muted-foreground mt-1 truncate text-xs">
                            {model.model || 'Set a model identifier'} ·{' '}
                            {model.capabilities.join(', ')}
                          </p>
                        </div>
                        <ChevronRight
                          aria-hidden="true"
                          className="text-muted-foreground size-4"
                        />
                      </button>
                    ))
                  ) : (
                    <p className="text-muted-foreground py-3 text-sm">
                      Add a model to use this connection in AI features.
                    </p>
                  )}
                </div>
                {initial.ai.modelRegistry?.connections.some(
                  (item) => item.id === connectionId,
                ) ? (
                  <div className="border-t pt-4">
                    <Button
                      variant="ghost"
                      size="sm"
                      disabled={used}
                      onClick={removeConnection}
                    >
                      Remove connection
                    </Button>
                    {used ? (
                      <p className="text-muted-foreground mt-2 text-xs">
                        Choose another model in AI features before removing this
                        connection.
                      </p>
                    ) : null}
                  </div>
                ) : null}
              </div>
            )}
            {error ? (
              <p role="alert" className="text-destructive mt-5 text-sm">
                {error}
              </p>
            ) : null}
          </fieldset>
          <SheetFooter className="flex-row justify-end border-t px-6 py-4">
            <Button variant="outline" disabled={saving} onClick={cancel}>
              Cancel
            </Button>
            <Button disabled={saving} onClick={model ? back : done}>
              {saving ? 'Saving…' : model ? 'Done with model' : 'Done'}
            </Button>
          </SheetFooter>
        </SheetContent>
      </Sheet>
      <RemoteConsentDialog
        changes={consent}
        onCancel={cancelConsent}
        onConfirm={confirmConsent}
      />
      <AlertDialog
        open={discarding || navigation.blocked}
        onOpenChange={setDiscarding}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Discard changes?</AlertDialogTitle>
            <AlertDialogDescription>
              Changes in this connection have not been saved.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <Button variant="outline" onClick={keepEditing}>
              Keep editing
            </Button>
            <Button onClick={cancel}>Discard changes</Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
