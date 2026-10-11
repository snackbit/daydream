import { useState } from 'react';
import type {
  AppSettings,
  AiConfiguredModel,
  AiModelConnection,
  ChatGptConnectionState,
} from '@daydream/shared';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
} from '@/components/ui/card';
import type { UpdateSettingsDraft } from '../settings-draft-updaters';
import { ConfiguredModelRow } from './configured-model-row';
import { ConnectionEditor } from './connection-editor';
import { modelActionError, signOutChatGpt } from './model-client';
import {
  emptyRegistry,
  selectedFeatures,
  updateRegistry,
} from './model-registry';

export function ModelsSection({
  draft,
  keyDrafts,
  onKeyChange,
  onUpdateDraft,
}: {
  draft: AppSettings;
  keyDrafts: Readonly<Record<string, string>>;
  onKeyChange: (id: string, key: string) => void;
  onUpdateDraft: UpdateSettingsDraft;
}) {
  const registry = draft.ai.modelRegistry ?? emptyRegistry();
  const [accountAction, setAccountAction] = useState<string | null>(null);
  const [accountError, setAccountError] = useState<string | null>(null);
  async function disconnect(id: string) {
    const connection = registry.connections.find(
      (connection) => connection.id === id,
    );
    if (!connection?.subscriptionId) return true;
    setAccountAction(id);
    setAccountError(null);
    try {
      const revoked = await signOutChatGpt(connection.subscriptionId);
      if (!revoked)
        setAccountError(
          'Signed out locally. Review connected apps in ChatGPT to confirm remote revocation.',
        );
      return true;
    } catch (error) {
      setAccountError(modelActionError(error));
      return false;
    } finally {
      setAccountAction(null);
    }
  }
  async function updateConnection(
    id: string,
    changes: Partial<AiModelConnection>,
  ) {
    const connection = registry.connections.find(
      (connection) => connection.id === id,
    );
    if (
      connection?.authMode === 'chatgpt' &&
      ((changes.authMode && changes.authMode !== 'chatgpt') ||
        (changes.provider && changes.provider !== connection.provider))
    ) {
      if (!(await disconnect(id))) return;
    }
    onUpdateDraft((current) =>
      updateRegistry(current, (registry) => ({
        ...registry,
        connections: registry.connections.map((connection) =>
          connection.id === id ? { ...connection, ...changes } : connection,
        ),
      })),
    );
  }
  function addConnection() {
    const connection: AiModelConnection = {
      id: crypto.randomUUID(),
      name: 'New connection',
      provider: 'openai',
      authMode: 'api-key',
      baseUrl: 'https://api.openai.com/v1',
      apiKeyRef: null,
      subscriptionId: null,
      protocol: 'responses',
      tokenLimit: 'modern',
    };
    onUpdateDraft((current) =>
      updateRegistry(current, (registry) => ({
        ...registry,
        connections: [...registry.connections, connection],
      })),
    );
  }
  function addModel(connectionId: string, identifier = '', name = 'New model') {
    const model: AiConfiguredModel = {
      id: crypto.randomUUID(),
      name,
      model: identifier,
      connectionId,
      capabilities: ['text'],
      embeddingDimensions: null,
      embeddingConvention: 'plain',
    };
    onUpdateDraft((current) =>
      updateRegistry(current, (registry) => ({
        ...registry,
        models: [...registry.models, model],
      })),
    );
  }
  function updateModel(id: string, changes: Partial<AiConfiguredModel>) {
    onUpdateDraft((current) =>
      updateRegistry(current, (registry) => ({
        ...registry,
        models: registry.models.map((model) =>
          model.id === id ? { ...model, ...changes } : model,
        ),
      })),
    );
  }
  function removeModel(id: string) {
    onUpdateDraft((current) =>
      updateRegistry(current, (registry) => ({
        ...registry,
        models: registry.models.filter((model) => model.id !== id),
      })),
    );
  }
  async function removeConnection(id: string) {
    if (!(await disconnect(id))) return;
    onUpdateDraft((current) =>
      updateRegistry(current, (registry) => ({
        ...registry,
        connections: registry.connections.filter(
          (connection) => connection.id !== id,
        ),
        models: registry.models.filter((model) => model.connectionId !== id),
      })),
    );
    onKeyChange(id, '');
  }
  function connected(connectionId: string, state: ChatGptConnectionState) {
    void updateConnection(connectionId, { subscriptionId: state.id });
  }
  return (
    <Card id="ai-models" tabIndex={-1}>
      <CardHeader>
        <CardTitle>Models</CardTitle>
        <CardDescription>
          Configure connections once and add the models you want to use. Each
          feature chooses a model below. Signing in or testing does not enable
          remote processing.
        </CardDescription>
      </CardHeader>
      <CardContent className="grid gap-6">
        {accountError ? (
          <p role="alert" className="text-destructive text-sm">
            {accountError}
          </p>
        ) : null}
        {registry.connections.map((connection) => {
          const models = registry.models.filter(
            (model) => model.connectionId === connection.id,
          );
          const used = models.some(
            (model) => selectedFeatures(registry, model.id).length,
          );
          return (
            <fieldset
              disabled={accountAction !== null}
              key={connection.id}
              className="bg-muted rounded-panel grid gap-5 p-4 sm:p-6"
              aria-label={`Connection ${connection.name}`}
            >
              <ConnectionEditor
                connection={connection}
                keyDraft={keyDrafts[connection.id] ?? ''}
                onKeyChange={(key) => onKeyChange(connection.id, key)}
                onUpdate={(changes) => updateConnection(connection.id, changes)}
                onConnected={(state) => connected(connection.id, state)}
                onRegistered={(id) =>
                  void updateConnection(connection.id, { subscriptionId: id })
                }
                onAddModel={(slug, name) => addModel(connection.id, slug, name)}
              />
              {models.map((model) => (
                <ConfiguredModelRow
                  key={model.id}
                  registry={registry}
                  connection={connection}
                  model={model}
                  keyDraft={keyDrafts[connection.id] ?? ''}
                  onUpdate={(changes) => updateModel(model.id, changes)}
                  onRemove={() => removeModel(model.id)}
                />
              ))}
              <div className="flex flex-wrap gap-3">
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => addModel(connection.id)}
                >
                  Add model
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  disabled={used}
                  onClick={() => removeConnection(connection.id)}
                >
                  Remove connection
                </Button>
                {used ? (
                  <span className="text-muted-foreground self-center text-xs">
                    Reassign its selected models before removing this
                    connection.
                  </span>
                ) : null}
              </div>
            </fieldset>
          );
        })}
        <Button
          type="button"
          variant="outline"
          className="justify-self-start"
          onClick={addConnection}
        >
          Add connection
        </Button>
      </CardContent>
    </Card>
  );
}
