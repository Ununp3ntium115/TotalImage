/**
 * TotalImage MCP Client
 *
 * HTTP client for communicating with the TotalImage MCP server.
 * Implements JSON-RPC 2.0 protocol over HTTP.
 */

import axios, { AxiosInstance, AxiosError } from 'axios';
import type {
  TotalImageMCPConfig,
  AnalyzeImageInput,
  AnalyzeImageResult,
  ListPartitionsInput,
  ListPartitionsResult,
  ListFilesInput,
  ListFilesResult,
  ExtractFileInput,
  ExtractFileResult,
  ValidateIntegrityInput,
  ValidateIntegrityResult,
} from './types.js';

// ============================================================================
// JSON-RPC Types
// ============================================================================

interface JsonRpcRequest {
  jsonrpc: '2.0';
  id: number | string;
  method: string;
  params: {
    name: string;
    arguments: Record<string, unknown>;
  };
}

interface JsonRpcResponse<T = unknown> {
  jsonrpc: '2.0';
  id: number | string;
  result?: {
    content?: Array<{ type: string; text: string }>;
    [key: string]: unknown;
  } | T;
  error?: {
    code: number;
    message: string;
    data?: unknown;
  };
}

// ============================================================================
// Client Errors
// ============================================================================

export class MCPClientError extends Error {
  constructor(
    message: string,
    public code: number = -1,
    public data?: unknown
  ) {
    super(message);
    this.name = 'MCPClientError';
  }
}

export class MCPConnectionError extends MCPClientError {
  constructor(message: string, public originalError?: Error) {
    super(message, -32000);
    this.name = 'MCPConnectionError';
  }
}

export class MCPToolError extends MCPClientError {
  constructor(message: string, code: number = -32603, data?: unknown) {
    super(message, code, data);
    this.name = 'MCPToolError';
  }
}

// ============================================================================
// MCP Client
// ============================================================================

export class TotalImageMCPClient {
  private client: AxiosInstance;
  private requestId = 0;
  private config: Required<TotalImageMCPConfig>;

  constructor(config: TotalImageMCPConfig) {
    this.config = {
      url: config.url,
      timeout: config.timeout,
      apiKey: config.apiKey ?? '',
      retries: config.retries ?? 3,
      retryDelay: config.retryDelay ?? 1000,
    };

    this.client = axios.create({
      baseURL: this.config.url,
      timeout: this.config.timeout,
      headers: {
        'Content-Type': 'application/json',
        ...(this.config.apiKey && {
          Authorization: `Bearer ${this.config.apiKey}`,
        }),
      },
    });
  }

  /**
   * Execute a tool call with automatic retries
   */
  private async callTool<T>(
    toolName: string,
    args: Record<string, unknown>
  ): Promise<T> {
    const request: JsonRpcRequest = {
      jsonrpc: '2.0',
      id: ++this.requestId,
      method: 'tools/call',
      params: {
        name: toolName,
        arguments: args,
      },
    };

    let lastError: Error | null = null;

    for (let attempt = 0; attempt <= this.config.retries; attempt++) {
      try {
        const response = await this.client.post<JsonRpcResponse<T>>(
          '/mcp',
          request
        );

        if (response.data.error) {
          throw new MCPToolError(
            response.data.error.message,
            response.data.error.code,
            response.data.error.data
          );
        }

        // Parse result from MCP response format
        const result = response.data.result;
        if (!result) {
          throw new MCPToolError('Empty response from tool');
        }

        // Handle content array format (standard MCP response)
        if (
          'content' in result &&
          Array.isArray(result.content) &&
          result.content[0]?.text
        ) {
          try {
            return JSON.parse(result.content[0].text) as T;
          } catch {
            // Return raw result if not JSON
            return result as T;
          }
        }

        return result as T;
      } catch (error) {
        lastError = error as Error;

        if (error instanceof MCPToolError) {
          // Don't retry tool errors - they're not transient
          throw error;
        }

        if (error instanceof AxiosError) {
          if (error.code === 'ECONNREFUSED' || error.code === 'ENOTFOUND') {
            throw new MCPConnectionError(
              `Failed to connect to MCP server at ${this.config.url}`,
              error
            );
          }

          // Retry on network errors or 5xx responses
          const shouldRetry =
            !error.response || (error.response.status >= 500 && error.response.status < 600);

          if (shouldRetry && attempt < this.config.retries) {
            const delay = this.config.retryDelay * Math.pow(2, attempt);
            await new Promise((resolve) => setTimeout(resolve, delay));
            continue;
          }

          throw new MCPConnectionError(
            `HTTP error: ${error.message}`,
            error
          );
        }

        // Retry unknown errors
        if (attempt < this.config.retries) {
          const delay = this.config.retryDelay * Math.pow(2, attempt);
          await new Promise((resolve) => setTimeout(resolve, delay));
          continue;
        }

        throw error;
      }
    }

    throw lastError ?? new MCPClientError('Unknown error occurred');
  }

