# FRONT COLLECTIVE: User Interface Cells

**Codename:** Public Liberation Interface
**Purpose:** Provide autonomous user interaction with liberated data
**Current State:** C# Windows Forms (.NET 8.0)
**Target State:** Svelte Web Components + Rust Web API

---

## Overview

The Front Collective provides the public-facing interface for interacting with vaults, zones, and territories. It transitions from a desktop-bound Windows Forms application to a web-based autonomous interface.

---

## Current Architecture (Windows Forms)

### Main Window: `frmMain.cs`
**Brand Name:** `CommandCenter`
**Lines:** 2,689
**Purpose:** Primary interface hub

#### Components

**TreeView Panel (Left):**
- Container/Partition selection
- Directory tree navigation
- Hierarchical file system view

**ListView Panel (Right):**
- File/directory listing
- Multiple view modes (Details, List, Icons)
- Sort by name, size, date
- Drag-and-drop support

**Menu System:**
- File operations (Open, Extract, Export)
- View options (Refresh, Properties)
- Tools (Defragment, Undelete, Boot Sector)
- Help (About, Documentation)

**Status Bar:**
- Current vault info
- Territory statistics
- Selection status

#### Actions (Menu Items)

| Menu | Action | Brand Name | Target Svelte Component |
|------|--------|------------|------------------------|
| File → Open | Open vault | `infiltrate_vault()` | `VaultOpener` |
| File → Extract Files | Extract data | `liberate_files()` | `FileExtractor` |
| File → Export Image | Export vault | `export_vault()` | `VaultExporter` |
| View → Refresh | Reload view | `refresh_view()` | Reactive store update |
| Tools → Boot Sector | View manifesto | `inspect_manifesto()` | `ManifestoViewer` |
| Tools → Image Info | View vault props | `vault_info()` | `VaultInfoPanel` |
| Tools → Hex Viewer | View raw data | `raw_inspection()` | `HexViewer` |

---

## Dialog Forms (20 Total)

### Core Dialogs

| Dialog | Brand Name | Purpose | Svelte Equivalent |
|--------|------------|---------|-------------------|
| `dlgAbout` | AboutPanel | Show application info | Modal component |
| `dlgBootSector` | ManifestoInspector | View/edit boot sector | `<ManifestoEditor>` |
| `dlgChangeVolLabel` | BannerEditor | Change volume label | `<BannerInput>` |
| `dlgDefragment` | TerritoryOptimizer | Defragment territory | `<DefragmentWizard>` |
| `dlgExtract` | LiberationWizard | Extract files wizard | `<ExtractionWizard>` |
| `dlgHexView` | RawInspector | Hex editor | `<HexViewer>` |
| `dlgImageInfo` | VaultInspector | Image properties | `<VaultProperties>` |
| `dlgNewImage` | VaultManufacturer | Create new vault | `<VaultCreator>` |
| `dlgNewImageAdvanced` | AdvancedManufacturer | Advanced vault creation | `<AdvancedVaultCreator>` |
| `dlgNewFolder` | DirectoryCreator | Create directory | `<NewFolderDialog>` |
| `dlgNotifications` | MessageCenter | Notification center | `<NotificationPanel>` |
| `dlgSelectPartition` | ZoneSelector | Select partition | `<PartitionPicker>` |
| `dlgSettings` | ConfigurationPanel | Application settings | `<SettingsPanel>` |
| `dlgUndelete` | RecoveryPanel | Undelete files | `<FileRecovery>` |

---

## Svelte Architecture Design

### Component Tree

```
App.svelte (Root)
├── Header.svelte
│   ├── MenuBar.svelte
│   └── ToolBar.svelte
│
├── MainView.svelte
│   ├── LeftPanel.svelte
│   │   ├── VaultTreeView.svelte
│   │   └── DirectoryTreeView.svelte
│   │
│   └── RightPanel.svelte
│       ├── FileListView.svelte
│       └── FileGridView.svelte
│
├── StatusBar.svelte
│
└── Modals
    ├── VaultOpener.svelte
    ├── ExtractionWizard.svelte
    ├── VaultProperties.svelte
    ├── ManifestoEditor.svelte
    ├── HexViewer.svelte
    └── SettingsPanel.svelte
```

