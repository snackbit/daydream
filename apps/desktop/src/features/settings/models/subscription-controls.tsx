import { useEffect, useRef, useState } from 'react';
import type { ChatGptConnectionState } from '@daydream/shared';
import { Button } from '@/components/ui/button';
import {
  cancelModelRequest,
  getChatGptConnection,
  modelActionError,
  prepareChatGptConnection,
  signInChatGpt,
  signOutChatGpt,
} from './model-client';

export function SubscriptionControls({
  id,
  onConnected,
  onRegistered,
  onAddModel,
}: {
  id: string | null;
  onConnected: (state: ChatGptConnectionState) => void;
  onRegistered: (id: string) => void;
  onAddModel: (slug: string, name: string) => void;
}) {
  const [state, setState] = useState<ChatGptConnectionState | null>(null),
    [action, setAction] = useState<'sign-in' | 'sign-out' | 'refresh' | null>(
      null,
    ),
    [error, setError] = useState<string | null>(null);
  const pending = useRef<string | null>(null);
  const readVersion = useRef(0);
  useEffect(() => {
    let alive = true;
    const ownership = readVersion;
    const version = ++ownership.current;
    // Registering a new connection during sign-in must not start a competing read.
    if (id && !pending.current)
      void getChatGptConnection(id)
        .then((state) => {
          if (alive && ownership.current === version) setState(state);
        })
        .catch((error) => {
          if (alive && ownership.current === version)
            setError(modelActionError(error));
        });
    return () => {
      alive = false;
      ownership.current++;
    };
  }, [id]);
  useEffect(
    () => () => {
      const request = pending.current;
      pending.current = null;
      if (request) void cancelModelRequest(request).catch(() => undefined);
    },
    [],
  );
  const busy = action !== null;
  const currentState = state?.id === id ? state : null;
  async function signIn() {
    if (pending.current) return;
    readVersion.current++;
    const request = crypto.randomUUID();
    pending.current = request;
    setAction('sign-in');
    setError(null);
    try {
      const registered = id ?? (await prepareChatGptConnection());
      if (pending.current !== request) return;
      onRegistered(registered);
      const state = await signInChatGpt(registered, request);
      if (pending.current !== request) return;
      setState(state);
      onConnected(state);
    } catch (error) {
      if (pending.current === request) setError(modelActionError(error));
    } finally {
      if (pending.current === request) {
        pending.current = null;
        setAction(null);
      }
    }
  }
  async function signOut() {
    if (!id) return;
    const version = ++readVersion.current;
    setAction('sign-out');
    setError(null);
    try {
      const revoked = await signOutChatGpt(id);
      if (readVersion.current !== version) return;
      setState(null);
      if (!revoked)
        setError(
          'Signed out locally. Remote revocation could not be confirmed; review connected apps in ChatGPT.',
        );
    } catch (error) {
      if (readVersion.current === version) setError(modelActionError(error));
    } finally {
      if (readVersion.current === version) setAction(null);
    }
  }
  async function refresh() {
    if (!id) return;
    const version = ++readVersion.current;
    setAction('refresh');
    setError(null);
    try {
      const state = await getChatGptConnection(id);
      if (readVersion.current === version) setState(state);
    } catch (error) {
      if (readVersion.current === version) setError(modelActionError(error));
    } finally {
      if (readVersion.current === version) setAction(null);
    }
  }
  function cancel() {
    if (pending.current)
      void cancelModelRequest(pending.current).catch(() => undefined);
  }
  return (
    <div className="grid gap-3">
      <p className="text-muted-foreground text-sm">
        Use an eligible ChatGPT subscription. Availability and limits are
        managed in ChatGPT Settings → Usage. Signing in does not enable remote
        processing.
      </p>
      {currentState?.status === 'connected' ? (
        <p role="status" className="text-sm">
          Connected as {currentState.label}
        </p>
      ) : null}
      <div className="flex flex-wrap gap-2">
        <Button
          type="button"
          size="sm"
          variant="outline"
          disabled={busy}
          onClick={() => void signIn()}
        >
          {busy
            ? 'Working…'
            : currentState?.status === 'connected'
              ? 'Reconnect ChatGPT'
              : 'Sign in with ChatGPT'}
        </Button>
        {action === 'sign-in' ? (
          <Button type="button" size="sm" variant="ghost" onClick={cancel}>
            Cancel sign-in
          </Button>
        ) : null}
        {id ? (
          <Button
            type="button"
            size="sm"
            variant="outline"
            disabled={busy}
            onClick={() => void refresh()}
          >
            Refresh models
          </Button>
        ) : null}
        {id ? (
          <Button
            type="button"
            size="sm"
            variant="ghost"
            disabled={busy}
            onClick={() => void signOut()}
          >
            Sign out
          </Button>
        ) : null}
      </div>
      {currentState?.status === 'connected' && currentState.models.length ? (
        <div
          className="flex flex-wrap gap-2"
          aria-label="Available ChatGPT models"
        >
          {currentState.models.map((model) => (
            <Button
              key={model.slug}
              type="button"
              size="sm"
              variant="outline"
              onClick={() => onAddModel(model.slug, model.displayName)}
            >
              Add {model.displayName}
            </Button>
          ))}
        </div>
      ) : null}
      {error ? (
        <p role="alert" className="text-destructive text-sm">
          {error}
        </p>
      ) : null}
    </div>
  );
}
