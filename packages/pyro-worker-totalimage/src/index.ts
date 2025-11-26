/**
 * @pyro/worker-totalimage
 *
 * PYRO Platform worker package for TotalImage disk image analysis.
 *
 * @example
 * ```typescript
 * import { createWorker, createMCPClient } from '@pyro/worker-totalimage';
 *
 * // Create a standalone client
 * const client = createMCPClient({
 *   url: 'http://localhost:3002',
 *   timeout: 30000,
 *   apiKey: process.env.MCP_API_KEY,
 * });
 *
 * // Analyze a disk image
 * const result = await client.analyzeImage({
 *   path: '/images/disk.vhd',
 *   deep_scan: false,
 *   cache: true,
 * });
 *
 * // Or create a worker for job queue processing
 * const worker = createWorker({
 *   mcp: {
 *     url: 'http://localhost:3002',
 *     timeout: 30000,
 *   },
 *   queue: {
 *     redis: 'redis://localhost:6379',
 *     concurrency: 5,
 *     maxRetries: 3,
 *   },
 * });
 *
 * await worker.start();
 *
 * // Add a job
 * const jobId = await worker.addJob({
 *   job_type: 'analyze_disk_image',
 *   user_id: 'user-123',
 *   input: {
 *     path: '/images/disk.vhd',
 *   },
 * });
 * ```
 */

// Re-export types
export type {
  // Input types
  AnalyzeImageInput,
  ListPartitionsInput,
  ListFilesInput,
  ExtractFileInput,
  ValidateIntegrityInput,

  // Result types
  AnalyzeImageResult,
  ListPartitionsResult,
  ListFilesResult,
  ExtractFileResult,
  ValidateIntegrityResult,

  // Data types
  Partition,
  FileEntry,
  FilesystemInfo,

  // Job types
  JobType,
  JobData,
  BaseJobData,
  AnalyzeJob,
  ListPartitionsJob,
  ListFilesJob,
  ExtractFileJob,
  ValidateIntegrityJob,
  BatchAnalyzeJob,
  BatchExtractJob,

  // Config types
  WorkerConfig,
  TotalImageMCPConfig,
  QueueConfig,
  StorageConfig,

  // Event types
  JobProgress,
  JobCompleted,
  JobFailed,
  WorkerEvent,
} from './types.js';

// Re-export schemas for runtime validation
export {
  AnalyzeImageInputSchema,
  ListPartitionsInputSchema,
  ListFilesInputSchema,
  ExtractFileInputSchema,
  ValidateIntegrityInputSchema,
} from './types.js';

// Re-export client
export {
  TotalImageMCPClient,
  createMCPClient,
  MCPClientError,
  MCPConnectionError,
  MCPToolError,
} from './client.js';

// Re-export worker
export { TotalImageWorker, createWorker } from './worker.js';

// Default export for convenience
export { createMCPClient as default } from './client.js';
