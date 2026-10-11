import { useState } from 'react';
import type { AppSettings } from '@daydream/shared';
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
import { applySettingsEdit } from '../autosave/settings-edit';
import { ProtectedWindowSettingsCard } from '../components/protected-window-settings-card';
import type {
  SettingsScreenTabsModel,
  SettingsScreenTabsActions,
} from '../screen/settings-screen-tabs';
import type { SettingsDetail } from '../settings-location';
import { validateDraftSettings } from '../settings-screen-utils';
import { useSettingsStore } from '../settings-store';
import { featureNames } from './features/feature-overview';
import { ProcessingPreferences } from './features/processing-preferences';
import { RecallPreferences } from './features/recall-preferences';
import {
  remoteConsentChanges,
  type RemoteConsent,
} from './features/remote-consent';
import { ScreenPreferences } from './features/screen-preferences';
import { TranscriptionPreferences } from './features/transcription-preferences';
import { RemoteConsentDialog } from './remote-consent-dialog';
import { useEditorNavigation } from './use-editor-navigation';

export interface FeatureEditorProps {
  detail: SettingsDetail;
  model: SettingsScreenTabsModel;
  actions: SettingsScreenTabsActions;
  onClose: () => void;
  onDiscard: () => void;
  onSave: (settings: AppSettings) => Promise<boolean>;
}
export function FeatureEditor({
  detail,
  model,
  actions,
  onClose,
  onDiscard,
  onSave,
}: FeatureEditorProps) {
  const [base] = useState(model.draft);
  const [edited, setEdited] = useState(base);
  const [saving, setSaving] = useState(false);
  const [discarding, setDiscarding] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [consent, setConsent] = useState<RemoteConsent[]>([]);
  const update = (updater: (current: AppSettings) => AppSettings) => {
    if (saving) return;
    setEdited(updater);
    setError(null);
  };
  const editorActions = { ...actions, updateDraft: update };
  const editorModel = { ...model, draft: edited };
  const dirty =
    JSON.stringify(base) !== JSON.stringify(edited) ||
    Boolean(
      model.transcriptionKeyDraft ||
      model.visionKeyDraft ||
      model.recallEmbeddingKeyDraft ||
      model.recallGenerationKeyDraft,
    );
  const navigation = useEditorNavigation(dirty, saving);
  function dismiss(open: boolean) {
    if (!open && !saving) {
      if (dirty) setDiscarding(true);
      else onClose();
    }
  }
  function discard() {
    if (saving) return;
    onDiscard();
    navigation.finish(onClose);
  }
  function keepEditing() {
    setDiscarding(false);
    navigation.stay();
  }
  function cancelConsent() {
    setConsent([]);
  }
  async function save(approved = false) {
    if (saving) return;
    const current = useSettingsStore.getState().draft;
    if (!current) return;
    const next = applySettingsEdit(base, edited, current);
    const validation = validateDraftSettings(next, model.limits);
    if (validation) {
      setError(validation);
      return;
    }
    const changes = remoteConsentChanges(current, next);
    if (!approved && changes.length) {
      setConsent(changes);
      return;
    }
    setConsent([]);
    setSaving(true);
    try {
      if (await onSave(next)) navigation.finish(onClose);
      else
        setError(
          'Could not save changes. Review the connection and try again.',
        );
    } finally {
      setSaving(false);
    }
  }
  function done() {
    void save();
  }
  function confirmConsent() {
    void save(true);
  }
  return (
    <>
      <Sheet open onOpenChange={dismiss}>
        <SheetContent
          className="gap-0 data-[side=right]:w-full data-[side=right]:sm:max-w-[560px]"
          showCloseButton={!saving}
        >
          <SheetHeader className="border-b px-6 py-5">
            <SheetTitle className="text-xl">{featureNames[detail]}</SheetTitle>
            <SheetDescription>
              {detail === 'protection'
                ? 'Pause recording when a protected window appears.'
                : 'Changes apply when you choose Done.'}
            </SheetDescription>
          </SheetHeader>
          <fieldset
            aria-label="Feature settings"
            disabled={saving}
            className="min-h-0 min-w-0 flex-1 overflow-y-auto px-6 py-6"
          >
            {detail === 'summary' || detail === 'search' ? (
              <RecallPreferences
                kind={detail}
                draft={edited}
                models={model.aiModels}
                onDownloadModel={actions.downloadAiModel}
                onUpdate={update}
              />
            ) : detail === 'ocr' || detail === 'observations' ? (
              <ScreenPreferences
                ocrStatus={model.aiProviderStatus?.ocr}
                kind={detail}
                draft={edited}
                onUpdate={update}
              />
            ) : detail === 'transcription' ? (
              <TranscriptionPreferences
                model={editorModel}
                actions={editorActions}
              />
            ) : detail === 'processing' ? (
              <ProcessingPreferences
                draft={edited}
                limits={model.limits}
                onUpdate={update}
              />
            ) : (
              <ProtectedWindowSettingsCard
                draft={edited}
                limits={model.limits}
                onUpdateDraft={update}
              />
            )}
            {error ? (
              <p role="alert" className="text-destructive mt-5 text-sm">
                {error}
              </p>
            ) : null}
          </fieldset>
          <SheetFooter className="flex-row justify-end border-t px-6 py-4">
            <Button variant="outline" disabled={saving} onClick={discard}>
              Cancel
            </Button>
            <Button disabled={saving} onClick={done}>
              {saving ? 'Saving…' : 'Done'}
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
              Your changes in this editor have not been saved.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <Button variant="outline" onClick={keepEditing}>
              Keep editing
            </Button>
            <Button disabled={saving} onClick={discard}>
              Discard changes
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
