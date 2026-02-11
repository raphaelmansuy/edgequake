/**
 * Barrel export coverage — ensures resources/index.ts exports are exercised.
 *
 * WHY: v8 coverage counts barrel re-exports as uncovered unless imported
 * from the barrel path. This test imports from the barrel to cover it.
 */
import { describe, it, expect } from "vitest";
import {
  ApiKeysResource,
  AuthResource,
  ChatResource,
  ChunksResource,
  ConversationsResource,
  MessagesResource,
  CostsResource,
  DocumentsResource,
  PdfResource,
  FoldersResource,
  EntitiesResource,
  GraphResource,
  RelationshipsResource,
  LineageResource,
  ModelsResource,
  OllamaResource,
  PipelineResource,
  ProvenanceResource,
  QueryResource,
  SettingsResource,
  SharedResource,
  TasksResource,
  TenantsResource,
  UsersResource,
  WorkspacesResource,
} from "../../src/resources/index.js";

describe("resources barrel export", () => {
  it("exports all 25 resource classes", () => {
    const resources = [
      ApiKeysResource,
      AuthResource,
      ChatResource,
      ChunksResource,
      ConversationsResource,
      MessagesResource,
      CostsResource,
      DocumentsResource,
      PdfResource,
      FoldersResource,
      EntitiesResource,
      GraphResource,
      RelationshipsResource,
      LineageResource,
      ModelsResource,
      OllamaResource,
      PipelineResource,
      ProvenanceResource,
      QueryResource,
      SettingsResource,
      SharedResource,
      TasksResource,
      TenantsResource,
      UsersResource,
      WorkspacesResource,
    ];

    for (const Resource of resources) {
      expect(Resource).toBeDefined();
      expect(typeof Resource).toBe("function");
    }
  });
});
