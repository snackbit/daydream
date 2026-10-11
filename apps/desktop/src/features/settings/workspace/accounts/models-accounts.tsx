import { useEffect, useState } from 'react';
import type {
  AiModelConnection,
  AppSettings,
  ChatGptConnectionState,
} from '@daydream/shared';
import {
  Plus,
  ChevronRight,
  KeyRound,
  Download,
  UserRound,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from '@/components/ui/dialog';
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetDescription,
} from '@/components/ui/sheet';
import { ModelTransferRow } from '@/features/ai/models/model-transfer-row';
import { getChatGptConnection } from '../../models/model-client';
import { updateRegistry } from '../../models/model-registry';
import type {
  SettingsScreenTabsModel,
  SettingsScreenTabsActions,
} from '../../screen/settings-screen-tabs';
import { ConnectionManagement } from './connection-management';

function AccountRow({
  connection,
  count,
  onManage,
}: {
  connection: AiModelConnection;
  count: number;
  onManage: (id: string) => void;
}) {
  const [state, setState] = useState<ChatGptConnectionState | null>(null);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    let active = true;
    if (connection.authMode === 'chatgpt' && connection.subscriptionId)
      void getChatGptConnection(connection.subscriptionId)
        .then((value) => {
          if (active) setState(value);
        })
        .catch(() => {
          if (active) setFailed(true);
        });
    return () => {
      active = false;
    };
  }, [connection.authMode, connection.subscriptionId]);
  function manage() {
    onManage(connection.id);
  }
  const status =
    connection.authMode === 'chatgpt'
      ? failed
        ? 'Could not check sign-in'
        : !connection.subscriptionId
          ? 'Sign in needed'
          : state?.status === 'connected'
            ? 'Connected'
            : state?.status === 'reconnect-required'
              ? 'Reconnect needed'
              : state?.status === 'signed-out'
                ? 'Signed out'
                : 'Checking sign-in…'
      : connection.authMode === 'none'
        ? 'No authentication'
        : connection.apiKeyRef
          ? 'Key saved'
          : 'API key needed';
  const Icon = connection.authMode === 'chatgpt' ? UserRound : KeyRound;
  return (
    <button
      type="button"
      aria-label={`Manage ${connection.name}`}
      onClick={manage}
      className="hover:bg-muted/40 focus-visible:outline-ring flex w-full items-center gap-4 border-b py-5 text-left last:border-b-0 focus-visible:rounded-lg focus-visible:outline-2"
    >
      <div className="bg-muted flex size-9 shrink-0 items-center justify-center rounded-xl">
        <Icon aria-hidden="true" className="size-4" />
      </div>
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium">{connection.name}</p>
        <p className="text-muted-foreground mt-1 text-xs">
          {connection.authMode === 'chatgpt'
            ? 'ChatGPT'
            : connection.provider === 'openai'
              ? 'OpenAI API'
              : 'Custom API'}{' '}
          · {count} {count === 1 ? 'model' : 'models'}
        </p>
      </div>
      <span className="text-muted-foreground max-w-40 text-right text-xs">
        {status}
      </span>
      <ChevronRight
        aria-hidden="true"
        className="text-muted-foreground size-4 shrink-0"
      />
    </button>
  );
}
export function ModelsAccounts({
  model,
  actions,
}: {
  model: SettingsScreenTabsModel;
  actions: SettingsScreenTabsActions;
}) {
  const [adding, setAdding] = useState(false);
  const [browsing, setBrowsing] = useState(false);
  const [editor, setEditor] = useState<{
    initial: AppSettings;
    id: string;
  } | null>(null);
  function manage(id: string) {
    setEditor({ initial: model.draft, id });
  }
  function close() {
    setEditor(null);
  }
  function add() {
    setAdding(true);
  }
  function browse() {
    setBrowsing(true);
  }
  function create(kind: 'chatgpt' | 'openai' | 'custom') {
    const connection: AiModelConnection = {
      id: crypto.randomUUID(),
      name:
        kind === 'chatgpt'
          ? 'ChatGPT'
          : kind === 'openai'
            ? 'OpenAI API'
            : 'Custom API',
      provider: kind === 'custom' ? 'openai-compatible' : 'openai',
      authMode: kind === 'chatgpt' ? 'chatgpt' : 'api-key',
      baseUrl: kind === 'custom' ? '' : 'https://api.openai.com/v1',
      apiKeyRef: null,
      subscriptionId: null,
      protocol: kind === 'custom' ? 'chat-completions' : 'responses',
      tokenLimit: 'modern',
    };
    const initial = updateRegistry(model.draft, (registry) => ({
      ...registry,
      connections: [...registry.connections, connection],
    }));
    setAdding(false);
    setEditor({ initial, id: connection.id });
  }
  function chatgpt() {
    create('chatgpt');
  }
  function openai() {
    create('openai');
  }
  function custom() {
    create('custom');
  }
  const registry = model.draft.ai.modelRegistry;
  const selected = new Set([
    model.draft.ai.transcription.localModelId,
    model.draft.ai.recall.embedding.localModelId,
    ...(model.draft.ai.ocr.enabled && model.draft.ai.ocr.engine === 'tesseract'
      ? ['tesseract-runtime', 'tessdata-eng']
      : []),
  ]);
  const installed = model.aiModels.filter(
    (asset) => asset.downloadStatus !== 'missing' || selected.has(asset.id),
  );
  const transfers = {
    onCancelModelDownload: actions.cancelAiModelDownload,
    onDeleteModel: actions.deleteAiModel,
    onDownloadModel: actions.downloadAiModel,
    onTransferAction: actions.transferAiModel,
  };
  return (
    <div>
      <div className="flex items-center justify-between gap-3">
        <h3 className="text-sm font-medium">Accounts & APIs</h3>
        <Button variant="outline" size="sm" onClick={add}>
          <Plus aria-hidden="true" className="size-3.5" />
          Add connection
        </Button>
      </div>
      <p className="text-muted-foreground mt-2 text-sm">
        Connect once, then choose a model for each AI feature.
      </p>
      <div className="mt-3">
        {registry?.connections.length ? (
          registry.connections.map((connection) => (
            <AccountRow
              key={`${connection.id}-${JSON.stringify(connection)}`}
              connection={connection}
              count={
                registry.models.filter(
                  (model) => model.connectionId === connection.id,
                ).length
              }
              onManage={manage}
            />
          ))
        ) : (
          <p className="text-muted-foreground py-7 text-sm">
            No accounts connected. Local models work without an account.
          </p>
        )}
      </div>
      <div className="mt-10 mb-3 flex items-center justify-between gap-3">
        <h3 className="text-sm font-medium">On this device</h3>
        <Button variant="ghost" size="sm" onClick={browse}>
          <Download aria-hidden="true" className="size-3.5" />
          Browse models
        </Button>
      </div>
      <div className="divide-y">
        {installed.map((asset) =>
          asset.id === 'tesseract-runtime' &&
          model.aiProviderStatus?.ocr.runtimeSource === 'system' ? (
            <div key={asset.id} className="py-4">
              <p className="text-sm font-medium">Tesseract</p>
              <p className="text-muted-foreground text-xs">
                System runtime installed
              </p>
            </div>
          ) : (
            <ModelTransferRow
              key={asset.id}
              model={asset}
              actionError={model.modelActionErrors?.[asset.id]}
              {...transfers}
            />
          ),
        )}
      </div>
      <p className="text-muted-foreground mt-4 text-xs">
        Downloads contact the model host. Recordings stay on this device.
      </p>
      <Dialog open={adding} onOpenChange={setAdding}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Add connection</DialogTitle>
            <DialogDescription>Choose how to connect.</DialogDescription>
          </DialogHeader>
          <div className="grid gap-2">
            <Button
              variant="outline"
              className="h-auto justify-start px-4 py-4"
              onClick={chatgpt}
            >
              ChatGPT subscription
            </Button>
            <Button
              variant="outline"
              className="h-auto justify-start px-4 py-4"
              onClick={openai}
            >
              OpenAI API key
            </Button>
            <Button
              variant="outline"
              className="h-auto justify-start px-4 py-4"
              onClick={custom}
            >
              Custom API
            </Button>
          </div>
        </DialogContent>
      </Dialog>
      <Sheet open={browsing} onOpenChange={setBrowsing}>
        <SheetContent className="gap-0 data-[side=right]:w-full data-[side=right]:sm:max-w-[560px]">
          <SheetHeader className="border-b">
            <SheetTitle className="text-xl">Local models</SheetTitle>
            <SheetDescription>
              Install models for transcription, search and text recognition.
            </SheetDescription>
          </SheetHeader>
          <div className="min-h-0 flex-1 divide-y overflow-y-auto px-6">
            {model.aiModels.map((asset) => (
              <ModelTransferRow
                key={asset.id}
                model={asset}
                actionError={model.modelActionErrors?.[asset.id]}
                {...transfers}
              />
            ))}
          </div>
        </SheetContent>
      </Sheet>
      {editor ? (
        <ConnectionManagement
          key={editor.id}
          initial={editor.initial}
          connectionId={editor.id}
          onClose={close}
        />
      ) : null}
    </div>
  );
}
