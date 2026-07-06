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

/** Human label for provider auth model (SPEC-043 §011). */
export function getProviderAuthLabel(
  providerId: string,
  authKind?: string,
): string | null {
  const kind =
    authKind ??
    (providerId === "vertexai" ? "oauth2_identity" : undefined);
  switch (kind) {
    case "oauth2_identity":
      return "Identity (ADC)";
    case "api_key":
      return "API key";
    case "local":
      return "Local";
    case "aws_chain":
      return "AWS IAM";
    default:
      return null;
  }
}
