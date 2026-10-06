import type { AiOcrEngine, AiVisionProviderSettings } from '@daydream/shared';

export function missingVisionRequirements({
  vision,
  visionKeyDraft,
}: {
  vision: AiVisionProviderSettings;
  visionKeyDraft: string;
}): string[] {
  const missing: string[] = [];
  if (vision.provider !== 'openai-compatible') {
    missing.push('an OpenAI-compatible provider');
  }
  if (!isValidHttpEndpoint(vision.baseUrl)) {
    missing.push('a valid HTTP(S) base URL');
  }
  if (!vision.model?.trim()) {
    missing.push('a Vision model');
  }
  if (
    vision.authMode !== 'none' &&
    !vision.apiKeyRef?.trim() &&
    !visionKeyDraft.trim()
  ) {
    missing.push('an API key');
  }

  return missing;
}

export function joinRequirements(requirements: string[]): string {
  if (requirements.length < 2) {
    return requirements[0] ?? 'a configured Vision endpoint';
  }

  const finalRequirement = requirements.at(-1);
  const precedingRequirements = requirements.slice(0, -1).join(', ');
  return finalRequirement
    ? `${precedingRequirements} and ${finalRequirement}`
    : precedingRequirements;
}

export function visionRemoteDisclosure({
  observationsEnabled,
  ocrEnabled,
  ocrEngine,
}: {
  observationsEnabled: boolean;
  ocrEnabled: boolean;
  ocrEngine: AiOcrEngine;
}): string {
  const sendingFeatures: string[] = [];
  if (ocrEnabled && ocrEngine === 'vision') {
    sendingFeatures.push('Original-size sampled frames will be sent for OCR');
  }
  if (observationsEnabled) {
    sendingFeatures.push(
      'Retained screen frames and OCR text will be sent for screen observations',
    );
  }
  if (sendingFeatures.length === 0) {
    return 'No frame or OCR data will be sent while OCR and screen observations are off. The endpoint settings and encrypted key remain available for later use.';
  }

  return `${sendingFeatures.join('; ')} when you process a recording.`;
}

function isValidHttpEndpoint(baseUrl: string | null): boolean {
  if (!baseUrl?.trim()) {
    return false;
  }

  try {
    const parsedUrl = new URL(baseUrl.trim());
    return (
      (parsedUrl.protocol === 'http:' || parsedUrl.protocol === 'https:') &&
      Boolean(parsedUrl.hostname)
    );
  } catch {
    return false;
  }
}
