<script lang="ts">
  import { imageState, hasImage } from './stores';
  import api from './api';
  import { setImagePath, setImageInfo, setImageError } from './stores';

  let imagePath = '';
  let fileInput: HTMLInputElement;

  $: loaded = $hasImage;
  $: loading = $imageState.loading;
  $: error = $imageState.error;
  $: info = $imageState.info;
  $: zones = $imageState.zones;

  async function loadImage() {
    if (!imagePath.trim()) {
      return;
    }

    setImagePath(imagePath);

    try {
      const [vaultInfo, vaultZones] = await Promise.all([
        api.getVaultInfo(imagePath),
        api.getVaultZones(imagePath),
      ]);

      setImageInfo(vaultInfo, vaultZones);
    } catch (err: any) {
      setImageError(err.response?.data?.error || err.message || 'Failed to load image');
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i];
  }

  function formatSectorSize(sectors: number): string {
    const bytes = sectors * 512; // Assuming 512-byte sectors
    return formatBytes(bytes);
  }
</script>

<div class="dashboard">
  <header class="header">
    <h1>🔍 TotalImage</h1>
    <p class="subtitle">Forensic Disk Image Analysis</p>
  </header>

  <div class="load-section">
    <h2>Load Disk Image</h2>
    <div class="input-group">
      <input
        type="text"
        bind:value={imagePath}
        placeholder="/path/to/image.vhd or /path/to/image.e01"
        disabled={loading}
        on:keypress={(e) => e.key === 'Enter' && loadImage()}
      />
      <button on:click={loadImage} disabled={loading || !imagePath.trim()}>
        {loading ? 'Loading...' : 'Load Image'}
      </button>
    </div>

    {#if error}
      <div class="error">
        <strong>Error:</strong> {error}
      </div>
    {/if}
  </div>

  {#if loaded && info}
    <div class="info-section">
      <h2>Image Information</h2>
      <div class="info-grid">
        <div class="info-card">
          <div class="info-label">Format</div>
          <div class="info-value">{info.format}</div>
        </div>
        <div class="info-card">
          <div class="info-label">Size</div>
          <div class="info-value">{formatBytes(info.size)}</div>
        </div>
        <div class="info-card">
          <div class="info-label">Partitions</div>
          <div class="info-value">{info.zones}</div>
        </div>
      </div>

      <h3>Partitions / Zones</h3>
      <div class="zones-list">
        {#each zones as zone}
          <div class="zone-card">
            <div class="zone-header">
              <span class="zone-index">Zone {zone.index}</span>
              <span class="zone-type">{zone.partition_type}</span>
            </div>
            <div class="zone-details">
              <div class="detail">
                <span class="detail-label">Start:</span>
                <span class="detail-value">Sector {zone.start_sector.toLocaleString()}</span>
              </div>
              <div class="detail">
                <span class="detail-label">Size:</span>
                <span class="detail-value">{formatSectorSize(zone.total_sectors)}</span>
              </div>
              <div class="detail">
                <span class="detail-label">Filesystem:</span>
                <span class="detail-value">{zone.filesystem || 'Unknown'}</span>
              </div>
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .dashboard {
    max-width: 1200px;
    margin: 0 auto;
    padding: 2rem;
  }

  .header {
    text-align: center;
    margin-bottom: 3rem;
  }

  .header h1 {
    font-size: 2.5rem;
    margin: 0;
    color: #1a1a1a;
  }

  .subtitle {
    color: #666;
    margin-top: 0.5rem;
  }

  .load-section {
    background: white;
    border-radius: 8px;
    padding: 2rem;
    box-shadow: 0 2px 8px rgba(0,0,0,0.1);
    margin-bottom: 2rem;
  }

  .load-section h2 {
    margin-top: 0;
    color: #333;
  }

  .input-group {
    display: flex;
    gap: 1rem;
    margin-top: 1rem;
  }

  input[type="text"] {
    flex: 1;
    padding: 0.75rem 1rem;
    border: 2px solid #e0e0e0;
    border-radius: 4px;
    font-size: 1rem;
  }

  input[type="text"]:focus {
    outline: none;
    border-color: #4CAF50;
  }

  button {
    padding: 0.75rem 2rem;
    background: #4CAF50;
    color: white;
    border: none;
    border-radius: 4px;
    font-size: 1rem;
    cursor: pointer;
    transition: background 0.2s;
  }

  button:hover:not(:disabled) {
    background: #45a049;
  }

  button:disabled {
    background: #ccc;
    cursor: not-allowed;
  }

  .error {
    margin-top: 1rem;
    padding: 1rem;
    background: #ffebee;
    color: #c62828;
    border-radius: 4px;
    border-left: 4px solid #c62828;
  }

  .info-section {
    background: white;
    border-radius: 8px;
    padding: 2rem;
    box-shadow: 0 2px 8px rgba(0,0,0,0.1);
  }

  .info-section h2 {
    margin-top: 0;
    color: #333;
  }

  .info-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1rem;
    margin: 1.5rem 0;
  }

  .info-card {
    background: #f5f5f5;
    padding: 1.5rem;
    border-radius: 6px;
    text-align: center;
  }

  .info-label {
    font-size: 0.875rem;
    color: #666;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .info-value {
    font-size: 1.5rem;
    font-weight: bold;
    color: #333;
    margin-top: 0.5rem;
  }

  .zones-list {
    display: grid;
    gap: 1rem;
    margin-top: 1rem;
  }

  .zone-card {
    border: 2px solid #e0e0e0;
    border-radius: 6px;
    padding: 1rem;
    transition: border-color 0.2s;
  }

  .zone-card:hover {
    border-color: #4CAF50;
  }

  .zone-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }

  .zone-index {
    font-weight: bold;
    font-size: 1.1rem;
    color: #333;
  }

  .zone-type {
    background: #4CAF50;
    color: white;
    padding: 0.25rem 0.75rem;
    border-radius: 12px;
    font-size: 0.875rem;
  }

  .zone-details {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 0.75rem;
  }

  .detail {
    display: flex;
    justify-content: space-between;
    padding: 0.5rem;
    background: #f9f9f9;
    border-radius: 4px;
  }

  .detail-label {
    color: #666;
    font-weight: 500;
  }

  .detail-value {
    color: #333;
    font-weight: 600;
  }

  h3 {
    color: #333;
    margin-top: 2rem;
    margin-bottom: 1rem;
  }
</style>
