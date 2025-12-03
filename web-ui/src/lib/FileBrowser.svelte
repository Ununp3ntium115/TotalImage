<script lang="ts">
  import { browserState, imageState } from './stores';
  import api, { type OccupantInfo } from './api';
  import {
    selectZone,
    navigateTo,
    setBrowserContents,
    setBrowserLoading,
    setBrowserError,
  } from './stores';

  $: zones = $imageState.zones;
  $: selectedZone = $browserState.selectedZone;
  $: currentPath = $browserState.currentPath;
  $: contents = $browserState.contents;
  $: breadcrumbs = $browserState.breadcrumbs;
  $: loading = $browserState.loading;
  $: error = $browserState.error;

  async function handleZoneSelect(event: Event) {
    const select = event.target as HTMLSelectElement;
    const zoneIndex = parseInt(select.value);
    const zone = zones.find(z => z.index === zoneIndex);

    if (zone) {
      selectZone(zone);
      await loadDirectory('/');
    }
  }

  async function loadDirectory(path: string) {
    if (!$imageState.path || !selectedZone) return;

    setBrowserLoading(true);
    navigateTo(path);

    try {
      const contents = await api.listDirectory(
        $imageState.path,
        selectedZone.index,
        path
      );
      setBrowserContents(contents);
    } catch (err: any) {
      setBrowserError(err.response?.data?.error || err.message);
    }
  }

  async function handleItemClick(item: OccupantInfo) {
    if (item.is_directory) {
      const newPath = currentPath === '/'
        ? `/${item.name}`
        : `${currentPath}/${item.name}`;
      await loadDirectory(newPath);
    } else {
      await downloadFile(item);
    }
  }

  async function downloadFile(item: OccupantInfo) {
    if (!$imageState.path || !selectedZone) return;

    try {
      const filePath = currentPath === '/'
        ? `/${item.name}`
        : `${currentPath}/${item.name}`;

      const blob = await api.extractFile(
        $imageState.path,
        selectedZone.index,
        filePath
      );

      // Create download link
      const url = window.URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = item.name;
      document.body.appendChild(a);
      a.click();
      window.URL.revokeObjectURL(url);
      document.body.removeChild(a);
    } catch (err: any) {
      alert(`Failed to extract file: ${err.message}`);
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i];
  }

  function formatDate(dateStr: string | null): string {
    if (!dateStr) return '-';
    try {
      return new Date(dateStr).toLocaleString();
    } catch {
      return '-';
    }
  }

  function getFileIcon(item: OccupantInfo): string {
    if (item.is_directory) return '📁';
    const name = item.name.toLowerCase();
    if (name.endsWith('.txt')) return '📄';
    if (name.endsWith('.jpg') || name.endsWith('.png')) return '🖼️';
    if (name.endsWith('.pdf')) return '📕';
    if (name.endsWith('.zip') || name.endsWith('.tar')) return '🗜️';
    if (name.endsWith('.exe') || name.endsWith('.dll')) return '⚙️';
    return '📄';
  }
</script>

