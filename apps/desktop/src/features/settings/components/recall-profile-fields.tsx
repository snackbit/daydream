import { useEffect, useRef, useState, type ChangeEvent } from 'react';
import type { RecallExternalProfile, RecallSettings } from '@daydream/shared';
import { Button } from '@/components/ui/button';
import { Field, FieldDescription, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { testRecallProvider } from '@/features/recall/recall-client';
import { CheckboxField } from './ai-settings-fields';

export interface RecallProfileFieldsProps {
  kind: 'embedding' | 'generation';
  settings: RecallSettings;
  keyDraft: string;
  onKeyChange: (value: string) => void;
  onChange: (profile: RecallExternalProfile) => void;
}

export function RecallProfileFields({
  kind,
  settings,
  keyDraft,
  onKeyChange,
  onChange,
}: RecallProfileFieldsProps) {
  const profile = settings[kind];
  const prefix = `recall-${kind}`;
  const label = kind === 'embedding' ? 'Embedding' : 'Generation';
  const [pending, setPending] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const [testedInput, setTestedInput] = useState<{
    settings: RecallSettings;
    keyDraft: string;
  } | null>(null);
  const currentTest =
    testedInput?.settings === settings && testedInput.keyDraft === keyDraft;
  const request = useRef(0);
  useEffect(() => {
    request.current += 1;
    return () => {
      request.current += 1;
    };
  }, [settings, keyDraft]);
  function handleEnabled(enabled: boolean) {
    onChange({ ...profile, enabled });
  }
  function handleBaseUrl(event: ChangeEvent<HTMLInputElement>) {
    onChange({ ...profile, baseUrl: event.target.value || null });
  }
  function handleModel(event: ChangeEvent<HTMLInputElement>) {
    onChange({ ...profile, model: event.target.value || null });
  }
  function handleKey(event: ChangeEvent<HTMLInputElement>) {
    onKeyChange(event.target.value);
  }
  function handleAuth(value: string) {
    if (value === 'api-key' || value === 'none')
      onChange({ ...profile, authMode: value });
  }
  async function handleTest() {
    const token = ++request.current;
    setTestedInput({ settings, keyDraft });
    setPending(true);
    setResult(null);
    try {
      const response = await testRecallProvider({
        kind,
        settings,
        keyDraft: keyDraft.trim() || null,
      });
      if (token === request.current)
        setResult(
          response.status === 'succeeded'
            ? `Synthetic test succeeded (${response.elapsedMs} ms).`
            : `Synthetic test failed: ${response.errorCode ?? 'provider unavailable'}.`,
        );
    } catch {
      if (token === request.current)
        setResult(
          'Could not test this provider. Check the endpoint and authentication.',
        );
    } finally {
      if (token === request.current) setPending(false);
    }
  }
  return (
    <div className="grid gap-5">
      <CheckboxField
        checked={profile.enabled}
        label={
          kind === 'embedding'
            ? 'Allow remote embeddings'
            : 'Enable external generation'
        }
        description={
          kind === 'embedding'
            ? 'Eligible OCR and both audio transcripts are sent to this endpoint for semantic indexing and queries are sent for search. This consent is separate from Vision and generation.'
            : 'Eligible OCR and both audio transcripts are sent to this endpoint to generate Day summaries, sessions, and task suggestions. This consent is separate from Vision and embeddings.'
        }
        onCheckedChange={handleEnabled}
      />
      <div className="grid gap-5 sm:grid-cols-2">
        <Field>
          <FieldLabel htmlFor={`${prefix}-url`}>
            {label} endpoint URL
          </FieldLabel>
          <Input
            id={`${prefix}-url`}
            value={profile.baseUrl ?? ''}
            onChange={handleBaseUrl}
            placeholder="https://provider.example/v1"
          />
        </Field>
        <Field>
          <FieldLabel htmlFor={`${prefix}-model`}>
            {label} model name
          </FieldLabel>
          <Input
            id={`${prefix}-model`}
            value={profile.model ?? ''}
            onChange={handleModel}
          />
        </Field>
        <Field>
          <FieldLabel htmlFor={`${prefix}-auth`}>
            {label} authentication
          </FieldLabel>
          <Select value={profile.authMode} onValueChange={handleAuth}>
            <SelectTrigger id={`${prefix}-auth`} className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="api-key">API key</SelectItem>
              <SelectItem value="none">Anonymous</SelectItem>
            </SelectContent>
          </Select>
        </Field>
        {profile.authMode === 'api-key' ? (
          <Field>
            <FieldLabel htmlFor={`${prefix}-key`}>{label} API key</FieldLabel>
            <Input
              id={`${prefix}-key`}
              autoComplete="off"
              type="password"
              value={keyDraft}
              onChange={handleKey}
              placeholder={
                profile.apiKeyRef ? 'Saved key present' : 'Paste API key'
              }
            />
            <FieldDescription>
              {keyDraft.trim()
                ? 'New key, not saved.'
                : profile.apiKeyRef
                  ? 'Key saved locally.'
                  : 'Enter an API key.'}
            </FieldDescription>
          </Field>
        ) : null}
      </div>
      <div className="flex flex-wrap items-center gap-3">
        <Button
          variant="outline"
          disabled={(pending && currentTest) || !profile.enabled}
          onClick={handleTest}
        >
          {pending && currentTest ? 'Testing…' : `Test ${kind} provider`}
        </Button>
        <span className="text-muted-foreground text-xs">
          {profile.enabled
            ? 'Uses synthetic text only. No captured content is sent by this test.'
            : 'Enable this capability to test its provider.'}
        </span>
      </div>
      {result && currentTest ? (
        <p className="text-sm" role="status">
          {result}
        </p>
      ) : null}
    </div>
  );
}
