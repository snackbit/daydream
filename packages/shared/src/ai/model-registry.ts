export type ModelCapability = 'text' | 'vision' | 'embedding';
export type ModelProtocol = 'responses' | 'chat-completions';
export interface AiModelRoute {
  protocol: ModelProtocol;
  tokenLimit: 'modern' | 'legacy';
  subscriptionId: string | null;
  subscription: boolean;
}
export interface AiModelConnection {
  id: string;
  name: string;
  provider: 'openai' | 'openai-compatible';
  authMode: 'api-key' | 'none' | 'chatgpt';
  baseUrl: string;
  apiKeyRef: string | null;
  subscriptionId: string | null;
  protocol: ModelProtocol;
  tokenLimit: 'modern' | 'legacy';
}
export interface AiConfiguredModel {
  id: string;
  name: string;
  model: string;
  connectionId: string;
  capabilities: ModelCapability[];
  embeddingDimensions: number | null;
  embeddingConvention: 'plain' | 'e5' | 'bge';
}
export interface AiModelRegistry {
  version: 1;
  connections: AiModelConnection[];
  models: AiConfiguredModel[];
  bindings: {
    embedding: string | null;
    summary: string | null;
    vision: string | null;
  };
}
export interface AiModelTestInput {
  connection: AiModelConnection;
  model: AiConfiguredModel;
  keyDraft: string | null;
  requestId: string;
}
export interface AiModelTestResult {
  fingerprint: string;
  checks: Array<{
    capability: ModelCapability;
    status: 'succeeded' | 'failed';
    message: string;
    errorCode?: string | null;
    elapsedMs: number;
    dimensions: number | null;
  }>;
}
export interface ChatGptConnectionState {
  id: string;
  label: string;
  status:
    'connected' | 'signed-out' | 'permission-required' | 'reconnect-required';
  models: Array<{ slug: string; displayName: string }>;
}
