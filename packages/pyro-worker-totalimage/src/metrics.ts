/**
 * Metrics collection for TotalImage PYRO Worker
 *
 * Provides in-memory metrics tracking for job processing.
 * Can be exported to Prometheus, Datadog, or other monitoring systems.
 */

export interface WorkerMetrics {
  /** Total jobs processed by type and status */
  jobsProcessed: Record<string, { success: number; failed: number }>;

  /** Job duration statistics in milliseconds */
  jobDurations: Record<string, { min: number; max: number; avg: number; count: number }>;

  /** MCP client request counts */
  mcpRequests: { total: number; errors: number };

  /** Queue statistics */
  queueStats: {
    waiting: number;
    active: number;
    completed: number;
    failed: number;
  };

  /** Worker uptime in seconds */
  uptimeSeconds: number;

  /** Last updated timestamp */
  lastUpdated: string;
}

/**
 * Metrics collector for worker statistics
 */
export class MetricsCollector {
  private metrics: WorkerMetrics;
  private startTime: number;
  private durations: Map<string, number[]> = new Map();

  constructor() {
    this.startTime = Date.now();
    this.metrics = {
      jobsProcessed: {},
      jobDurations: {},
      mcpRequests: { total: 0, errors: 0 },
      queueStats: { waiting: 0, active: 0, completed: 0, failed: 0 },
      uptimeSeconds: 0,
      lastUpdated: new Date().toISOString(),
    };
  }

  /**
   * Record a job completion
   */
  recordJobCompletion(jobType: string, success: boolean, durationMs: number): void {
    // Initialize job type if not exists
    if (!this.metrics.jobsProcessed[jobType]) {
      this.metrics.jobsProcessed[jobType] = { success: 0, failed: 0 };
    }

    // Increment counters
    if (success) {
      this.metrics.jobsProcessed[jobType]!.success++;
    } else {
      this.metrics.jobsProcessed[jobType]!.failed++;
    }

    // Track duration
    if (!this.durations.has(jobType)) {
      this.durations.set(jobType, []);
    }
    this.durations.get(jobType)!.push(durationMs);

    // Update duration statistics
    this.updateDurationStats(jobType);
    this.metrics.lastUpdated = new Date().toISOString();
  }

  /**
   * Record an MCP client request
   */
  recordMCPRequest(success: boolean): void {
    this.metrics.mcpRequests.total++;
    if (!success) {
      this.metrics.mcpRequests.errors++;
    }
    this.metrics.lastUpdated = new Date().toISOString();
  }

  /**
   * Update queue statistics
   */
  updateQueueStats(stats: { waiting: number; active: number; completed: number; failed: number }): void {
    this.metrics.queueStats = stats;
    this.metrics.lastUpdated = new Date().toISOString();
  }

  /**
   * Get current metrics snapshot
   */
  getMetrics(): WorkerMetrics {
    this.metrics.uptimeSeconds = Math.floor((Date.now() - this.startTime) / 1000);
    return JSON.parse(JSON.stringify(this.metrics)) as WorkerMetrics;
  }

  /**
   * Export metrics in Prometheus format
   */
  exportPrometheus(): string {
    const lines: string[] = [];

    // Job counts
    lines.push('# HELP totalimage_worker_jobs_total Total number of jobs processed');
    lines.push('# TYPE totalimage_worker_jobs_total counter');
    for (const [jobType, counts] of Object.entries(this.metrics.jobsProcessed)) {
      lines.push(`totalimage_worker_jobs_total{job_type="${jobType}",status="success"} ${counts.success}`);
      lines.push(`totalimage_worker_jobs_total{job_type="${jobType}",status="failed"} ${counts.failed}`);
    }

    // Job durations
    lines.push('# HELP totalimage_worker_job_duration_seconds Job duration in seconds');
    lines.push('# TYPE totalimage_worker_job_duration_seconds summary');
    for (const [jobType, stats] of Object.entries(this.metrics.jobDurations)) {
      lines.push(`totalimage_worker_job_duration_seconds{job_type="${jobType}",quantile="0.0"} ${stats.min / 1000}`);
      lines.push(`totalimage_worker_job_duration_seconds{job_type="${jobType}",quantile="0.5"} ${stats.avg / 1000}`);
      lines.push(`totalimage_worker_job_duration_seconds{job_type="${jobType}",quantile="1.0"} ${stats.max / 1000}`);
      lines.push(`totalimage_worker_job_duration_seconds_sum{job_type="${jobType}"} ${(stats.avg * stats.count) / 1000}`);
      lines.push(`totalimage_worker_job_duration_seconds_count{job_type="${jobType}"} ${stats.count}`);
    }

    // MCP requests
    lines.push('# HELP totalimage_worker_mcp_requests_total Total MCP client requests');
    lines.push('# TYPE totalimage_worker_mcp_requests_total counter');
    lines.push(`totalimage_worker_mcp_requests_total{status="success"} ${this.metrics.mcpRequests.total - this.metrics.mcpRequests.errors}`);
    lines.push(`totalimage_worker_mcp_requests_total{status="error"} ${this.metrics.mcpRequests.errors}`);

    // Queue stats
    lines.push('# HELP totalimage_worker_queue_size Current queue size by state');
    lines.push('# TYPE totalimage_worker_queue_size gauge');
    lines.push(`totalimage_worker_queue_size{state="waiting"} ${this.metrics.queueStats.waiting}`);
    lines.push(`totalimage_worker_queue_size{state="active"} ${this.metrics.queueStats.active}`);
    lines.push(`totalimage_worker_queue_size{state="completed"} ${this.metrics.queueStats.completed}`);
    lines.push(`totalimage_worker_queue_size{state="failed"} ${this.metrics.queueStats.failed}`);

    // Uptime
    lines.push('# HELP totalimage_worker_uptime_seconds Worker uptime in seconds');
    lines.push('# TYPE totalimage_worker_uptime_seconds gauge');
    lines.push(`totalimage_worker_uptime_seconds ${this.metrics.uptimeSeconds}`);

    return lines.join('\n') + '\n';
  }

  /**
   * Reset all metrics
   */
  reset(): void {
    this.startTime = Date.now();
    this.durations.clear();
    this.metrics = {
      jobsProcessed: {},
      jobDurations: {},
      mcpRequests: { total: 0, errors: 0 },
      queueStats: { waiting: 0, active: 0, completed: 0, failed: 0 },
      uptimeSeconds: 0,
      lastUpdated: new Date().toISOString(),
    };
  }

  /**
   * Update duration statistics for a job type
   */
  private updateDurationStats(jobType: string): void {
    const durations = this.durations.get(jobType);
    if (!durations || durations.length === 0) return;

    const min = Math.min(...durations);
    const max = Math.max(...durations);
    const avg = durations.reduce((a, b) => a + b, 0) / durations.length;

    this.metrics.jobDurations[jobType] = { min, max, avg, count: durations.length };
  }
}
