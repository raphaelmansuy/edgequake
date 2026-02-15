/**
 * Document management tools.
 */
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { readFile } from "fs/promises";
import { basename, extname } from "path";
import { z } from "zod";
import { getClient } from "../client.js";
import { formatError } from "../errors.js";

export function registerDocumentTools(server: McpServer): void {
  // document_upload
  server.tool(
    "document_upload",
    "Upload a text document to EdgeQuake for knowledge graph extraction. The document will be chunked, entities extracted, and relationships mapped.",
    {
      content: z.string().describe("Document text content"),
      title: z.string().optional().describe("Document title"),
      metadata: z
        .record(z.unknown())
        .optional()
        .describe("Custom metadata key-value pairs"),
      enable_gleaning: z
        .boolean()
        .optional()
        .describe(
          "Enable multi-pass extraction for better recall (default: true)",
        ),
    },
    async (params) => {
      try {
        const client = await getClient();
        const result = await client.documents.upload({
          content: params.content,
          title: params.title,
          metadata: params.metadata,
          enable_gleaning: params.enable_gleaning,
          async_processing: true,
        });

        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify(
                {
                  document_id: result.document_id,
                  status: result.status,
                  task_id: result.task_id,
                  track_id: result.track_id,
                  chunk_count: result.chunk_count,
                  entity_count: result.entity_count,
                  relationship_count: result.relationship_count,
                },
                null,
                2,
              ),
            },
          ],
        };
      } catch (error) {
        return formatError(error);
      }
    },
  );

  // document_upload_file
  server.tool(
    "document_upload_file",
    "Upload a file from a file path to EdgeQuake for knowledge graph extraction. Supports text files (.txt, .md) and PDFs (.pdf). The file will be read, uploaded, chunked, and entities extracted.",
    {
      file_path: z.string().describe("Absolute path to the file to upload"),
      title: z
        .string()
        .optional()
        .describe("Document title (defaults to filename)"),
      metadata: z
        .record(z.unknown())
        .optional()
        .describe("Custom metadata key-value pairs"),
      enable_gleaning: z
        .boolean()
        .optional()
        .describe(
          "Enable multi-pass extraction for better recall (default: true)",
        ),
    },
    async (params) => {
      try {
        const client = await getClient();
        const fileExt = extname(params.file_path).toLowerCase();
        const fileName = basename(params.file_path);
        const title = params.title || fileName;

        // Read file from filesystem
        const fileBuffer = await readFile(params.file_path);

        // Determine file type and upload accordingly
        if (fileExt === ".pdf") {
          // Upload as PDF using pdf.upload
          const blob = new Blob([fileBuffer], { type: "application/pdf" });
          // Create a File-like object
          const file = Object.assign(blob, {
            name: fileName,
            lastModified: Date.now(),
          }) as File;

          const result = await client.documents.pdf.upload(
            file,
            params.metadata as Record<string, string> | undefined,
          );

          return {
            content: [
              {
                type: "text" as const,
                text: JSON.stringify(
                  {
                    document_id: result.pdf_id,
                    file_name: fileName,
                    file_type: "pdf",
                    status: result.status,
                    track_id: result.track_id,
                    message:
                      result.message ||
                      "PDF uploaded successfully. Use document_status to track processing.",
                  },
                  null,
                  2,
                ),
              },
            ],
          };
        } else if (
          fileExt === ".txt" ||
          fileExt === ".md" ||
          fileExt === ".markdown"
        ) {
          // Upload as text file using uploadFile
          const textContent = fileBuffer.toString("utf-8");
          const blob = new Blob([textContent], { type: "text/plain" });
          const file = Object.assign(blob, {
            name: fileName,
            lastModified: Date.now(),
          }) as File;

          const result = await client.documents.uploadFile(file);

          return {
            content: [
              {
                type: "text" as const,
                text: JSON.stringify(
                  {
                    document_id: result.document_id,
                    file_name: fileName,
                    file_type: fileExt.substring(1),
                    status: result.status,
                    track_id: result.track_id,
                    chunk_count: result.chunk_count,
                    entity_count: result.entity_count,
                    relationship_count: result.relationship_count,
                    message: "File uploaded successfully.",
                  },
                  null,
                  2,
                ),
              },
            ],
          };
        } else {
          return {
            content: [
              {
                type: "text" as const,
                text: JSON.stringify(
                  {
                    error: "Unsupported file type",
                    message: `File extension '${fileExt}' is not supported. Please use .txt, .md, .markdown, or .pdf files.`,
                    file_path: params.file_path,
                  },
                  null,
                  2,
                ),
              },
            ],
            isError: true,
          };
        }
      } catch (error) {
        // Check if it's a file read error
        if (
          error instanceof Error &&
          (error.message.includes("ENOENT") ||
            error.message.includes("no such file"))
        ) {
          return {
            content: [
              {
                type: "text" as const,
                text: JSON.stringify(
                  {
                    error: "File not found",
                    message: `The file at path '${params.file_path}' does not exist or cannot be accessed.`,
                    file_path: params.file_path,
                  },
                  null,
                  2,
                ),
              },
            ],
            isError: true,
          };
        }
        return formatError(error);
      }
    },
  );

  // document_list
  server.tool(
    "document_list",
    "List documents with pagination and filtering",
    {
      page: z.number().optional().describe("Page number (default: 1)"),
      page_size: z.number().optional().describe("Items per page (default: 20)"),
      status: z
        .enum(["pending", "processing", "completed", "failed"])
        .optional()
        .describe("Filter by processing status"),
      search: z
        .string()
        .optional()
        .describe("Full-text search in title/content"),
    },
    async (params) => {
      try {
        const client = await getClient();
        const result = await client.documents.list({
          page: params.page,
          page_size: params.page_size,
          status: params.status,
          search: params.search,
        });

        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify(
                {
                  documents: result.documents.map((d) => ({
                    id: d.id,
                    title: d.title,
                    status: d.status,
                    chunk_count: d.chunk_count,
                    entity_count: d.entity_count,
                    content_summary: d.content_summary,
                    source_type: d.source_type,
                    created_at: d.created_at,
                  })),
                  total: result.total,
                  page: result.page,
                  page_size: result.page_size,
                  has_more: result.has_more,
                },
                null,
                2,
              ),
            },
          ],
        };
      } catch (error) {
        return formatError(error);
      }
    },
  );

  // document_get
  server.tool(
    "document_get",
    "Get document details including full content and metadata",
    {
      document_id: z.string().describe("Document UUID"),
    },
    async (params) => {
      try {
        const client = await getClient();
        const doc = await client.documents.get(params.document_id);

        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify(
                {
                  id: doc.id,
                  title: doc.title,
                  content: doc.content,
                  status: doc.status,
                  chunk_count: doc.chunk_count,
                  entity_count: doc.entity_count,
                  source_type: doc.source_type,
                  current_stage: doc.current_stage,
                  stage_progress: doc.stage_progress,
                  metadata: doc.metadata,
                  created_at: doc.created_at,
                  updated_at: doc.updated_at,
                },
                null,
                2,
              ),
            },
          ],
        };
      } catch (error) {
        return formatError(error);
      }
    },
  );

  // document_delete
  server.tool(
    "document_delete",
    "Delete a document and its extracted knowledge (entities, relationships, chunks)",
    {
      document_id: z.string().describe("Document UUID"),
    },
    async (params) => {
      try {
        const client = await getClient();
        await client.documents.delete(params.document_id);

        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify({
                success: true,
                document_id: params.document_id,
              }),
            },
          ],
        };
      } catch (error) {
        return formatError(error);
      }
    },
  );

  // document_status
  server.tool(
    "document_status",
    "Check the processing status of a document (useful after async upload)",
    {
      document_id: z.string().describe("Document UUID"),
    },
    async (params) => {
      try {
        const client = await getClient();
        const doc = await client.documents.get(params.document_id);

        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify(
                {
                  id: doc.id,
                  status: doc.status,
                  current_stage: doc.current_stage,
                  stage_progress: doc.stage_progress,
                  stage_message: doc.stage_message,
                  error_message: doc.error_message,
                  chunk_count: doc.chunk_count,
                  entity_count: doc.entity_count,
                },
                null,
                2,
              ),
            },
          ],
        };
      } catch (error) {
        return formatError(error);
      }
    },
  );
}
