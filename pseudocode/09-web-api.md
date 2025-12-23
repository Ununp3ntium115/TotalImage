# Web API Module - Pseudocode Documentation

**Component:** `totalimage-web`  
**Location:** `crates/totalimage-web/src/`  
**Purpose:** REST API server for web-based disk image analysis

---

## Table of Contents

1. [Overview](#overview)
2. [API Endpoints](#api-endpoints)
3. [Request/Response Formats](#requestresponse-formats)
4. [Code References](#code-references)

---

## Overview

The web API provides REST endpoints for:
- Vault information
- Zone enumeration
- File listing
- File extraction
- Health checks

**Code Reference:** `crates/totalimage-web/src/lib.rs`

---

## API Endpoints

### Health Check

```pseudocode
ENDPOINT: GET /health

FUNCTION health_handler() -> Response:
    RETURN Response {
        status_code: 200,
        body: {
            status: "healthy",
            version: API_VERSION
        }
    }
END FUNCTION
```

### Vault Info

```pseudocode
ENDPOINT: GET /api/vault/info?path={path}

FUNCTION vault_info_handler(path: STRING) -> Response:
    // Validate path
    validated_path = validate_file_path(path)
    
    // Open vault
    vault = open_vault(validated_path, VaultConfig::default())
    
    RETURN Response {
        status_code: 200,
        body: {
            type: vault.identify(),
            size: vault.length()
        }
    }
END FUNCTION
```

### List Zones

```pseudocode
ENDPOINT: GET /api/vault/zones?path={path}

FUNCTION list_zones_handler(path: STRING) -> Response:
    validated_path = validate_file_path(path)
    vault = open_vault(validated_path, VaultConfig::default())
    vault_content = vault.content()
    zone_table = detect_zone_table(vault_content)
    
    zones = IF zone_table IS NOT NULL:
        zone_table.enumerate_zones()
    ELSE:
        []
    
    RETURN Response {
        status_code: 200,
        body: {
            zones: zones.map(zone => {
                index: zone.index,
                type: zone.zone_type,
                offset: zone.offset,
                length: zone.length
            })
        }
    }
END FUNCTION
```

---

## Code References

### File Structure

```
crates/totalimage-web/src/
├── lib.rs              # Module exports
└── server.rs           # Axum server implementation
```

---

## Cross-References

- **MCP Server:** See [07-mcp-server.md](07-mcp-server.md) (similar functionality)
- **Fire Marshal:** See [08-fire-marshal.md](08-fire-marshal.md) (orchestration)

---

**Document Version:** 1.0.0  
**Last Updated:** December 21, 2025  
**Next:** [10-cli.md](10-cli.md) - CLI Interface
