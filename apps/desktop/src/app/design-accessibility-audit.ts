// Browser-only visual QA helpers. These measure the rendered colors, including
// alpha-composited surfaces; they are not a catalog of expected theme tokens.
import type {} from '@vitest/browser-playwright';
import { cdp, page } from 'vitest/browser';

export async function emulate200PercentDesktopZoom(
  cssWidth: 320 | 390 = 390,
): Promise<void> {
  // Desktop zoom halves CSS viewport dimensions and doubles device pixel ratio.
  // Emulate both without changing the test runner's browser chrome.
  await page.viewport(cssWidth, 450);
  await cdp().send('Emulation.setDeviceMetricsOverride', {
    width: cssWidth,
    height: 450,
    deviceScaleFactor: 2,
    mobile: false,
    screenWidth: cssWidth * 2,
    screenHeight: 900,
  });
}

export async function resetZoomEmulation(): Promise<void> {
  await cdp().send('Emulation.clearDeviceMetricsOverride');
}

export async function settleVisualTransitions(): Promise<void> {
  // Theme changes can start finite color transitions on shared controls.
  // Measure the settled state rather than a halfway blend between appearances.
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
  await Promise.all(
    document
      .getAnimations()
      .filter(
        (animation) =>
          animation.effect?.getComputedTiming().iterations !== Infinity,
      )
      .map((animation) => animation.finished.catch(() => undefined)),
  );
}

interface Pixel {
  r: number;
  g: number;
  b: number;
  alpha: number;
}

function pixel(color: string): Pixel {
  if (!CSS.supports('color', color) || color.includes('var('))
    throw new Error(`Unresolved rendered audit color: ${color}`);
  const canvas = document.createElement('canvas');
  canvas.width = 1;
  canvas.height = 1;
  const context = canvas.getContext('2d');
  if (!context)
    throw new Error('A canvas context is required for the color audit');
  context.fillStyle = color;
  context.fillRect(0, 0, 1, 1);
  const [r, g, b, alpha] = context.getImageData(0, 0, 1, 1).data;
  if (
    r === undefined ||
    g === undefined ||
    b === undefined ||
    alpha === undefined
  )
    throw new Error('Missing canvas color data');
  return { r, g, b, alpha: alpha / 255 };
}

function composite(front: Pixel, back: Pixel): Pixel {
  return {
    r: front.r * front.alpha + back.r * (1 - front.alpha),
    g: front.g * front.alpha + back.g * (1 - front.alpha),
    b: front.b * front.alpha + back.b * (1 - front.alpha),
    alpha: 1,
  };
}

function background(element: Element | null): Pixel {
  const layers: Element[] = [];
  for (let owner = element; owner; owner = owner.parentElement)
    layers.push(owner);
  let result: Pixel = { r: 255, g: 255, b: 255, alpha: 1 };
  for (const owner of layers.reverse())
    result = composite(pixel(getComputedStyle(owner).backgroundColor), result);
  return result;
}

function luminance(color: Pixel): number {
  const linear = (channel: number) => {
    const value = channel / 255;
    return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  };
  return (
    linear(color.r) * 0.2126 +
    linear(color.g) * 0.7152 +
    linear(color.b) * 0.0722
  );
}

function ratio(front: Pixel, back: Pixel): number {
  const first = luminance(front);
  const second = luminance(back);
  return (Math.max(first, second) + 0.05) / (Math.min(first, second) + 0.05);
}

export function textContrast(element: Element, pseudo?: string): number {
  const surface = background(element);
  const color = pixel(getComputedStyle(element, pseudo).color);
  let opacity = 1;
  for (let owner: Element | null = element; owner; owner = owner.parentElement)
    opacity *= Number(getComputedStyle(owner).opacity);
  return ratio(
    composite({ ...color, alpha: color.alpha * opacity }, surface),
    surface,
  );
}

export function controlContrast(
  element: Element,
  property: 'borderColor' | 'backgroundColor',
): number {
  const surface = background(element.parentElement);
  return ratio(
    composite(pixel(getComputedStyle(element)[property]), surface),
    surface,
  );
}

export function focusContrast(element: Element): number {
  const style = getComputedStyle(element);
  if (!element.matches(':focus-visible') || style.boxShadow === 'none')
    return 1;
  const surface = background(element.parentElement);
  return ratio(
    composite(pixel(style.getPropertyValue('--tw-ring-color')), surface),
    surface,
  );
}
