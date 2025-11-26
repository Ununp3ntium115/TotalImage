/**
 * TotalImage PYRO Worker Types
 *
 * Type definitions for disk image analysis jobs and results.
 */

import { z } from 'zod';

// ============================================================================
// MCP Tool Schemas
// ============================================================================

export const AnalyzeImageInputSchema = z.object({
  path: z.string().describe('Path to the disk image file'),
  deep_scan: z.boolean().optional().default(false),
  cache: z.boolean().optional().default(true),
});

export const ListPartitionsInputSchema = z.object({
  path: z.string().describe('Path to the disk image file'),
  cache: z.boolean().optional().default(true),
});

export const ListFilesInputSchema = z.object({
  path: z.string().describe('Path to the disk image file'),
  zone_index: z.number().int().min(0).describe('Partition/zone index'),
  directory: z.string().optional().default('/'),
  cache: z.boolean().optional().default(true),
});

export const ExtractFileInputSchema = z.object({
  image_path: z.string().describe('Path to the disk image file'),
  file_path: z.string().describe('Path to file within the image'),
  zone_index: z.number().int().min(0).describe('Partition/zone index'),
  output_path: z.string().describe('Destination path for extracted file'),
});

export const ValidateIntegrityInputSchema = z.object({
  path: z.string().describe('Path to the disk image file'),
  check_checksums: z.boolean().optional().default(true),
  check_boot_sectors: z.boolean().optional().default(true),
});

// ============================================================================
// Inferred Types
// ============================================================================

export type AnalyzeImageInput = z.infer<typeof AnalyzeImageInputSchema>;
export type ListPartitionsInput = z.infer<typeof ListPartitionsInputSchema>;
export type ListFilesInput = z.infer<typeof ListFilesInputSchema>;
export type ExtractFileInput = z.infer<typeof ExtractFileInputSchema>;
export type ValidateIntegrityInput = z.infer<typeof ValidateIntegrityInputSchema>;

// ============================================================================
// Result Types
// ============================================================================

export interface Partition {
  index: number;
  type: string;
  offset: number;
  length: number;
  bootable?: boolean;
}

export interface FileEntry {
  name: string;
  size: number;
  is_directory: boolean;
  modified?: string;
  created?: string;
  attributes?: string[];
}

export interface FilesystemInfo {
  zone_index: number;
  type: string;
  label?: string;
  total_size: number;
  free_size?: number;
  cluster_size?: number;
}

export interface AnalyzeImageResult {
  vault_type: string;
  vault_size: number;
  partitions: Partition[];
  filesystems: FilesystemInfo[];
  metadata?: Record<string, unknown>;
}

export interface ListPartitionsResult {
  partition_table: 'MBR' | 'GPT' | 'None';
  zones: Partition[];
}

export interface ListFilesResult {
  files: FileEntry[];
  directory: string;
  total_count: number;
}

export interface ExtractFileResult {
  success: boolean;
  output_path: string;
  size: number;
  checksum: string;
}

export interface ValidateIntegrityResult {
  valid: boolean;
  issues: string[];
  checksums: {
    md5?: string;
    sha1?: string;
    sha256?: string;
  };
  boot_sector_valid?: boolean;
  partition_table_valid?: boolean;
}

// ============================================================================
// Job Types
// ============================================================================

export type JobType =
  | 'analyze_disk_image'
  | 'list_partitions'
  | 'list_files'
  | 'extract_file'
  | 'validate_integrity'
  | 'batch_analyze'
  | 'batch_extract';

export interface BaseJobData {
  job_type: JobType;
  user_id: string;
  project_id?: string;
  priority?: number;
  metadata?: Record<string, unknown>;
}

export interface AnalyzeJob extends BaseJobData {
  job_type: 'analyze_disk_image';
  input: AnalyzeImageInput;
}

export interface ListPartitionsJob extends BaseJobData {
  job_type: 'list_partitions';
  input: ListPartitionsInput;
}

export interface ListFilesJob extends BaseJobData {
  job_type: 'list_files';
  input: ListFilesInput;
}

export interface ExtractFileJob extends BaseJobData {
  job_type: 'extract_file';
  input: ExtractFileInput;
}

export interface ValidateIntegrityJob extends BaseJobData {
  job_type: 'validate_integrity';
  input: ValidateIntegrityInput;
}

export interface BatchAnalyzeJob extends BaseJobData {
  job_type: 'batch_analyze';
  input: {
    paths: string[];
    deep_scan?: boolean;
  };
}

export interface BatchExtractJob extends BaseJobData {
  job_type: 'batch_extract';
  input: {
    image_path: string;
    zone_index: number;
    file_paths: string[];
    output_directory: string;
  };
}

export type JobData =
  | AnalyzeJob
  | ListPartitionsJob
  | ListFilesJob
  | ExtractFileJob
  | ValidateIntegrityJob
  | BatchAnalyzeJob
  | BatchExtractJob;

// ============================================================================
// Configuration Types
// ============================================================================

export interface TotalImageMCPConfig {
  url: string;
  timeout: number;
  apiKey?: string;
  retries?: number;
  retryDelay?: number;
}

export interface QueueConfig {
  redis: string;
  concurrency: number;
  maxRetries: number;
  backoffType?: 'exponential' | 'fixed';
  backoffDelay?: number;
}

export interface StorageConfig {
  type: 's3' | 'local' | 'minio';
  bucket?: string;
  region?: string;
  endpoint?: string;
  accessKeyId?: string;
  secretAccessKey?: string;
  localPath?: string;
}

export interface WorkerConfig {
  mcp: TotalImageMCPConfig;
  queue: QueueConfig;
  storage?: StorageConfig;
  logging?: {
    level: 'debug' | 'info' | 'warn' | 'error';
    format?: 'json' | 'pretty';
  };
}

// ============================================================================
// Event Types
// ============================================================================

export interface JobProgress {
  job_id: string;
  percentage: number;
  stage: string;
  message?: string;
  timestamp: string;
}

export interface JobCompleted<T = unknown> {
  job_id: string;
  result: T;
  duration_ms: number;
  timestamp: string;
}

export interface JobFailed {
  job_id: string;
  error: string;
  stack?: string;
  attempts: number;
  timestamp: string;
}

export type WorkerEvent =
  | { type: 'progress'; data: JobProgress }
  | { type: 'completed'; data: JobCompleted }
  | { type: 'failed'; data: JobFailed };
