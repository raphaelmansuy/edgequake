/**
 * Central provider display names (SPEC-043 DRY).
 */
export const PROVIDER_DISPLAY_NAMES: Record<string, string> = {
  openai: "OpenAI",
  ollama: "Ollama",
  lmstudio: "LM Studio",
  anthropic: "Anthropic",
  gemini: "Google Gemini",
  vertexai: "Google Vertex AI",
  xai: "xAI",
  openrouter: "OpenRouter",
  azure: "Azure OpenAI",
  minimax: "MiniMax",
  mistral: "Mistral AI",
  nvidia: "NVIDIA NIM",
  cohere: "Cohere",
  jina: "Jina AI",
  huggingface: "HuggingFace",
  "vscode-copilot": "GitHub Copilot",
  bedrock: "AWS Bedrock",
  mock: "Mock (Dev)",
};

export function getProviderDisplayName(providerId: string): string {
  return PROVIDER_DISPLAY_NAMES[providerId.toLowerCase()] || providerId;
}
