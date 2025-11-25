/**
 * TotalImage PYRO Worker
 *
 * BullMQ worker for processing disk image analysis jobs.
 * Integrates with PYRO Platform job queue system.
 */

import { Worker, Job, Queue, QueueEvents } from 'bullmq';
import type { Redis } from 'ioredis';
import { TotalImageMCPClient } from './client.js';
import type {
  WorkerConfig,
  JobData,
  JobProgress,
  AnalyzeJob,
  ListPartitionsJob,
  ListFilesJob,
  ExtractFileJob,
  ValidateIntegrityJob,
  BatchAnalyzeJob,
  BatchExtractJob,
  AnalyzeImageResult,
  ListPartitionsResult,
  ListFilesResult,
  ExtractFileResult,
  ValidateIntegrityResult,
} from './types.js';

// ============================================================================
// Constants
// ============================================================================

const QUEUE_NAME = 'totalimage';
const WORKER_NAME = 'totalimage-worker';

// ============================================================================
// Worker Class
// ============================================================================

export class TotalImageWorker {
  private worker: Worker | null = null;
  private queue: Queue | null = null;
  private queueEvents: QueueEvents | null = null;
  private mcpClient: TotalImageMCPClient;
  private config: WorkerConfig;
  private isShuttingDown = false;

  constructor(config: WorkerConfig) {
    this.config = config;
    this.mcpClient = new TotalImageMCPClient(config.mcp);
  }

  /**
   * Start the worker
   */
  async start(): Promise<void> {
    const redisConnection = {
      host: this.parseRedisHost(this.config.queue.redis),
      port: this.parseRedisPort(this.config.queue.redis),
      maxRetriesPerRequest: null,
    };

    // Create queue for adding jobs
    this.queue = new Queue(QUEUE_NAME, {
      connection: redisConnection,
    });

    // Create queue events for monitoring
    this.queueEvents = new QueueEvents(QUEUE_NAME, {
      connection: redisConnection,
    });

    // Create worker
    this.worker = new Worker(
      QUEUE_NAME,
      async (job: Job<JobData>) => this.processJob(job),
      {
        connection: redisConnection,
        concurrency: this.config.queue.concurrency,
        name: WORKER_NAME,
      }
    );

    // Set up event handlers
    this.worker.on('completed', (job) => {
      this.log('info', `Job ${job.id} completed successfully`);
    });

    this.worker.on('failed', (job, err) => {
      this.log('error', `Job ${job?.id} failed: ${err.message}`);
    });

    this.worker.on('error', (err) => {
      this.log('error', `Worker error: ${err.message}`);
    });

    // Verify MCP server connection
    const healthy = await this.mcpClient.healthCheck();
    if (!healthy) {
      this.log('warn', 'MCP server health check failed - jobs may fail');
    }

    this.log('info', `TotalImage worker started with concurrency ${this.config.queue.concurrency}`);
  }

  /**
   * Stop the worker gracefully
   */
  async stop(): Promise<void> {
    this.isShuttingDown = true;
    this.log('info', 'Shutting down worker...');

    if (this.worker) {
      await this.worker.close();
    }

    if (this.queueEvents) {
      await this.queueEvents.close();
    }

    if (this.queue) {
      await this.queue.close();
    }

    this.log('info', 'Worker shutdown complete');
  }

  /**
   * Add a job to the queue
   */
  async addJob(data: JobData, opts?: { priority?: number; delay?: number }): Promise<string> {
    if (!this.queue) {
      throw new Error('Worker not started');
    }

    const job = await this.queue.add(data.job_type, data, {
      priority: opts?.priority ?? data.priority ?? 0,
      delay: opts?.delay,
      attempts: this.config.queue.maxRetries,
      backoff: {
        type: this.config.queue.backoffType ?? 'exponential',
        delay: this.config.queue.backoffDelay ?? 1000,
      },
    });

    return job.id ?? '';
  }

