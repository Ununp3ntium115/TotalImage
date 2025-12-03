/**
 * Svelte stores for application state management
 */

import { writable, derived } from 'svelte/store';
import type { VaultInfo, Zone, OccupantInfo } from './api';

export interface ImageState {
  path: string | null;
  info: VaultInfo | null;
  zones: Zone[];
  loading: boolean;
  error: string | null;
}

export interface BrowserState {
  selectedZone: Zone | null;
  currentPath: string;
  contents: OccupantInfo[];
  breadcrumbs: string[];
  loading: boolean;
  error: string | null;
}

export interface UploadState {
  uploading: boolean;
  progress: number;
  filename: string | null;
}

// Image/Vault state
export const imageState = writable<ImageState>({
  path: null,
  info: null,
  zones: [],
  loading: false,
  error: null,
});

// File browser state
export const browserState = writable<BrowserState>({
  selectedZone: null,
  currentPath: '/',
  contents: [],
  breadcrumbs: ['/'],
  loading: false,
  error: null,
});

// Upload state
export const uploadState = writable<UploadState>({
  uploading: false,
  progress: 0,
  filename: null,
});

// Derived store: is an image loaded?
export const hasImage = derived(
  imageState,
  ($imageState) => $imageState.path !== null && $imageState.info !== null
);

// Derived store: current zone info
export const currentZone = derived(
  browserState,
  ($browserState) => $browserState.selectedZone
);

// Helper functions for store updates

export function setImagePath(path: string) {
  imageState.update(state => ({
    ...state,
    path,
    loading: true,
    error: null,
  }));
}

export function setImageInfo(info: VaultInfo, zones: Zone[]) {
  imageState.update(state => ({
    ...state,
    info,
    zones,
    loading: false,
    error: null,
  }));
}

export function setImageError(error: string) {
  imageState.update(state => ({
    ...state,
    loading: false,
    error,
  }));
}

export function selectZone(zone: Zone) {
  browserState.update(state => ({
    ...state,
    selectedZone: zone,
    currentPath: '/',
    breadcrumbs: ['/'],
  }));
}

export function navigateTo(path: string) {
  const parts = path.split('/').filter(p => p.length > 0);
  const breadcrumbs = ['/'];

  for (let i = 0; i < parts.length; i++) {
    breadcrumbs.push('/' + parts.slice(0, i + 1).join('/'));
  }

  browserState.update(state => ({
    ...state,
    currentPath: path,
    breadcrumbs,
  }));
}

export function setBrowserContents(contents: OccupantInfo[]) {
  browserState.update(state => ({
    ...state,
    contents,
    loading: false,
    error: null,
  }));
}

export function setBrowserLoading(loading: boolean) {
  browserState.update(state => ({
    ...state,
    loading,
  }));
}

export function setBrowserError(error: string) {
  browserState.update(state => ({
    ...state,
    loading: false,
    error,
  }));
}

export function setUploadProgress(progress: number, filename: string) {
  uploadState.set({
    uploading: true,
    progress,
    filename,
  });
}

export function clearUpload() {
  uploadState.set({
    uploading: false,
    progress: 0,
    filename: null,
  });
}