<div class="file-browser">
  <div class="browser-header">
    <h2>📂 File Browser</h2>
    {#if zones.length > 0}
      <select on:change={handleZoneSelect} value={selectedZone?.index ?? ''}>
        <option value="" disabled>Select a partition...</option>
        {#each zones as zone}
          <option value={zone.index}>
            Zone {zone.index} - {zone.filesystem || 'Unknown'} ({formatBytes(zone.total_sectors * 512)})
          </option>
        {/each}
      </select>
    {/if}
  </div>

  {#if selectedZone}
    <div class="breadcrumb">
      {#each breadcrumbs as crumb, i}
        {#if i > 0}<span class="separator">/</span>{/if}
        <button
          class="crumb"
          on:click={() => loadDirectory(crumb)}
          class:active={crumb === currentPath}
        >
          {crumb === '/' ? 'Root' : crumb.split('/').filter(p => p).pop()}
        </button>
      {/each}
    </div>

    {#if loading}
      <div class="loading">
        <div class="spinner"></div>
        <p>Loading directory...</p>
      </div>
    {:else if error}
      <div class="error">
        <strong>Error:</strong> {error}
      </div>
    {:else if contents.length === 0}
      <div class="empty">
        <p>📭 This directory is empty</p>
      </div>
    {:else}
      <div class="file-list">
        <table>
          <thead>
            <tr>
              <th class="col-icon"></th>
              <th class="col-name">Name</th>
              <th class="col-size">Size</th>
              <th class="col-modified">Modified</th>
            </tr>
          </thead>
          <tbody>
            {#each contents as item}
              <tr
                class="file-row"
                class:directory={item.is_directory}
                on:click={() => handleItemClick(item)}
              >
                <td class="col-icon">{getFileIcon(item)}</td>
                <td class="col-name">{item.name}</td>
                <td class="col-size">
                  {item.is_directory ? '-' : formatBytes(item.size)}
                </td>
                <td class="col-modified">{formatDate(item.modified)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {:else if zones.length > 0}
    <div class="empty">
      <p>☝️ Select a partition above to browse files</p>
    </div>
  {:else}
    <div class="empty">
      <p>📀 Load a disk image first</p>
    </div>
  {/if}
</div>

<style>
  .file-browser {
    background: white;
    border-radius: 8px;
    padding: 2rem;
    box-shadow: 0 2px 8px rgba(0,0,0,0.1);
    margin-top: 2rem;
  }

  .browser-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.5rem;
  }

  .browser-header h2 {
    margin: 0;
    color: #333;
  }

  select {
    padding: 0.5rem 1rem;
    border: 2px solid #e0e0e0;
    border-radius: 4px;
    font-size: 0.95rem;
    background: white;
    cursor: pointer;
  }

  select:focus {
    outline: none;
    border-color: #4CAF50;
  }

  .breadcrumb {
    display: flex;
    align-items: center;
    margin-bottom: 1rem;
    padding: 0.75rem;
    background: #f5f5f5;
    border-radius: 4px;
    flex-wrap: wrap;
  }

  .crumb {
    background: none;
    border: none;
    color: #666;
    cursor: pointer;
    padding: 0.25rem 0.5rem;
    border-radius: 3px;
    transition: all 0.2s;
  }

  .crumb:hover {
    background: #e0e0e0;
    color: #333;
  }

  .crumb.active {
    color: #4CAF50;
    font-weight: bold;
  }

  .separator {
    color: #999;
    margin: 0 0.25rem;
  }

  .loading, .empty {
    text-align: center;
    padding: 3rem;
    color: #666;
  }

  .spinner {
    border: 4px solid #f3f3f3;
    border-top: 4px solid #4CAF50;
    border-radius: 50%;
    width: 40px;
    height: 40px;
    animation: spin 1s linear infinite;
    margin: 0 auto 1rem;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .error {
    padding: 1rem;
    background: #ffebee;
    color: #c62828;
    border-radius: 4px;
    border-left: 4px solid #c62828;
  }

  .file-list {
    overflow-x: auto;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  thead {
    background: #f5f5f5;
  }

  th {
    text-align: left;
    padding: 0.75rem 1rem;
    font-weight: 600;
    color: #666;
    border-bottom: 2px solid #e0e0e0;
  }

  .col-icon {
    width: 40px;
    text-align: center;
  }

  .col-size {
    width: 120px;
  }

  .col-modified {
    width: 200px;
  }

  .file-row {
    cursor: pointer;
    transition: background 0.2s;
  }

  .file-row:hover {
    background: #f9f9f9;
  }

  .file-row.directory {
    font-weight: 500;
  }

  td {
    padding: 0.75rem 1rem;
    border-bottom: 1px solid #f0f0f0;
  }

  .col-icon {
    font-size: 1.25rem;
  }
</style>
