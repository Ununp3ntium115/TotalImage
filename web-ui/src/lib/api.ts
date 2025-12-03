/**
 * TotalImage Web API Client
 *
 * Provides typed access to the TotalImage forensic analysis backend
 */

import axios, { type AxiosInstance } from 'axios';

const API_BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:3000';

export interface VaultInfo {
  format: string;
  size: number;
  zones: number;
}

export interface Zone {
  index: number;
  start_sector: number;
  total_sectors: number;
  partition_type: string;
  filesystem: string | null;
}

export interface OccupantInfo {
  name: string;
  is_directory: boolean;
  size: number;
  created: string | null;
  modified: string | null;
  accessed: string | null;
  attributes: number;
}

export interface AnalysisProgress {
  stage: string;
  progress: number;
  total: number;
  message: string;
}

class TotalImageAPI {
  private client: AxiosInstance;

  constructor(baseURL: string = API_BASE_URL) {
    this.client = axios.create({
      baseURL,
      headers: {
        'Content-Type': 'application/json',
      },
      timeout: 30000, // 30 second timeout
    });

    // Response interceptor for error handling
    this.client.interceptors.response.use(
      (response) => response,
      (error) => {
        console.error('API Error:', error.response?.data || error.message);
        return Promise.reject(error);
      }
    );
  }

  /**
   * Health check
   */
  async health(): Promise<{ status: string }> {
    const response = await this.client.get('/health');
    return response.data;
  }

  /**
   * Get vault (disk image) information
   */
  async getVaultInfo(imagePath: string): Promise<VaultInfo> {
    const response = await this.client.get('/api/vault/info', {
      params: { path: imagePath },
    });
    return response.data;
  }

  /**
   * List zones (partitions) in vault
   */
  async getVaultZones(imagePath: string): Promise<Zone[]> {
    const response = await this.client.get('/api/vault/zones', {
      params: { path: imagePath },
    });
    return response.data;
  }

  /**
   * List directory contents
   */
  async listDirectory(
    imagePath: string,
    zoneIndex: number,
    directoryPath: string = '/'
  ): Promise<OccupantInfo[]> {
    const response = await this.client.get('/api/territory/list', {
      params: {
        vault_path: imagePath,
        zone_index: zoneIndex,
        path: directoryPath,
      },
    });
    return response.data;
  }

  /**
   * Extract file from disk image
   */
  async extractFile(
    imagePath: string,
    zoneIndex: number,
    filePath: string
  ): Promise<Blob> {
    const response = await this.client.get('/api/territory/extract', {
      params: {
        vault_path: imagePath,
        zone_index: zoneIndex,
        path: filePath,
      },
      responseType: 'blob',
    });
    return response.data;
  }

  /**
   * Upload disk image (if supported)
   */
  async uploadImage(file: File, onProgress?: (progress: number) => void): Promise<string> {
    const formData = new FormData();
    formData.append('image', file);

    const response = await this.client.post('/api/upload', formData, {
      headers: {
        'Content-Type': 'multipart/form-data',
      },
      onUploadProgress: (progressEvent) => {
        if (onProgress && progressEvent.total) {
          const percentCompleted = Math.round((progressEvent.loaded * 100) / progressEvent.total);
          onProgress(percentCompleted);
        }
      },
    });

    return response.data.path;
  }
}

// Singleton instance
export const api = new TotalImageAPI();
export default api;
