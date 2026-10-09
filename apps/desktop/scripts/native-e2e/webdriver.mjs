import { setTimeout as delay } from 'node:timers/promises';

// W3C actions reach WebKit's native automation backend; Broadway is display-only
// for text entry. This never assigns DOM values or substitutes application IPC.
export async function connectNativeKeyboard() {
  const endpoint = 'http://127.0.0.1:39290';
  let ready = false;
  for (let attempt = 0; attempt < 80 && !ready; attempt += 1) {
    try {
      ready = (await fetch(`${endpoint}/status`)).ok;
    } catch {
      /* startup */
    }
    if (!ready) await delay(100);
  }
  if (!ready) throw new Error('WebKitWebDriver did not start');
  async function request(path, body, method = 'POST') {
    const response = await fetch(`${endpoint}${path}`, {
      method,
      signal: AbortSignal.timeout(15000),
      headers: { 'Content-Type': 'application/json' },
      ...(body === undefined ? {} : { body: JSON.stringify(body) }),
    });
    const result = await response.json();
    if (!response.ok || result.value?.error)
      throw new Error(
        `WebKitWebDriver: ${result.value?.message ?? response.status}`,
      );
    return result.value;
  }
  const session = await request('/session', {
    capabilities: { alwaysMatch: {} },
  });
  const prefix = `/session/${session.sessionId}`;
  const evaluate = (expression) =>
    request(`${prefix}/execute/sync`, {
      script: 'return eval(arguments[0]);',
      args: [expression],
    });
  const wait = async (expression, description) => {
    for (let attempt = 0; attempt < 100; attempt += 1) {
      if (await evaluate(expression)) return;
      await delay(100);
    }
    throw new Error(`Timed out: ${description}`);
  };
  const keys = async (text) =>
    request(`${prefix}/actions`, {
      actions: [
        {
          type: 'key',
          id: 'native-keyboard',
          actions: [...text].flatMap((value) => [
            { type: 'keyDown', value },
            { type: 'keyUp', value },
          ]),
        },
      ],
    });
  return {
    type: keys,
    enter: () => keys(String.fromCharCode(0xe007)),
    evaluate,
    wait,
    async click(selector) {
      const element = await request(`${prefix}/element`, {
        using: 'css selector',
        value: selector,
      });
      await request(
        `${prefix}/element/${element['element-6066-11e4-a52e-4f735466cecf']}/click`,
        {},
      );
    },
    async invoke(command, args = {}) {
      const result = await request(`${prefix}/execute/async`, {
        script:
          'const done = arguments[2]; window.__TAURI_INTERNALS__.invoke(arguments[0], arguments[1]).then(value => done({value}), error => done({error:String(error)}));',
        args: [command, args],
      });
      if (result.error) throw new Error(result.error);
      return result.value;
    },
    close: () => request(prefix, undefined, 'DELETE'),
  };
}
