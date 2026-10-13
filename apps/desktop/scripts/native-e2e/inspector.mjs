import { setTimeout as delay } from 'node:timers/promises';

// WebKit's inspector transports page commands inside Target envelopes. This
// driver talks to the actual WebView; it never substitutes Tauri IPC or SQLite.
export async function connectInspector() {
  const port = Number(process.env.DAYDREAM_E2E_INSPECTOR_PORT ?? 39288);
  if (!Number.isInteger(port) || port < 1 || port > 65535)
    throw new Error('DAYDREAM_E2E_INSPECTOR_PORT must be a valid port');
  let socketPath;
  for (let attempt = 0; attempt < 80 && !socketPath; attempt += 1) {
    try {
      const index = await fetch(`http://127.0.0.1:${port}`).then((r) =>
        r.text(),
      );
      socketPath = index.match(/\/socket\/\d+\/\d+\/WebPage/)?.[0];
    } catch {
      // Native startup and the inspector listener settle independently.
    }
    if (!socketPath) await delay(100);
  }
  if (!socketPath) throw new Error('Native WebKit inspector did not start');
  const socket = new WebSocket(`ws://127.0.0.1:${port}${socketPath}`);
  const pending = new Map();
  const events = [];
  let nextId = 1;
  let targetId;
  const target = new Promise((resolve, reject) => {
    const timeout = setTimeout(
      () => reject(new Error('No native WebView target')),
      8000,
    );
    socket.onmessage = ({ data }) => {
      const envelope = JSON.parse(data);
      if (envelope.method === 'Target.targetCreated') {
        targetId = envelope.params.targetInfo.targetId;
        clearTimeout(timeout);
        resolve();
      }
      if (envelope.method !== 'Target.dispatchMessageFromTarget') return;
      const message = JSON.parse(envelope.params.message);
      if (
        message.method?.startsWith('Timeline.') ||
        message.method?.startsWith('ScriptProfiler.')
      )
        events.push(message);
      const request = pending.get(message.id);
      if (!request) return;
      pending.delete(message.id);
      clearTimeout(request.timeout);
      if (message.error) request.reject(new Error(message.error.message));
      else request.resolve(message.result);
    };
    socket.onerror = () => {
      clearTimeout(timeout);
      reject(new Error('Native inspector connection failed'));
    };
  });
  await target;
  async function command(method, params = {}) {
    const id = nextId++;
    const response = new Promise((resolve, reject) => {
      const timeout = setTimeout(
        () => {
          pending.delete(id);
          reject(new Error(`Native ${method} timed out`));
        },
        method.startsWith('ScriptProfiler.') ? 30_000 : 8000,
      );
      pending.set(id, { resolve, reject, timeout });
    });
    socket.send(
      JSON.stringify({
        id: nextId++,
        method: 'Target.sendMessageToTarget',
        params: {
          targetId,
          message: JSON.stringify({
            id,
            method,
            params,
          }),
        },
      }),
    );
    return response;
  }
  async function evaluate(expression) {
    const result = await command('Runtime.evaluate', {
      expression,
      returnByValue: true,
    });
    if (result.wasThrown)
      throw new Error(result.result.description ?? 'WebView evaluation failed');
    return result.result.value;
  }
  async function wait(expression, description) {
    for (let attempt = 0; attempt < 100; attempt += 1) {
      if (await evaluate(expression)) return;
      await delay(100);
    }
    throw new Error(`Timed out: ${description}`);
  }
  async function invoke(command, args = {}) {
    const key = `__nativeE2e${nextId}`;
    await evaluate(
      `window[${JSON.stringify(key)}] = { done: false }; window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, ${JSON.stringify(args)}).then(value => window[${JSON.stringify(key)}] = {done:true,value}, error => window[${JSON.stringify(key)}] = {done:true,error:typeof error === 'object' && error !== null ? {code:error.code,field:error.field} : String(error)}); true`,
    );
    await wait(`window[${JSON.stringify(key)}].done`, `native ${command}`);
    const result = await evaluate(
      `JSON.stringify(window[${JSON.stringify(key)}])`,
    );
    await evaluate(`delete window[${JSON.stringify(key)}]`);
    const decoded = JSON.parse(result);
    if (decoded.error)
      throw new Error(`Native ${command}: ${JSON.stringify(decoded.error)}`);
    return decoded.value;
  }
  return {
    evaluate,
    wait,
    invoke,
    command,
    events,
    close: () => socket.close(),
  };
}
