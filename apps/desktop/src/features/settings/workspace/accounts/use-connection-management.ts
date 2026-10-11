import { useEffect, useRef, useState } from 'react';
import type {
  AiConfiguredModel,
  AiModelConnection,
  AppSettings,
  ChatGptConnectionState,
} from '@daydream/shared';
import { applySettingsEdit } from '../../autosave/settings-edit';
import { modelActionError, signOutChatGpt } from '../../models/model-client';
import {
  emptyRegistry,
  selectedFeatures,
  updateRegistry,
} from '../../models/model-registry';
import { useSettingsStore } from '../../settings-store';
import {
  remoteConsentChanges,
  type RemoteConsent,
} from '../features/remote-consent';
import { useEditorNavigation } from '../use-editor-navigation';

export interface ConnectionManagementProps {
  initial: AppSettings;
  connectionId: string;
  onClose: () => void;
}
export function useConnectionManagement({
  initial,
  connectionId,
  onClose,
}: ConnectionManagementProps) {
  const [base] = useState(() => useSettingsStore.getState().draft ?? initial);
  const [edited, setEdited] = useState(initial);
  const [keyDraft, setKeyDraft] = useState('');
  const [selected, setSelected] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [discarding, setDiscarding] = useState(false);
  const [consent, setConsent] = useState<RemoteConsent[]>([]);
  const registered = useRef<string | null>(null);
  useEffect(
    () => () => {
      const id = registered.current;
      if (
        id &&
        !useSettingsStore
          .getState()
          .draft?.ai.modelRegistry?.connections.some(
            (connection) => connection.subscriptionId === id,
          )
      )
        void signOutChatGpt(id).catch(() => undefined);
    },
    [],
  );
  const registry = edited.ai.modelRegistry ?? emptyRegistry();
  const connection = registry.connections.find(
    (connection) => connection.id === connectionId,
  );
  const models = registry.models.filter(
    (model) => model.connectionId === connectionId,
  );
  const model = models.find((model) => model.id === selected);
  const used = models.some(
    (model) => selectedFeatures(registry, model.id).length,
  );
  const dirty =
    JSON.stringify(base) !== JSON.stringify(edited) || Boolean(keyDraft.trim());
  const navigation = useEditorNavigation(dirty, saving);
  function updateConnection(changes: Partial<AiModelConnection>) {
    setError(null);
    setEdited((current) =>
      updateRegistry(current, (registry) => ({
        ...registry,
        connections: registry.connections.map((item) =>
          item.id === connectionId ? { ...item, ...changes } : item,
        ),
      })),
    );
  }
  function updateModel(changes: Partial<AiConfiguredModel>) {
    setError(null);
    setEdited((current) =>
      updateRegistry(current, (registry) => ({
        ...registry,
        models: registry.models.map((item) =>
          item.id === selected ? { ...item, ...changes } : item,
        ),
      })),
    );
  }
  function addModel(identifier = '', name = '') {
    const existing = models.find(
      (model) => model.model === identifier && identifier,
    );
    if (existing) {
      setSelected(existing.id);
      return;
    }
    const model: AiConfiguredModel = {
      id: crypto.randomUUID(),
      name,
      model: identifier,
      connectionId,
      capabilities: ['text'],
      embeddingDimensions: null,
      embeddingConvention: 'plain',
    };
    setEdited((current) =>
      updateRegistry(current, (registry) => ({
        ...registry,
        models: [...registry.models, model],
      })),
    );
    setSelected(model.id);
  }
  function addBlankModel() {
    addModel();
  }
  function removeModel() {
    if (model && !selectedFeatures(registry, model.id).length) {
      setEdited((current) =>
        updateRegistry(current, (registry) => ({
          ...registry,
          models: registry.models.filter((item) => item.id !== selected),
        })),
      );
      setSelected(null);
    }
  }
  function connected(state: ChatGptConnectionState) {
    updateConnection({ subscriptionId: state.id });
  }
  function registration(id: string) {
    registered.current = id;
    updateConnection({ subscriptionId: id });
  }
  function dismiss(open: boolean) {
    if (!open && !saving) {
      if (dirty) setDiscarding(true);
      else onClose();
    }
  }
  function cancel() {
    if (!useSettingsStore.getState().isDirty)
      useSettingsStore.getState().revertAutosave();
    navigation.finish(onClose);
  }
  function keepEditing() {
    setDiscarding(false);
    navigation.stay();
  }
  function back() {
    setSelected(null);
  }
  function cancelConsent() {
    setConsent([]);
  }
  async function save(approved = false, removing = false) {
    const current = useSettingsStore.getState().draft;
    if (!current || !connection) return;
    if (
      !removing &&
      (!connection.name.trim() ||
        !/^https?:\/\//.test(connection.baseUrl) ||
        models.some(
          (model) =>
            !model.name.trim() ||
            !model.model.trim() ||
            !model.capabilities.length ||
            (model.capabilities.includes('embedding') &&
              (!model.embeddingDimensions ||
                !Number.isInteger(model.embeddingDimensions) ||
                model.embeddingDimensions < 1 ||
                model.embeddingDimensions > 8192)),
        ))
    ) {
      setError(
        'Enter a connection name, a valid API URL and complete each model. Test embedding models to detect their dimensions.',
      );
      return;
    }
    const edit = removing
      ? updateRegistry(edited, (registry) => ({
          ...registry,
          connections: registry.connections.filter(
            (item) => item.id !== connectionId,
          ),
          models: registry.models.filter(
            (item) => item.connectionId !== connectionId,
          ),
        }))
      : edited;
    const next = applySettingsEdit(base, edit, current);
    const changes = remoteConsentChanges(current, next);
    if (!approved && changes.length) {
      setConsent(changes);
      return;
    }
    setConsent([]);
    setSaving(true);
    setError(null);
    try {
      const previous = current.ai.modelRegistry?.connections.find(
        (item) => item.id === connectionId,
      );
      if (
        previous?.subscriptionId &&
        (removing ||
          previous.authMode !== connection.authMode ||
          previous.provider !== connection.provider)
      )
        await signOutChatGpt(previous.subscriptionId);
      const credentials =
        keyDraft.trim() && connection.authMode === 'api-key' && !removing
          ? { modelKeys: { [connectionId]: keyDraft } }
          : {};
      const saved = await useSettingsStore
        .getState()
        .commitEditor(
          (latest) => applySettingsEdit(current, next, latest),
          credentials,
        );
      if (saved) navigation.finish(onClose);
      else
        setError(
          'Could not save this connection. Check its fields and try again.',
        );
    } catch (error) {
      setError(modelActionError(error));
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
  function removeConnection() {
    if (!used) void save(false, true);
  }
  return {
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
  };
}
