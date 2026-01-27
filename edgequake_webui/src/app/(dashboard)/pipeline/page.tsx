/**
 * @module PipelinePage
 * @description Dedicated page for monitoring document ingestion pipeline.
 * Provides real-time visibility into processing stages, document status,
 * and overall pipeline health.
 *
 * @implements FEAT0004 - Processing status tracking
 * @implements UC0007 - User monitors document processing progress
 */
import { PipelineMonitor } from '@/components/pipeline/pipeline-monitor';

export default function PipelinePage() {
  return <PipelineMonitor />;
}