### Svelte Store Architecture

```typescript
// stores/vault.ts
export const currentVault = writable<Vault | null>(null);
export const vaultInfo = derived(currentVault, $vault => {
    if (!$vault) return null;
    return {
        name: $vault.identify(),
        size: $vault.length(),
        // ...
    };
});

// stores/territory.ts
export const currentTerritory = writable<Territory | null>(null);
export const currentDirectory = writable<DirectoryCell | null>(null);
export const fileList = writable<OccupantInfo[]>([]);

// stores/selection.ts
export const selectedFiles = writable<Set<string>>(new Set());
export const selectionStats = derived(selectedFiles, $selected => ({
    count: $selected.size,
    totalSize: 0, // Calculate from fileList
}));

// stores/zones.ts
export const zoneTable = writable<ZoneTable | null>(null);
export const zones = derived(zoneTable, $table =>
    $table ? $table.enumerate_zones() : []
);
export const activeZone = writable<Zone | null>(null);
```

---

## Rust Web API Layer

### API Endpoints

```rust
// Web API using axum framework

use axum::{
    Router,
    routing::{get, post},
    extract::{Path, State, Multipart},
    response::Json,
    http::StatusCode,
};

// VAULT OPERATIONS
POST   /api/vault/open              // Open vault from file
GET    /api/vault/{id}/info         // Get vault information
GET    /api/vault/{id}/hash/md5     // Calculate MD5
GET    /api/vault/{id}/hash/sha1    // Calculate SHA1
POST   /api/vault/{id}/export       // Export vault
DELETE /api/vault/{id}/close        // Close vault

// ZONE OPERATIONS
GET    /api/vault/{id}/zones        // List all zones
GET    /api/vault/{id}/zones/{idx}  // Get specific zone
POST   /api/vault/{id}/zones/{idx}/select  // Select zone

// TERRITORY OPERATIONS
GET    /api/territory/info          // Current territory info
GET    /api/territory/root          // Get root directory
GET    /api/territory/dir/{path}    // List directory contents
GET    /api/territory/file/{path}   // Get file info
POST   /api/territory/file/{path}/extract  // Extract file

// FILE OPERATIONS
GET    /api/file/download/{path}    // Download file
POST   /api/file/batch-extract      // Extract multiple files
GET    /api/file/hex/{path}         // Get hex view

// MANIFESTO OPERATIONS
GET    /api/manifesto/boot-sector   // Get boot sector
POST   /api/manifesto/boot-sector   // Update boot sector (future)

// SESSION MANAGEMENT
GET    /api/session                 // Current session info
POST   /api/session/clear           // Clear session
```

### API Types (Rust)

```rust
// API REQUEST/RESPONSE TYPES

#[derive(Serialize, Deserialize)]
pub struct VaultInfo {
    pub id: String,
    pub vault_type: String,
    pub size: u64,
    pub md5: Option<String>,
    pub sha1: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ZoneInfo {
    pub index: usize,
    pub zone_type: String,
    pub offset: u64,
    pub length: u64,
}

#[derive(Serialize, Deserialize)]
pub struct TerritoryInfo {
    pub territory_type: String,
    pub banner: String,
    pub domain_size: u64,
    pub liberated_space: u64,
    pub block_size: u64,
    pub hierarchical: bool,
}

#[derive(Serialize, Deserialize)]
pub struct OccupantInfo {
    pub name: String,
    pub is_directory: bool,
    pub size: u64,
    pub created: Option<String>,  // ISO 8601
    pub modified: Option<String>, // ISO 8601
    pub attributes: u32,
}

#[derive(Serialize, Deserialize)]
pub struct DirectoryListing {
    pub path: String,
    pub occupants: Vec<OccupantInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct ExtractionRequest {
    pub files: Vec<String>,
    pub destination: String,
}

#[derive(Serialize, Deserialize)]
pub struct HexViewResponse {
    pub offset: u64,
    pub data: String,  // Hex string
    pub length: u64,
}
```

