/**
 * Shared resource — public access to shared conversations.
 *
 * @module resources/shared
 * @see edgequake/crates/edgequake-api/src/handlers/conversations.rs
 */

import type { ConversationDetail } from "../types/conversations.js";
import { Resource } from "./base.js";

export class SharedResource extends Resource {
  /** Get a shared conversation by share ID (public, no auth). */
  async get(shareId: string): Promise<ConversationDetail> {
    return this._get(`/api/v1/shared/${shareId}`);
  }
}