  /**
   * Process a job
   */
  private async processJob(job: Job<JobData>): Promise<unknown> {
    const { data } = job;

    this.log('info', `Processing job ${job.id}: ${data.job_type}`);

    try {
      switch (data.job_type) {
        case 'analyze_disk_image':
          return await this.handleAnalyzeJob(job as Job<AnalyzeJob>);

        case 'list_partitions':
          return await this.handleListPartitionsJob(job as Job<ListPartitionsJob>);

        case 'list_files':
          return await this.handleListFilesJob(job as Job<ListFilesJob>);

        case 'extract_file':
          return await this.handleExtractFileJob(job as Job<ExtractFileJob>);

        case 'validate_integrity':
          return await this.handleValidateIntegrityJob(job as Job<ValidateIntegrityJob>);

        case 'batch_analyze':
          return await this.handleBatchAnalyzeJob(job as Job<BatchAnalyzeJob>);

        case 'batch_extract':
          return await this.handleBatchExtractJob(job as Job<BatchExtractJob>);

        default:
          throw new Error(`Unknown job type: ${(data as JobData).job_type}`);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      this.log('error', `Job ${job.id} failed: ${message}`);
      throw error;
    }
  }

  // ==========================================================================
  // Job Handlers
  // ==========================================================================

  private async handleAnalyzeJob(job: Job<AnalyzeJob>): Promise<AnalyzeImageResult> {
    await job.updateProgress({ stage: 'analyzing', percentage: 0 });

    const result = await this.mcpClient.analyzeImage(job.data.input);

    await job.updateProgress({ stage: 'complete', percentage: 100 });
    return result;
  }

  private async handleListPartitionsJob(job: Job<ListPartitionsJob>): Promise<ListPartitionsResult> {
    await job.updateProgress({ stage: 'listing_partitions', percentage: 0 });

    const result = await this.mcpClient.listPartitions(job.data.input);

    await job.updateProgress({ stage: 'complete', percentage: 100 });
    return result;
  }

  private async handleListFilesJob(job: Job<ListFilesJob>): Promise<ListFilesResult> {
    await job.updateProgress({ stage: 'listing_files', percentage: 0 });

    const result = await this.mcpClient.listFiles(job.data.input);

    await job.updateProgress({ stage: 'complete', percentage: 100 });
    return result;
  }

  private async handleExtractFileJob(job: Job<ExtractFileJob>): Promise<ExtractFileResult> {
    await job.updateProgress({ stage: 'extracting', percentage: 0 });

    const result = await this.mcpClient.extractFile(job.data.input);

    await job.updateProgress({ stage: 'complete', percentage: 100 });
    return result;
  }

  private async handleValidateIntegrityJob(job: Job<ValidateIntegrityJob>): Promise<ValidateIntegrityResult> {
    await job.updateProgress({ stage: 'validating', percentage: 0 });

    const result = await this.mcpClient.validateIntegrity(job.data.input);

    await job.updateProgress({ stage: 'complete', percentage: 100 });
    return result;
  }

  private async handleBatchAnalyzeJob(job: Job<BatchAnalyzeJob>): Promise<AnalyzeImageResult[]> {
    const { paths, deep_scan } = job.data.input;
    const results: AnalyzeImageResult[] = [];

    for (let i = 0; i < paths.length; i++) {
      const path = paths[i];
      if (!path) continue;

      await job.updateProgress({
        stage: `analyzing_${i + 1}_of_${paths.length}`,
        percentage: Math.round((i / paths.length) * 100),
      });

      const result = await this.mcpClient.analyzeImage({
        path,
        deep_scan,
        cache: true,
      });

      results.push(result);
    }

    await job.updateProgress({ stage: 'complete', percentage: 100 });
    return results;
  }

  private async handleBatchExtractJob(job: Job<BatchExtractJob>): Promise<ExtractFileResult[]> {
    const { image_path, zone_index, file_paths, output_directory } = job.data.input;
    const results: ExtractFileResult[] = [];

    for (let i = 0; i < file_paths.length; i++) {
      const file_path = file_paths[i];
      if (!file_path) continue;

      await job.updateProgress({
        stage: `extracting_${i + 1}_of_${file_paths.length}`,
        percentage: Math.round((i / file_paths.length) * 100),
      });

      const result = await this.mcpClient.extractFile({
        image_path,
        file_path,
        zone_index,
        output_path: output_directory,
      });

      results.push(result);
    }

    await job.updateProgress({ stage: 'complete', percentage: 100 });
    return results;
  }

  // ==========================================================================
  // Utility Methods
  // ==========================================================================

  private parseRedisHost(url: string): string {
    try {
      const parsed = new URL(url);
      return parsed.hostname;
    } catch {
      return 'localhost';
    }
  }

  private parseRedisPort(url: string): number {
    try {
      const parsed = new URL(url);
      return parseInt(parsed.port, 10) || 6379;
    } catch {
      return 6379;
    }
  }

  private log(level: 'debug' | 'info' | 'warn' | 'error', message: string): void {
    const configLevel = this.config.logging?.level ?? 'info';
    const levels = ['debug', 'info', 'warn', 'error'];

    if (levels.indexOf(level) >= levels.indexOf(configLevel)) {
      const timestamp = new Date().toISOString();
      const format = this.config.logging?.format ?? 'pretty';

      if (format === 'json') {
        console.log(JSON.stringify({ timestamp, level, message, worker: WORKER_NAME }));
      } else {
        console.log(`[${timestamp}] [${level.toUpperCase()}] [${WORKER_NAME}] ${message}`);
      }
    }
  }

  /**
   * Get queue statistics
   */
  async getStats(): Promise<{
    waiting: number;
    active: number;
    completed: number;
    failed: number;
  }> {
    if (!this.queue) {
      throw new Error('Worker not started');
    }

    const counts = await this.queue.getJobCounts();
    return {
      waiting: counts.waiting ?? 0,
      active: counts.active ?? 0,
      completed: counts.completed ?? 0,
      failed: counts.failed ?? 0,
    };
  }
}

// ============================================================================
// Factory Function
// ============================================================================

export function createWorker(config: WorkerConfig): TotalImageWorker {
  return new TotalImageWorker(config);
}

export default TotalImageWorker;