  // ==========================================================================
  // Tool Methods
  // ==========================================================================

  /**
   * Analyze a disk image file
   */
  async analyzeImage(input: AnalyzeImageInput): Promise<AnalyzeImageResult> {
    return this.callTool<AnalyzeImageResult>('analyze_disk_image', {
      path: input.path,
      deep_scan: input.deep_scan ?? false,
      cache: input.cache ?? true,
    });
  }

  /**
   * List partitions in a disk image
   */
  async listPartitions(input: ListPartitionsInput): Promise<ListPartitionsResult> {
    return this.callTool<ListPartitionsResult>('list_partitions', {
      path: input.path,
      cache: input.cache ?? true,
    });
  }

  /**
   * List files in a partition
   */
  async listFiles(input: ListFilesInput): Promise<ListFilesResult> {
    return this.callTool<ListFilesResult>('list_files', {
      path: input.path,
      zone_index: input.zone_index,
      directory: input.directory ?? '/',
      cache: input.cache ?? true,
    });
  }

  /**
   * Extract a file from a disk image
   */
  async extractFile(input: ExtractFileInput): Promise<ExtractFileResult> {
    return this.callTool<ExtractFileResult>('extract_file', {
      image_path: input.image_path,
      file_path: input.file_path,
      zone_index: input.zone_index,
      output_path: input.output_path,
    });
  }

  /**
   * Validate disk image integrity
   */
  async validateIntegrity(
    input: ValidateIntegrityInput
  ): Promise<ValidateIntegrityResult> {
    return this.callTool<ValidateIntegrityResult>('validate_integrity', {
      path: input.path,
      check_checksums: input.check_checksums ?? true,
      check_boot_sectors: input.check_boot_sectors ?? true,
    });
  }

  // ==========================================================================
  // Utility Methods
  // ==========================================================================

  /**
   * Check if the MCP server is healthy
   */
  async healthCheck(): Promise<boolean> {
    try {
      const response = await this.client.get('/health', { timeout: 5000 });
      return response.status === 200;
    } catch {
      return false;
    }
  }

  /**
   * Get server information
   */
  async getServerInfo(): Promise<Record<string, unknown>> {
    const request: JsonRpcRequest = {
      jsonrpc: '2.0',
      id: ++this.requestId,
      method: 'initialize',
      params: {
        name: 'pyro-worker-totalimage',
        arguments: {
          protocolVersion: '2024-11-05',
          capabilities: {},
          clientInfo: {
            name: '@pyro/worker-totalimage',
            version: '0.1.0',
          },
        },
      },
    };

    const response = await this.client.post<JsonRpcResponse>('/mcp', request);
    return (response.data.result as Record<string, unknown>) ?? {};
  }
}

// ============================================================================
// Factory Function
// ============================================================================

export function createMCPClient(config: TotalImageMCPConfig): TotalImageMCPClient {
  return new TotalImageMCPClient(config);
}

export default TotalImageMCPClient;
