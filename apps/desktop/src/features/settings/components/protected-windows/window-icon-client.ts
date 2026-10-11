import { invoke, decodeResponse } from '@/lib/diagnostics/invoke';

const icons = new Map<string, Promise<string | null>>();
const maxEncodedPngBytes = Math.ceil((64 * 1024) / 3) * 4;

export function decodeWindowIcon(
  value: unknown,
  expectedKey: string,
): string | null {
  if (
    typeof value !== 'object' ||
    value === null ||
    !('iconKey' in value) ||
    value.iconKey !== expectedKey ||
    !('pngBase64' in value)
  )
    throw new Error('Invalid local icon response');
  if (value.pngBase64 === null) return null;
  if (
    typeof value.pngBase64 !== 'string' ||
    value.pngBase64.length > maxEncodedPngBytes ||
    !value.pngBase64.startsWith('iVBORw0KGgo') ||
    !/^[A-Za-z0-9+/]*={0,2}$/.test(value.pngBase64)
  )
    throw new Error('Invalid local PNG icon');
  return `data:image/png;base64,${value.pngBase64}`;
}

export function getWindowApplicationIcon(
  iconKey: string,
): Promise<string | null> {
  const existing = icons.get(iconKey);
  if (existing) return existing;
  if (icons.size >= 128) {
    const oldest = icons.keys().next().value;
    if (oldest !== undefined) icons.delete(oldest);
  }
  const request = invoke<unknown>('getWindowApplicationIcon', {
    input: { iconKey },
  })
    .then((value) =>
      decodeResponse('getWindowApplicationIcon', value, (value) =>
        decodeWindowIcon(value, iconKey),
      ),
    )
    .catch((error: unknown) => {
      if (icons.get(iconKey) === request) icons.delete(iconKey);
      throw error;
    });
  icons.set(iconKey, request);
  return request;
}
