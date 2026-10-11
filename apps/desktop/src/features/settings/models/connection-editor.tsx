import type {
  AiModelConnection,
  ChatGptConnectionState,
} from '@daydream/shared';
import { SharedDisclosure } from '@/components/shared-disclosure';
import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { SubscriptionControls } from './subscription-controls';

export function ConnectionEditor({
  connection,
  keyDraft,
  onKeyChange,
  onUpdate,
  onConnected,
  onRegistered,
  onAddModel,
  fixedType = false,
}: {
  connection: AiModelConnection;
  keyDraft: string;
  onKeyChange: (key: string) => void;
  onUpdate: (changes: Partial<AiModelConnection>) => void;
  onConnected: (state: ChatGptConnectionState) => void;
  onRegistered: (id: string) => void;
  onAddModel: (slug: string, name: string) => void;
  fixedType?: boolean;
}) {
  function provider(value: string) {
    if (value !== connection.provider) onKeyChange('');
    if (value === 'openai' || value === 'openai-compatible')
      onUpdate({
        provider: value,
        baseUrl: value === 'openai' ? 'https://api.openai.com/v1' : '',
        authMode: 'api-key',
        apiKeyRef: null,
        subscriptionId: null,
        protocol: value === 'openai' ? 'responses' : 'chat-completions',
        tokenLimit: 'modern',
      });
  }
  function auth(value: string) {
    if (value !== connection.authMode) onKeyChange('');
    if (
      value === 'api-key' ||
      value === 'none' ||
      (value === 'chatgpt' && connection.provider === 'openai')
    )
      onUpdate({ authMode: value, apiKeyRef: null, subscriptionId: null });
  }
  return (
    <div className="grid gap-4">
      <div className={fixedType ? 'grid gap-4' : 'grid gap-4 sm:grid-cols-2'}>
        <Field>
          <FieldLabel htmlFor={`connection-name-${connection.id}`}>
            Connection name
          </FieldLabel>
          <Input
            id={`connection-name-${connection.id}`}
            value={connection.name}
            onChange={(event) => onUpdate({ name: event.target.value })}
          />
        </Field>
        {!fixedType ? (
          <Field>
            <FieldLabel htmlFor={`provider-${connection.id}`}>
              Provider
            </FieldLabel>
            <Select value={connection.provider} onValueChange={provider}>
              <SelectTrigger id={`provider-${connection.id}`}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="openai">OpenAI</SelectItem>
                <SelectItem value="openai-compatible">
                  OpenAI compatible
                </SelectItem>
              </SelectContent>
            </Select>
          </Field>
        ) : null}
        {!fixedType || connection.provider === 'openai-compatible' ? (
          <Field>
            <FieldLabel htmlFor={`auth-${connection.id}`}>
              Authentication
            </FieldLabel>
            <Select value={connection.authMode} onValueChange={auth}>
              <SelectTrigger id={`auth-${connection.id}`}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="api-key">API key</SelectItem>
                {connection.provider === 'openai' ? (
                  <SelectItem value="chatgpt">ChatGPT subscription</SelectItem>
                ) : (
                  <SelectItem value="none">No authentication</SelectItem>
                )}
              </SelectContent>
            </Select>
          </Field>
        ) : null}
        {connection.provider === 'openai-compatible' ? (
          <Field>
            <FieldLabel htmlFor={`url-${connection.id}`}>
              API base URL
            </FieldLabel>
            <Input
              id={`url-${connection.id}`}
              placeholder="http://localhost:8080/v1"
              value={connection.baseUrl}
              onChange={(event) => onUpdate({ baseUrl: event.target.value })}
            />
          </Field>
        ) : null}
        {connection.authMode === 'api-key' ? (
          <Field>
            <FieldLabel htmlFor={`key-${connection.id}`}>API key</FieldLabel>
            <Input
              id={`key-${connection.id}`}
              type="password"
              autoComplete="off"
              value={keyDraft}
              placeholder={
                connection.apiKeyRef ? 'Saved key present' : 'Paste API key'
              }
              onChange={(event) => onKeyChange(event.target.value)}
            />
            <p className="text-muted-foreground text-xs">
              {keyDraft.trim()
                ? 'New key, not saved.'
                : connection.apiKeyRef
                  ? 'Key saved locally.'
                  : 'Keys are encrypted when settings are saved.'}
            </p>
          </Field>
        ) : null}
      </div>
      {connection.authMode === 'chatgpt' ? (
        <SubscriptionControls
          id={connection.subscriptionId}
          onConnected={onConnected}
          onRegistered={onRegistered}
          onAddModel={onAddModel}
        />
      ) : null}
      {connection.provider === 'openai-compatible' ? (
        <SharedDisclosure label="API compatibility">
          <div className="grid gap-4 sm:grid-cols-2">
            <Field>
              <FieldLabel htmlFor={`protocol-${connection.id}`}>
                Protocol
              </FieldLabel>
              <Select
                value={connection.protocol}
                onValueChange={(value) => {
                  if (value === 'responses' || value === 'chat-completions')
                    onUpdate({ protocol: value });
                }}
              >
                <SelectTrigger id={`protocol-${connection.id}`}>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="chat-completions">
                    Chat Completions
                  </SelectItem>
                  <SelectItem value="responses">Responses</SelectItem>
                </SelectContent>
              </Select>
            </Field>
            <Field>
              <FieldLabel htmlFor={`token-limit-${connection.id}`}>
                Token limit parameter
              </FieldLabel>
              <Select
                value={connection.tokenLimit}
                onValueChange={(value) => {
                  if (value === 'modern' || value === 'legacy')
                    onUpdate({ tokenLimit: value });
                }}
              >
                <SelectTrigger id={`token-limit-${connection.id}`}>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="modern">max_completion_tokens</SelectItem>
                  <SelectItem value="legacy">
                    max_tokens (older servers)
                  </SelectItem>
                </SelectContent>
              </Select>
            </Field>
          </div>
        </SharedDisclosure>
      ) : null}
    </div>
  );
}
