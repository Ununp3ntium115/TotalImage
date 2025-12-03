# TotalImage Web UI

Modern web interface for TotalImage forensic disk image analysis, built with Svelte and TypeScript.

## Features

- 🔍 **Image Loading** - Load VHD, E01, AFF4, and raw disk images
- 📂 **File Browser** - Navigate filesystem hierarchies with tree view
- 💾 **File Extraction** - Download files directly from disk images
- 🗂️ **Multi-Format Support** - FAT, exFAT, NTFS, ISO-9660 filesystems
- ⚡ **Real-Time Analysis** - Live progress updates during analysis
- 🎨 **Modern UI** - Clean, responsive interface with gradient design

## Quick Start

### Prerequisites

- Node.js 18+ and npm
- TotalImage backend API running (default: http://localhost:3000)

### Installation

```bash
# Install dependencies
npm install

# Create environment configuration
cp .env.example .env

# Start development server
npm run dev
```

The web UI will be available at http://localhost:5173

### Build for Production

```bash
# Build optimized production bundle
npm run build

# Preview production build
npm run preview
```

## Configuration

Edit `.env` to configure the backend API URL:

```
VITE_API_URL=http://localhost:3000
```

## Architecture

### Components

- **Dashboard.svelte** - Main dashboard for loading disk images and displaying metadata
- **FileBrowser.svelte** - File tree browser with directory navigation
- **api.ts** - TypeScript API client for backend communication
- **stores.ts** - Svelte stores for state management

### State Management

Uses Svelte's built-in reactive stores:

- `imageState` - Currently loaded disk image metadata
- `browserState` - File browser navigation and contents
- `uploadState` - File upload progress tracking

### API Integration

The UI communicates with the TotalImage Web API (`crates/totalimage-web`) using:

- `GET /health` - Health check
- `GET /api/vault/info` - Get disk image metadata
- `GET /api/vault/zones` - List partitions
- `GET /api/territory/list` - List directory contents
- `GET /api/territory/extract` - Extract file data

## Usage

### Loading a Disk Image

1. Enter the full path to your disk image (e.g., `/path/to/image.vhd`)
2. Click "Load Image"
3. View partition information and filesystem details

### Browsing Files

1. Select a partition from the dropdown
2. Navigate directories by clicking on folder names
3. Use breadcrumbs to navigate back up the tree

### Extracting Files

Click on any file to download it to your local system. Files are extracted directly from the disk image without mounting.

## Development

### Project Structure

```
web-ui/
├── src/
│   ├── lib/
│   │   ├── Dashboard.svelte    # Main dashboard component
│   │   ├── FileBrowser.svelte  # File browser component
│   │   ├── api.ts              # API client
│   │   └── stores.ts           # State management
│   ├── App.svelte              # Root component
│   ├── main.ts                 # Entry point
│   └── app.css                 # Global styles
├── public/                     # Static assets
├── .env                        # Environment configuration
└── package.json                # Dependencies
```

### Tech Stack

- **Svelte 5** - Reactive UI framework
- **Vite** - Build tool and dev server
- **TypeScript** - Type-safe development
- **Axios** - HTTP client for API calls

### Type Safety

All components use TypeScript with strict type checking. API responses are fully typed for compile-time safety.

## Browser Support

- Chrome/Edge 90+
- Firefox 88+
- Safari 14+

## License

GPL-3.0-or-later - see LICENSE file

## Links

- [TotalImage Backend](../crates/totalimage-web/)
- [Main Repository](https://github.com/Ununp3ntium115/TotalImage)
- [Documentation](../steering/)
