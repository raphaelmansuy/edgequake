/**
 * Settings resource — provider status and configuration.
 *
 * @module resources/settings
 * @see edgequake/crates/edgequake-api/src/handlers/settings.rs
 */

import { Resource } from "./base.js";
import type { ProvidersHealth } from "../types/health.js";

export class SettingsResource extends Resource {
  /** Get provider status information. */
  async providerStatus(): Promise<ProvidersHealth> {
    return this._get("/api/v1/settings/provider/status");
  }

  /** List all available providers. */
  async listProviders(): Promise<ProvidersHealth> {
    return this._get("/api/v1/settings/providers");
  }
}