### Server Implementation Sketch

```rust
use axum::{Router, Extension};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState {
    pub vault_registry: Arc<RwLock<VaultRegistry>>,
    pub session: Arc<RwLock<SessionState>>,
}

pub struct VaultRegistry {
    vaults: HashMap<String, Box<dyn Vault>>,
    next_id: u64,
}

pub struct SessionState {
    current_vault_id: Option<String>,
    current_zone: Option<usize>,
    current_territory: Option<Box<dyn Territory>>,
    current_path: String,
}

#[tokio::main]
async fn main() {
    let state = AppState {
        vault_registry: Arc::new(RwLock::new(VaultRegistry::new())),
        session: Arc::new(RwLock::new(SessionState::default())),
    };

    let app = Router::new()
        // VAULT ROUTES
        .route("/api/vault/open", post(open_vault))
        .route("/api/vault/:id/info", get(get_vault_info))
        .route("/api/vault/:id/zones", get(list_zones))

        // TERRITORY ROUTES
        .route("/api/territory/info", get(get_territory_info))
        .route("/api/territory/dir/:path", get(list_directory))
        .route("/api/territory/file/:path/extract", post(extract_file))

        // STATIC ASSETS (Svelte build)
        .route("/", get(serve_index))
        .nest_service("/assets", tower_http::services::ServeDir::new("./dist"))

        .layer(Extension(state));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}

// HANDLER EXAMPLES

async fn open_vault(
    State(state): State<Arc<AppState>>,
    multipart: Multipart,
) -> Result<Json<VaultInfo>, StatusCode> {
    // Parse uploaded file
    // Detect vault type
    // Register in vault_registry
    // Return VaultInfo
    todo!()
}

async fn list_directory(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Result<Json<DirectoryListing>, StatusCode> {
    let session = state.session.read().await;
    let territory = session.current_territory
        .as_ref()
        .ok_or(StatusCode::BAD_REQUEST)?;

    let dir = territory.navigate_to(&path)?;
    let occupants = dir.list_occupants()?;

    Ok(Json(DirectoryListing {
        path,
        occupants,
    }))
}

async fn extract_file(
    State(state): State<Arc<AppState>>,
    Path(file_path): Path<String>,
) -> Result<Vec<u8>, StatusCode> {
    let session = state.session.read().await;
    let territory = session.current_territory
        .as_ref()
        .ok_or(StatusCode::BAD_REQUEST)?;

    let data = territory.extract_file(&file_path)?;
    Ok(data)
}
```

---

## Svelte Component Examples

### VaultTreeView.svelte

```svelte
<script lang="ts">
  import { currentVault, zoneTable, zones } from '../stores';
  import { onMount } from 'svelte';

  let selectedZone: number | null = null;

  async function selectZone(idx: number) {
    selectedZone = idx;
    // Call API to select zone and load territory
    const response = await fetch(`/api/vault/${$currentVault.id}/zones/${idx}/select`, {
      method: 'POST'
    });
    // Update stores
  }
</script>

<div class="vault-tree">
  {#if $currentVault}
    <div class="vault-header">
      <h3>{$currentVault.identify()}</h3>
      <span>{formatBytes($currentVault.length())}</span>
    </div>

    {#if $zones.length > 0}
      <div class="zones">
        <h4>Zones ({$zones.length})</h4>
        {#each $zones as zone, idx}
          <div
            class="zone-item"
            class:active={selectedZone === idx}
            on:click={() => selectZone(idx)}
          >
            <span class="zone-type">{zone.zone_type}</span>
            <span class="zone-size">{formatBytes(zone.length)}</span>
          </div>
        {/each}
      </div>
    {:else}
      <p class="no-zones">Direct Territory (No Zones)</p>
    {/if}
  {:else}
    <p class="placeholder">No vault loaded</p>
  {/if}
</div>

<style>
  .vault-tree {
    background: var(--panel-bg);
    padding: 1rem;
    overflow-y: auto;
  }

  .zone-item {
    padding: 0.5rem;
    cursor: pointer;
    border-left: 3px solid transparent;
  }

  .zone-item:hover {
    background: var(--hover-bg);
  }

  .zone-item.active {
    border-left-color: var(--accent);
    background: var(--active-bg);
  }
</style>
```

### FileListView.svelte

```svelte
<script lang="ts">
  import { fileList, selectedFiles, currentDirectory } from '../stores';
  import { formatBytes, formatDate } from '../utils';

  function toggleSelection(name: string) {
    selectedFiles.update(set => {
      if (set.has(name)) {
        set.delete(name);
      } else {
        set.add(name);
      }
      return set;
    });
  }

  async function navigateToDirectory(name: string) {
    const response = await fetch(`/api/territory/dir/${name}`);
    const listing = await response.json();
    fileList.set(listing.occupants);
  }
</script>

<div class="file-list">
  <table>
    <thead>
      <tr>
        <th>Name</th>
        <th>Size</th>
        <th>Modified</th>
        <th>Attributes</th>
      </tr>
    </thead>
    <tbody>
      {#each $fileList as occupant}
        <tr
          class:selected={$selectedFiles.has(occupant.name)}
          class:directory={occupant.is_directory}
          on:click={() => toggleSelection(occupant.name)}
          on:dblclick={() => {
            if (occupant.is_directory) {
              navigateToDirectory(occupant.name);
            }
          }}
        >
          <td>
            {#if occupant.is_directory}
              📁 {occupant.name}
            {:else}
              📄 {occupant.name}
            {/if}
          </td>
          <td>{formatBytes(occupant.size)}</td>
          <td>{formatDate(occupant.modified)}</td>
          <td>{formatAttributes(occupant.attributes)}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  .file-list {
    flex: 1;
    overflow-y: auto;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  tr {
    cursor: pointer;
  }

  tr:hover {
    background: var(--hover-bg);
  }

  tr.selected {
    background: var(--selected-bg);
  }

  tr.directory {
    font-weight: bold;
  }
</style>
```

---

## Technology Stack

### Frontend (Svelte)
```json
{
  "dependencies": {
    "svelte": "^4.0.0",
    "svelte-routing": "^2.0.0",
    "axios": "^1.6.0"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^3.0.0",
    "typescript": "^5.0.0",
    "vite": "^5.0.0"
  }
}
```

### Backend (Rust)
```toml
[dependencies]
# WEB FRAMEWORK
axum = { version = "0.7", features = ["multipart"] }
tokio = { version = "1", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["fs", "cors"] }

# SERIALIZATION
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# ASYNC
async-trait = "0.1"

# VAULT/TERRITORY CELLS (our implementation)
totalimage-vaults = { path = "../vaults" }
totalimage-territories = { path = "../territories" }
totalimage-zones = { path = "../zones" }
```

---

## Deployment Architecture

```
┌─────────────────────────────────────┐
│  BROWSER (Autonomous Interface)     │
│  Svelte SPA                          │
└──────────────┬──────────────────────┘
               │ HTTP/WebSocket
               ↓
┌─────────────────────────────────────┐
│  RUST WEB SERVER                    │
│  axum + tower                        │
│  ┌─────────────────────────────────┐│
│  │ API Routes                      ││
│  │ - Vault Operations              ││
│  │ - Territory Navigation          ││
│  │ - File Liberation               ││
│  └─────────────────────────────────┘│
└──────────────┬──────────────────────┘
               │
               ↓
┌─────────────────────────────────────┐
│  ARSENAL (Core Libraries)           │
│  ├─ Vaults                          │
│  ├─ Territories                     │
│  ├─ Zones                           │
│  └─ Pipelines                       │
└─────────────────────────────────────┘
               │
               ↓
┌─────────────────────────────────────┐
│  LOCAL FILE SYSTEM / STORAGE        │
│  Disk images, metadata cache        │
└─────────────────────────────────────┘
```

---

## Status

- ✅ Windows Forms architecture documented
- ✅ Svelte component tree designed
- ✅ Web API routes specified
- ✅ Store architecture designed
- ⏳ Component implementation pending
- ⏳ API implementation pending
- ⏳ Integration testing pending

**Next Action:** Infrastructure specifications for Rust crate structure and redb schemas
