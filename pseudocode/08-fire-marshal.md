# Fire Marshal API - Comprehensive Pseudocode Specification

**Version:** 1.0.0  
**Date:** December 21, 2025  
**Status:** Implementation Guide  
**Purpose:** Complete pseudocode specification for Fire Marshal API implementation following industry best practices

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [API Design Principles](#api-design-principles)
3. [Authentication & Authorization](#authentication--authorization)
4. [Rate Limiting Strategy](#rate-limiting-strategy)
5. [Error Handling Framework](#error-handling-framework)
6. [API Versioning](#api-versioning)
7. [Request/Response Formats](#requestresponse-formats)
8. [Endpoint Specifications](#endpoint-specifications)
9. [WebSocket Support](#websocket-support)
10. [Caching Strategy](#caching-strategy)
11. [Health Checks & Monitoring](#health-checks--monitoring)
12. [Security Best Practices](#security-best-practices)
13. [Performance Optimization](#performance-optimization)
14. [Implementation Pseudocode](#implementation-pseudocode)

---

## Architecture Overview

### System Components

```
┌─────────────────────────────────────────────────────────────┐
│                    Fire Marshal API Layer                    │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   HTTP API   │  │  WebSocket   │  │   Metrics    │     │
│  │   (REST)     │  │   (Real-time)│  │   (Prometheus)│     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│         │                  │                  │              │
│         └──────────────────┼──────────────────┘            │
│                            │                                 │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              Request Processing Pipeline              │   │
│  │  1. Authentication Middleware                        │   │
│  │  2. Rate Limiting Middleware                         │   │
│  │  3. Request Validation                               │   │
│  │  4. Tool Registry Lookup                             │   │
│  │  5. Tool Execution (via Transport)                   │   │
│  │  6. Response Caching                                 │   │
│  │  7. Metrics Collection                               │   │
│  │  8. Error Handling                                   │   │
│  └──────────────────────────────────────────────────────┘   │
│                            │                                 │
│         ┌──────────────────┼──────────────────┐              │
│         │                  │                  │              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   Tool       │  │  Platform    │  │  Execution   │     │
│  │   Registry   │  │  Database    │  │   Log        │     │
│  │   (in-memory)│  │  (redb)      │  │   (redb)     │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## API Design Principles

### RESTful Design

```
PRINCIPLE: Follow REST conventions strictly
- Use HTTP methods correctly (GET, POST, PUT, DELETE, PATCH)
- Use proper HTTP status codes
- Use nouns for resources, not verbs
- Use plural nouns for collections
- Use hierarchical URLs for relationships
- Use query parameters for filtering, pagination, sorting
- Use request/response bodies for complex data
- Support JSON as primary format
- Support Content-Type and Accept headers
```

### URL Structure

```
BASE_URL: https://api.pyro-platform.local/v1

RESOURCE PATTERNS:
- /v1/tools                    (collection)
- /v1/tools/{tool_name}        (resource)
- /v1/tools/{tool_name}/methods (sub-resource)
- /v1/tools/{tool_name}/call    (action endpoint)
- /v1/health                   (system endpoint)
- /v1/metrics                  (system endpoint)
- /v1/stats                    (system endpoint)
```

### HTTP Methods Usage

```
GET     - Retrieve resources (idempotent, safe)
POST    - Create resources or execute actions (not idempotent)
PUT     - Replace entire resource (idempotent)
PATCH   - Partial update (idempotent)
DELETE  - Remove resource (idempotent)
HEAD    - Get headers only (idempotent, safe)
OPTIONS - Get allowed methods (idempotent, safe)
```

---

## Authentication & Authorization

### Authentication Methods

```
SUPPORTED_METHODS:
1. API Key (Header: X-API-Key)
2. JWT Bearer Token (Header: Authorization: Bearer <token>)
3. OAuth2 Client Credentials (Header: Authorization: Bearer <access_token>)
4. Mutual TLS (mTLS) for service-to-service

PRIORITY_ORDER:
1. Check API Key first (fastest)
2. Check JWT Bearer Token
3. Check OAuth2 Token
4. Check mTLS certificate
5. Reject if none valid
```

### Authentication Pseudocode

```pseudocode
FUNCTION authenticate_request(request):
    // Extract authentication credentials
    api_key = request.headers.get("X-API-Key")
    auth_header = request.headers.get("Authorization")
    
    // Try API Key authentication
    IF api_key IS NOT NULL:
        api_key_record = database.get_api_key(api_key)
        IF api_key_record IS NULL:
            RETURN error("Invalid API key", 401)
        IF api_key_record.expired:
            RETURN error("API key expired", 401)
        IF api_key_record.revoked:
            RETURN error("API key revoked", 401)
        
        // Check rate limits for this key
        rate_limit = get_rate_limit_for_key(api_key_record)
        IF NOT rate_limit.check():
            RETURN error("Rate limit exceeded", 429)
        
        RETURN success(user=api_key_record.user, permissions=api_key_record.permissions)
    
    // Try Bearer Token authentication
    IF auth_header STARTS WITH "Bearer ":
        token = extract_token(auth_header)
        
        // Validate JWT token
        jwt_payload = validate_jwt_token(token)
        IF jwt_payload IS NULL:
            RETURN error("Invalid token", 401)
        
        IF jwt_payload.expired:
            RETURN error("Token expired", 401)
        
        // Check rate limits for this user
        rate_limit = get_rate_limit_for_user(jwt_payload.user_id)
        IF NOT rate_limit.check():
            RETURN error("Rate limit exceeded", 429)
        
        RETURN success(user=jwt_payload.user, permissions=jwt_payload.permissions)
    
    // Try OAuth2 token
    oauth_token = validate_oauth2_token(token)
    IF oauth_token IS NOT NULL:
        RETURN success(user=oauth_token.user, permissions=oauth_token.scopes)
    
    // No valid authentication found
    RETURN error("Authentication required", 401)
END FUNCTION
```

### Authorization Pseudocode

```pseudocode
FUNCTION authorize_action(user, action, resource):
    // Check user permissions
    user_permissions = get_user_permissions(user.id)
    
    // Check if user has required permission
    required_permission = get_required_permission(action, resource)
    
    IF required_permission NOT IN user_permissions:
        RETURN error("Insufficient permissions", 403)
    
    // Check resource-level permissions
    IF resource.owner_id != user.id AND "admin" NOT IN user_permissions:
        RETURN error("Access denied to resource", 403)
    
    RETURN success()
END FUNCTION
```

---

## Rate Limiting Strategy

### Token Bucket Algorithm

```pseudocode
CLASS TokenBucket:
    capacity: INTEGER          // Maximum tokens
    tokens: FLOAT             // Current tokens
    refill_rate: FLOAT        // Tokens per second
    last_refill: TIMESTAMP    // Last refill time
    
    FUNCTION check_rate_limit():
        current_time = NOW()
        time_passed = current_time - last_refill
        
        // Refill tokens based on time passed
        tokens_to_add = time_passed * refill_rate
        tokens = MIN(capacity, tokens + tokens_to_add)
        last_refill = current_time
        
        // Check if request can be processed
        IF tokens >= 1.0:
            tokens = tokens - 1.0
            RETURN true
        ELSE:
            RETURN false
    END FUNCTION
END CLASS
```

### Rate Limiting Implementation

```pseudocode
FUNCTION apply_rate_limiting(request, user):
    // Get rate limit configuration for user
    rate_limit_config = get_rate_limit_config(user)
    
    // Create or get token bucket for user
    bucket = get_or_create_bucket(user.id, rate_limit_config)
    
    // Check rate limit
    IF NOT bucket.check_rate_limit():
        // Calculate retry-after
        retry_after = calculate_retry_after(bucket)
        
        RETURN error(
            message="Rate limit exceeded",
            status_code=429,
            headers={
                "X-RateLimit-Limit": rate_limit_config.limit,
                "X-RateLimit-Remaining": 0,
                "X-RateLimit-Reset": bucket.reset_time,
                "Retry-After": retry_after
            }
        )
    
    // Update rate limit headers
    response_headers = {
        "X-RateLimit-Limit": rate_limit_config.limit,
        "X-RateLimit-Remaining": bucket.tokens,
        "X-RateLimit-Reset": bucket.reset_time
    }
    
    RETURN success(headers=response_headers)
END FUNCTION
```

### Rate Limit Tiers

```pseudocode
RATE_LIMIT_TIERS = {
    "free": {
        requests_per_second: 10,
        requests_per_minute: 100,
        requests_per_hour: 1000,
        burst_size: 20
    },
    "basic": {
        requests_per_second: 50,
        requests_per_minute: 500,
        requests_per_hour: 10000,
        burst_size: 100
    },
    "professional": {
        requests_per_second: 100,
        requests_per_minute: 1000,
        requests_per_hour: 100000,
        burst_size: 200
    },
    "enterprise": {
        requests_per_second: 500,
        requests_per_minute: 5000,
        requests_per_hour: 500000,
        burst_size: 1000
    }
}
```

---

## Error Handling Framework

### Error Response Structure

```pseudocode
STRUCTURE ErrorResponse:
    error: {
        code: STRING              // Machine-readable error code
        message: STRING           // Human-readable message
        details: OBJECT?          // Additional error details
        request_id: STRING        // Request ID for tracing
        timestamp: TIMESTAMP      // When error occurred
        path: STRING              // API path that caused error
        method: STRING            // HTTP method
    }
    metadata: {
        version: STRING           // API version
        documentation_url: STRING // Link to error documentation
    }
END STRUCTURE
```

### Error Code Categories

```pseudocode
ERROR_CODES = {
    // Authentication errors (401)
    "AUTH_REQUIRED": "Authentication required",
    "AUTH_INVALID": "Invalid authentication credentials",
    "AUTH_EXPIRED": "Authentication token expired",
    "AUTH_INSUFFICIENT": "Insufficient authentication",
    
    // Authorization errors (403)
    "AUTHZ_DENIED": "Access denied",
    "AUTHZ_INSUFFICIENT": "Insufficient permissions",
    "AUTHZ_RESOURCE": "Access denied to resource",
    
    // Rate limiting errors (429)
    "RATE_LIMIT_EXCEEDED": "Rate limit exceeded",
    "RATE_LIMIT_QUOTA": "Quota exceeded",
    
    // Validation errors (400)
    "VALIDATION_FAILED": "Request validation failed",
    "VALIDATION_MISSING": "Required field missing",
    "VALIDATION_INVALID": "Invalid field value",
    "VALIDATION_FORMAT": "Invalid data format",
    
    // Not found errors (404)
    "NOT_FOUND": "Resource not found",
    "TOOL_NOT_FOUND": "Tool not found",
    "METHOD_NOT_FOUND": "Method not found",
    
    // Conflict errors (409)
    "CONFLICT": "Resource conflict",
    "TOOL_EXISTS": "Tool already registered",
    
    // Server errors (500)
    "INTERNAL_ERROR": "Internal server error",
    "DATABASE_ERROR": "Database operation failed",
    "TOOL_EXECUTION_ERROR": "Tool execution failed",
    "TIMEOUT": "Request timeout",
    
    // Service unavailable (503)
    "SERVICE_UNAVAILABLE": "Service temporarily unavailable",
    "TOOL_UNAVAILABLE": "Tool temporarily unavailable"
}
```

### Error Handling Pseudocode

```pseudocode
FUNCTION handle_error(error, request):
    // Determine error category
    error_category = categorize_error(error)
    
    // Map to HTTP status code
    status_code = map_error_to_status_code(error_category)
    
    // Generate request ID if not present
    request_id = request.headers.get("X-Request-ID") OR generate_uuid()
    
    // Create error response
    error_response = {
        error: {
            code: error.code,
            message: error.message,
            details: error.details,
            request_id: request_id,
            timestamp: NOW(),
            path: request.path,
            method: request.method
        },
        metadata: {
            version: API_VERSION,
            documentation_url: f"{DOCS_BASE_URL}/errors/{error.code}"
        }
    }
    
    // Log error (with appropriate level)
    IF status_code >= 500:
        LOG_ERROR(error, request_id, request)
    ELSE IF status_code >= 400:
        LOG_WARNING(error, request_id, request)
    ELSE:
        LOG_INFO(error, request_id, request)
    
    // Return error response
    RETURN response(
        status_code=status_code,
        body=error_response,
        headers={
            "X-Request-ID": request_id,
            "Content-Type": "application/json"
        }
    )
END FUNCTION
```

---

## API Versioning

### Versioning Strategy

```pseudocode
VERSIONING_STRATEGY = "URL_PATH"  // /v1/, /v2/, etc.

SUPPORTED_VERSIONS = ["v1"]
DEFAULT_VERSION = "v1"
DEPRECATED_VERSIONS = []  // Track deprecated versions

FUNCTION extract_api_version(request):
    path = request.path
    
    // Extract version from URL path
    IF path MATCHES "/v(\d+)/":
        version = extract_version_number(path)
        IF version IN SUPPORTED_VERSIONS:
            RETURN version
        ELSE:
            RETURN error("Unsupported API version", 400)
    
    // Default to latest version
    RETURN DEFAULT_VERSION
END FUNCTION

FUNCTION handle_versioned_request(request):
    version = extract_api_version(request)
    
    // Check if version is deprecated
    IF version IN DEPRECATED_VERSIONS:
        response.headers["Deprecation"] = "true"
        response.headers["Sunset"] = get_sunset_date(version)
        response.headers["Link"] = f"<{LATEST_VERSION_URL}>; rel=\"successor-version\""
    
    // Route to version-specific handler
    handler = get_version_handler(version)
    RETURN handler.process(request)
END FUNCTION
```

---

## Request/Response Formats

### Request Format

```pseudocode
STRUCTURE StandardRequest:
    headers: {
        "Content-Type": "application/json",
        "Accept": "application/json",
        "Authorization": "Bearer <token>" OR "X-API-Key: <key>",
        "X-Request-ID": STRING?,
        "X-API-Version": STRING?,
        "X-Client-Version": STRING?,
        "User-Agent": STRING
    }
    body: JSON_OBJECT?
    query_params: {
        page: INTEGER?,
        limit: INTEGER?,
        sort: STRING?,
        filter: STRING?,
        fields: STRING?  // Comma-separated field list
    }
END STRUCTURE
```

### Response Format

```pseudocode
STRUCTURE StandardResponse:
    headers: {
        "Content-Type": "application/json",
        "X-Request-ID": STRING,
        "X-API-Version": STRING,
        "X-RateLimit-Limit": INTEGER,
        "X-RateLimit-Remaining": INTEGER,
        "X-RateLimit-Reset": TIMESTAMP
    }
    body: {
        data: ANY,              // Response data
        meta: {                 // Metadata
            request_id: STRING,
            timestamp: TIMESTAMP,
            version: STRING,
            pagination: {
                page: INTEGER,
                limit: INTEGER,
                total: INTEGER,
                pages: INTEGER
            }?
        },
        links: {                // HATEOAS links
            self: STRING,
            first: STRING?,
            prev: STRING?,
            next: STRING?,
            last: STRING?
        }?
    }
END STRUCTURE
```

### Pagination

```pseudocode
FUNCTION paginate_results(results, page, limit):
    // Validate pagination parameters
    page = MAX(1, page)
    limit = CLAMP(1, MAX_PAGE_SIZE, limit)
    
    // Calculate pagination
    total = results.length
    total_pages = CEIL(total / limit)
    offset = (page - 1) * limit
    
    // Slice results
    paginated_results = results[offset:offset + limit]
    
    // Build pagination metadata
    pagination = {
        page: page,
        limit: limit,
        total: total,
        pages: total_pages,
        has_next: page < total_pages,
        has_prev: page > 1
    }
    
    // Build HATEOAS links
    links = {
        self: build_url(page=page, limit=limit),
        first: build_url(page=1, limit=limit),
        prev: IF page > 1 THEN build_url(page=page-1, limit=limit) ELSE NULL,
        next: IF page < total_pages THEN build_url(page=page+1, limit=limit) ELSE NULL,
        last: build_url(page=total_pages, limit=limit)
    }
    
    RETURN {
        data: paginated_results,
        meta: { pagination: pagination },
        links: links
    }
END FUNCTION
```

---

## Endpoint Specifications

### Health Check Endpoint

```pseudocode
ENDPOINT: GET /v1/health

FUNCTION health_check_handler():
    // Check system health
    database_healthy = check_database_health()
    registry_healthy = check_registry_health()
    
    // Determine overall health
    overall_health = database_healthy AND registry_healthy
    
    // Build response
    response = {
        status: IF overall_health THEN "healthy" ELSE "unhealthy",
        version: API_VERSION,
        timestamp: NOW(),
        checks: {
            database: {
                status: IF database_healthy THEN "healthy" ELSE "unhealthy",
                response_time_ms: database_response_time
            },
            registry: {
                status: IF registry_healthy THEN "healthy" ELSE "unhealthy",
                tools_registered: registry.count()
            }
        },
        uptime_seconds: get_uptime()
    }
    
    status_code = IF overall_health THEN 200 ELSE 503
    
    RETURN response(status_code=status_code, body=response)
END FUNCTION
```

### Tool Registration Endpoint

```pseudocode
ENDPOINT: POST /v1/tools/register

REQUEST_BODY: {
    name: STRING (required, pattern: "^[a-z0-9-]+$"),
    version: STRING (required, semver format),
    description: STRING (required, max_length: 500),
    endpoint: STRING (required, valid URL),
    methods: ARRAY<{
        name: STRING (required),
        description: STRING (required),
        input_schema: JSON_SCHEMA (required)
    }> (required, min_items: 1),
    metadata: OBJECT? (optional),
    health_check_url: STRING? (optional, valid URL),
    rate_limit: {
        requests_per_second: INTEGER?,
        requests_per_minute: INTEGER?,
        requests_per_hour: INTEGER?
    }? (optional)
}

FUNCTION register_tool_handler(request):
    // Authenticate request
    auth_result = authenticate_request(request)
    IF NOT auth_result.success:
        RETURN handle_error(auth_result.error, request)
    
    // Authorize action
    authz_result = authorize_action(auth_result.user, "tools:register", NULL)
    IF NOT authz_result.success:
        RETURN handle_error(authz_result.error, request)
    
    // Validate request body
    validation_result = validate_tool_registration(request.body)
    IF NOT validation_result.valid:
        RETURN handle_error(validation_result.errors, request)
    
    // Check if tool already exists
    existing_tool = registry.get(request.body.name)
    IF existing_tool IS NOT NULL:
        RETURN handle_error({
            code: "TOOL_EXISTS",
            message: f"Tool '{request.body.name}' already registered"
        }, request, status_code=409)
    
    // Create tool info
    tool_info = {
        name: request.body.name,
        version: request.body.version,
        description: request.body.description,
        endpoint: request.body.endpoint,
        methods: request.body.methods,
        metadata: request.body.metadata OR {},
        health_check_url: request.body.health_check_url,
        rate_limit: request.body.rate_limit,
        registered_at: NOW(),
        registered_by: auth_result.user.id
    }
    
    // Register tool
    registration_result = registry.register(tool_info)
    IF NOT registration_result.success:
        RETURN handle_error(registration_result.error, request)
    
    // Persist to database
    database.save_tool(tool_info)
    
    // Perform health check
    health_status = perform_health_check(tool_info)
    
    // Build response
    response = {
        data: {
            tool_id: tool_info.name,
            status: "registered",
            health: health_status,
            registered_at: tool_info.registered_at
        },
        meta: {
            request_id: generate_request_id(),
            timestamp: NOW()
        }
    }
    
    RETURN response(status_code=201, body=response)
END FUNCTION
```

### List Tools Endpoint

```pseudocode
ENDPOINT: GET /v1/tools

QUERY_PARAMS: {
    page: INTEGER? (default: 1, min: 1),
    limit: INTEGER? (default: 20, min: 1, max: 100),
    status: STRING? ("healthy" | "unhealthy" | "all"),
    search: STRING? (search in name/description),
    sort: STRING? ("name" | "version" | "registered_at", default: "name"),
    order: STRING? ("asc" | "desc", default: "asc")
}

FUNCTION list_tools_handler(request):
    // Authenticate request
    auth_result = authenticate_request(request)
    IF NOT auth_result.success:
        RETURN handle_error(auth_result.error, request)
    
    // Extract query parameters
    page = request.query.get("page", 1)
    limit = request.query.get("limit", 20)
    status_filter = request.query.get("status", "all")
    search_query = request.query.get("search")
    sort_field = request.query.get("sort", "name")
    sort_order = request.query.get("order", "asc")
    
    // Get tools from registry
    all_tools = registry.list_all()
    
    // Apply filters
    filtered_tools = all_tools
    IF status_filter != "all":
        filtered_tools = filtered_tools.filter(tool => tool.health == status_filter)
    
    IF search_query IS NOT NULL:
        filtered_tools = filtered_tools.filter(tool => 
            tool.name.contains(search_query) OR 
            tool.description.contains(search_query)
        )
    
    // Sort tools
    filtered_tools = sort_tools(filtered_tools, sort_field, sort_order)
    
    // Paginate results
    paginated_result = paginate_results(filtered_tools, page, limit)
    
    // Build response
    response = {
        data: paginated_result.data.map(tool => {
            name: tool.name,
            version: tool.version,
            description: tool.description,
            health: tool.health,
            methods_count: tool.methods.length,
            registered_at: tool.registered_at
        }),
        meta: {
            request_id: generate_request_id(),
            timestamp: NOW(),
            pagination: paginated_result.meta.pagination
        },
        links: paginated_result.links
    }
    
    RETURN response(status_code=200, body=response)
END FUNCTION
```

### Call Tool Endpoint

```pseudocode
ENDPOINT: POST /v1/tools/{tool_name}/call

PATH_PARAMS: {
    tool_name: STRING (required)
}

REQUEST_BODY: {
    method: STRING (required),
    arguments: OBJECT (required),
    timeout: INTEGER? (optional, seconds, default: 30),
    cache: BOOLEAN? (optional, default: true),
    async: BOOLEAN? (optional, default: false)
}

FUNCTION call_tool_handler(request, tool_name):
    // Authenticate request
    auth_result = authenticate_request(request)
    IF NOT auth_result.success:
        RETURN handle_error(auth_result.error, request)
    
    // Check rate limiting
    rate_limit_result = apply_rate_limiting(request, auth_result.user)
    IF NOT rate_limit_result.success:
        RETURN handle_error(rate_limit_result.error, request)
    
    // Get tool from registry
    tool = registry.get(tool_name)
    IF tool IS NULL:
        RETURN handle_error({
            code: "TOOL_NOT_FOUND",
            message: f"Tool '{tool_name}' not found"
        }, request, status_code=404)
    
    // Check tool health
    IF NOT tool.healthy:
        RETURN handle_error({
            code: "TOOL_UNAVAILABLE",
            message: f"Tool '{tool_name}' is currently unavailable"
        }, request, status_code=503)
    
    // Validate method exists
    method = tool.methods.find(m => m.name == request.body.method)
    IF method IS NULL:
        RETURN handle_error({
            code: "METHOD_NOT_FOUND",
            message: f"Method '{request.body.method}' not found in tool '{tool_name}'"
        }, request, status_code=404)
    
    // Validate arguments against schema
    validation_result = validate_against_schema(
        request.body.arguments,
        method.input_schema
    )
    IF NOT validation_result.valid:
        RETURN handle_error({
            code: "VALIDATION_FAILED",
            message: "Arguments validation failed",
            details: validation_result.errors
        }, request, status_code=400)
    
    // Check cache if enabled
    cache_key = generate_cache_key(tool_name, method.name, request.body.arguments)
    IF request.body.cache != false:
        cached_result = database.get_cached_result(cache_key)
        IF cached_result IS NOT NULL AND NOT cached_result.expired:
            RETURN response(status_code=200, body={
                data: cached_result.data,
                meta: {
                    request_id: generate_request_id(),
                    timestamp: NOW(),
                    cached: true,
                    cache_age_seconds: cached_result.age
                }
            })
    
    // Execute tool (async or sync)
    IF request.body.async == true:
        // Create async job
        job_id = create_async_job(tool, method, request.body.arguments)
        RETURN response(status_code=202, body={
            data: {
                job_id: job_id,
                status: "pending"
            },
            meta: {
                request_id: generate_request_id(),
                timestamp: NOW()
            },
            links: {
                status: f"/v1/jobs/{job_id}",
                cancel: f"/v1/jobs/{job_id}/cancel"
            }
        })
    ELSE:
        // Execute synchronously
        execution_start = NOW()
        execution_result = execute_tool(
            tool=tool,
            method=method,
            arguments=request.body.arguments,
            timeout=request.body.timeout OR 30
        )
        execution_duration = NOW() - execution_start
        
        // Log execution
        database.log_execution({
            tool_name: tool_name,
            method: method.name,
            user_id: auth_result.user.id,
            success: execution_result.success,
            duration_ms: execution_duration,
            timestamp: NOW()
        })
        
        // Cache result if successful and caching enabled
        IF execution_result.success AND request.body.cache != false:
            database.cache_result(cache_key, execution_result.data, ttl=3600)
        
        // Build response
        IF execution_result.success:
            RETURN response(status_code=200, body={
                data: execution_result.data,
                meta: {
                    request_id: generate_request_id(),
                    timestamp: NOW(),
                    execution_time_ms: execution_duration,
                    cached: false
                }
            })
        ELSE:
            RETURN handle_error({
                code: "TOOL_EXECUTION_ERROR",
                message: execution_result.error_message,
                details: execution_result.error_details
            }, request, status_code=500)
END FUNCTION
```

### Get Tool Details Endpoint

```pseudocode
ENDPOINT: GET /v1/tools/{tool_name}

PATH_PARAMS: {
    tool_name: STRING (required)
}

FUNCTION get_tool_handler(request, tool_name):
    // Authenticate request
    auth_result = authenticate_request(request)
    IF NOT auth_result.success:
        RETURN handle_error(auth_result.error, request)
    
    // Get tool from registry
    tool = registry.get(tool_name)
    IF tool IS NULL:
        RETURN handle_error({
            code: "TOOL_NOT_FOUND",
            message: f"Tool '{tool_name}' not found"
        }, request, status_code=404)
    
    // Build response
    response = {
        data: {
            name: tool.name,
            version: tool.version,
            description: tool.description,
            endpoint: tool.endpoint,
            health: tool.health,
            health_check_url: tool.health_check_url,
            methods: tool.methods.map(method => {
                name: method.name,
                description: method.description,
                input_schema: method.input_schema
            }),
            metadata: tool.metadata,
            rate_limit: tool.rate_limit,
            registered_at: tool.registered_at,
            registered_by: tool.registered_by,
            last_health_check: tool.last_health_check,
            statistics: {
                total_calls: database.get_tool_statistics(tool_name).total_calls,
                success_rate: database.get_tool_statistics(tool_name).success_rate,
                average_duration_ms: database.get_tool_statistics(tool_name).average_duration_ms
            }
        },
        meta: {
            request_id: generate_request_id(),
            timestamp: NOW()
        }
    }
    
    RETURN response(status_code=200, body=response)
END FUNCTION
```

### Delete Tool Endpoint

```pseudocode
ENDPOINT: DELETE /v1/tools/{tool_name}

PATH_PARAMS: {
    tool_name: STRING (required)
}

FUNCTION delete_tool_handler(request, tool_name):
    // Authenticate request
    auth_result = authenticate_request(request)
    IF NOT auth_result.success:
        RETURN handle_error(auth_result.error, request)
    
    // Authorize action (only admins or tool owner)
    tool = registry.get(tool_name)
    IF tool IS NULL:
        RETURN handle_error({
            code: "TOOL_NOT_FOUND",
            message: f"Tool '{tool_name}' not found"
        }, request, status_code=404)
    
    authz_result = authorize_action(
        auth_result.user,
        "tools:delete",
        tool
    )
    IF NOT authz_result.success:
        RETURN handle_error(authz_result.error, request)
    
    // Unregister tool
    registry.unregister(tool_name)
    
    // Remove from database
    database.delete_tool(tool_name)
    
    // Build response
    response = {
        data: {
            tool_id: tool_name,
            status: "deleted"
        },
        meta: {
            request_id: generate_request_id(),
            timestamp: NOW()
        }
    }
    
    RETURN response(status_code=200, body=response)
END FUNCTION
```

### Statistics Endpoint

```pseudocode
ENDPOINT: GET /v1/stats

QUERY_PARAMS: {
    tool: STRING? (filter by tool name),
    period: STRING? ("hour" | "day" | "week" | "month", default: "day")
}

FUNCTION stats_handler(request):
    // Authenticate request
    auth_result = authenticate_request(request)
    IF NOT auth_result.success:
        RETURN handle_error(auth_result.error, request)
    
    // Authorize action (stats viewing)
    authz_result = authorize_action(auth_result.user, "stats:view", NULL)
    IF NOT authz_result.success:
        RETURN handle_error(authz_result.error, request)
    
    // Extract query parameters
    tool_filter = request.query.get("tool")
    period = request.query.get("period", "day")
    
    // Get statistics
    stats = database.get_statistics(tool_filter, period)
    
    // Build response
    response = {
        data: {
            overall: {
                total_tools: registry.count(),
                healthy_tools: registry.count_healthy(),
                total_requests: stats.total_requests,
                successful_requests: stats.successful_requests,
                failed_requests: stats.failed_requests,
                average_response_time_ms: stats.average_response_time_ms,
                cache_hit_rate: stats.cache_hit_rate
            },
            tools: stats.tool_statistics.map(tool_stat => {
                tool_name: tool_stat.tool_name,
                total_calls: tool_stat.total_calls,
                success_rate: tool_stat.success_rate,
                average_duration_ms: tool_stat.average_duration_ms,
                error_rate: tool_stat.error_rate
            }),
            period: period,
            timestamp: NOW()
        },
        meta: {
            request_id: generate_request_id(),
            timestamp: NOW()
        }
    }
    
    RETURN response(status_code=200, body=response)
END FUNCTION
```

---

## WebSocket Support

### WebSocket Connection

```pseudocode
ENDPOINT: WS /v1/ws

FUNCTION websocket_handler(websocket):
    // Authenticate WebSocket connection
    auth_result = authenticate_websocket(websocket)
    IF NOT auth_result.success:
        websocket.send_error("Authentication failed")
        websocket.close()
        RETURN
    
    // Accept connection
    websocket.accept()
    
    // Register client
    client_id = register_websocket_client(auth_result.user, websocket)
    
    // Send welcome message
    websocket.send({
        type: "connected",
        client_id: client_id,
        timestamp: NOW()
    })
    
    // Handle messages
    WHILE websocket.is_connected():
        message = websocket.receive()
        
        IF message.type == "subscribe":
            handle_subscribe(client_id, message.channels)
        
        ELSE IF message.type == "unsubscribe":
            handle_unsubscribe(client_id, message.channels)
        
        ELSE IF message.type == "ping":
            websocket.send({ type: "pong", timestamp: NOW() })
        
        ELSE IF message.type == "tool_call":
            handle_websocket_tool_call(client_id, message)
        
        ELSE:
            websocket.send_error("Unknown message type")
    
    // Cleanup on disconnect
    unregister_websocket_client(client_id)
END FUNCTION
```

### Real-time Progress Updates

```pseudocode
FUNCTION execute_tool_with_progress(tool, method, arguments, websocket_client):
    // Send start notification
    websocket_client.send({
        type: "tool_call_started",
        tool: tool.name,
        method: method.name,
        timestamp: NOW()
    })
    
    // Execute tool with progress callbacks
    progress_callback = FUNCTION(progress):
        websocket_client.send({
            type: "tool_call_progress",
            tool: tool.name,
            method: method.name,
            progress: progress.percentage,
            message: progress.message,
            timestamp: NOW()
        })
    END FUNCTION
    
    result = execute_tool_async(
        tool=tool,
        method=method,
        arguments=arguments,
        progress_callback=progress_callback
    )
    
    // Send completion notification
    IF result.success:
        websocket_client.send({
            type: "tool_call_completed",
            tool: tool.name,
            method: method.name,
            result: result.data,
            timestamp: NOW()
        })
    ELSE:
        websocket_client.send({
            type: "tool_call_failed",
            tool: tool.name,
            method: method.name,
            error: result.error,
            timestamp: NOW()
        })
    
    RETURN result
END FUNCTION
```

---

## Caching Strategy

### Cache Implementation

```pseudocode
CLASS CacheManager:
    database: PlatformDatabase
    default_ttl: INTEGER = 3600  // 1 hour
    
    FUNCTION get(key: STRING):
        entry = database.get_cache_entry(key)
        
        IF entry IS NULL:
            RETURN NULL
        
        IF entry.expired:
            database.delete_cache_entry(key)
            RETURN NULL
        
        RETURN entry.data
    END FUNCTION
    
    FUNCTION set(key: STRING, value: ANY, ttl: INTEGER?):
        ttl = ttl OR default_ttl
        expires_at = NOW() + ttl
        
        database.set_cache_entry({
            key: key,
            data: value,
            expires_at: expires_at,
            created_at: NOW()
        })
    END FUNCTION
    
    FUNCTION generate_cache_key(tool_name, method_name, arguments):
        // Create deterministic key from arguments
        arguments_hash = sha256(JSON.stringify(sort_keys(arguments)))
        RETURN f"{tool_name}:{method_name}:{arguments_hash}"
    END FUNCTION
    
    FUNCTION invalidate_tool_cache(tool_name):
        database.delete_cache_entries_matching(f"{tool_name}:*")
    END FUNCTION
END CLASS
```

---

## Health Checks & Monitoring

### Health Check Implementation

```pseudocode
FUNCTION perform_tool_health_check(tool):
    IF tool.health_check_url IS NULL:
        // Try to ping endpoint
        health_status = ping_endpoint(tool.endpoint)
        RETURN health_status
    
    // Perform custom health check
    health_check_result = http_get(tool.health_check_url, timeout=5)
    
    IF health_check_result.status_code == 200:
        RETURN {
            healthy: true,
            response_time_ms: health_check_result.duration,
            timestamp: NOW()
        }
    ELSE:
        RETURN {
            healthy: false,
            error: health_check_result.error,
            timestamp: NOW()
        }
END FUNCTION

FUNCTION schedule_periodic_health_checks():
    WHILE true:
        tools = registry.list_all()
        
        FOR EACH tool IN tools:
            health_status = perform_tool_health_check(tool)
            registry.update_tool_health(tool.name, health_status)
            database.update_tool_health(tool.name, health_status)
        
        SLEEP(60)  // Check every minute
END FUNCTION
```

### Metrics Collection

```pseudocode
CLASS MetricsCollector:
    FUNCTION record_request_metric(endpoint, method, status_code, duration_ms):
        metrics.increment("http_requests_total", {
            endpoint: endpoint,
            method: method,
            status_code: status_code
        })
        
        metrics.histogram("http_request_duration_ms", duration_ms, {
            endpoint: endpoint,
            method: method
        })
    END FUNCTION
    
    FUNCTION record_tool_execution_metric(tool_name, method_name, success, duration_ms):
        metrics.increment("tool_executions_total", {
            tool: tool_name,
            method: method_name,
            success: success
        })
        
        metrics.histogram("tool_execution_duration_ms", duration_ms, {
            tool: tool_name,
            method: method_name
        })
    END FUNCTION
    
    FUNCTION record_cache_metric(hit: BOOLEAN):
        IF hit:
            metrics.increment("cache_hits_total")
        ELSE:
            metrics.increment("cache_misses_total")
    END FUNCTION
END CLASS
```

---

## Security Best Practices

### Input Validation

```pseudocode
FUNCTION validate_input(data, schema):
    // Use JSON Schema validation
    validator = JSONSchemaValidator(schema)
    result = validator.validate(data)
    
    IF NOT result.valid:
        RETURN {
            valid: false,
            errors: result.errors.map(error => {
                field: error.path,
                message: error.message,
                code: error.code
            })
        }
    
    RETURN { valid: true }
END FUNCTION

FUNCTION sanitize_string(input: STRING):
    // Remove null bytes
    input = input.replace("\0", "")
    
    // Trim whitespace
    input = input.trim()
    
    // Limit length
    input = input.substring(0, MAX_STRING_LENGTH)
    
    RETURN input
END FUNCTION
```

### Path Traversal Prevention

```pseudocode
FUNCTION validate_file_path(path: STRING):
    // Normalize path
    normalized_path = normalize_path(path)
    
    // Check for path traversal attempts
    IF normalized_path.contains(".."):
        RETURN error("Path traversal detected")
    
    // Check against allowed roots
    allowed_roots = get_allowed_roots()
    is_allowed = false
    
    FOR EACH root IN allowed_roots:
        IF normalized_path.starts_with(root):
            is_allowed = true
            BREAK
    
    IF NOT is_allowed:
        RETURN error("Path not in allowed roots")
    
    RETURN success(normalized_path)
END FUNCTION
```

### SQL Injection Prevention

```pseudocode
// Use parameterized queries only
FUNCTION safe_database_query(query_template, parameters):
    // All queries must use parameterized statements
    // Never concatenate user input into SQL
    
    prepared_statement = database.prepare(query_template)
    result = prepared_statement.execute(parameters)
    
    RETURN result
END FUNCTION
```

---

## Performance Optimization

### Connection Pooling

```pseudocode
CLASS ConnectionPool:
    max_connections: INTEGER = 10
    connections: ARRAY<Connection>
    
    FUNCTION get_connection():
        IF connections.length < max_connections:
            connection = create_new_connection()
            connections.append(connection)
            RETURN connection
        
        // Wait for available connection
        WHILE true:
            FOR EACH connection IN connections:
                IF connection.is_idle():
                    RETURN connection
            SLEEP(10)  // Wait 10ms
    END FUNCTION
    
    FUNCTION release_connection(connection):
        connection.mark_idle()
    END FUNCTION
END CLASS
```

### Async Processing

```pseudocode
FUNCTION process_request_async(request):
    // Create background job
    job = create_job({
        type: "tool_execution",
        request: request,
        status: "pending",
        created_at: NOW()
    })
    
    // Queue for processing
    job_queue.enqueue(job)
    
    // Return job ID immediately
    RETURN {
        job_id: job.id,
        status: "pending",
        status_url: f"/v1/jobs/{job.id}"
    }
END FUNCTION

FUNCTION process_job_worker():
    WHILE true:
        job = job_queue.dequeue()
        
        IF job IS NULL:
            SLEEP(1)
            CONTINUE
        
        // Update job status
        job.status = "processing"
        job.started_at = NOW()
        database.update_job(job)
        
        // Execute tool
        result = execute_tool(job.request)
        
        // Update job with result
        job.status = IF result.success THEN "completed" ELSE "failed"
        job.completed_at = NOW()
        job.result = result
        database.update_job(job)
        
        // Notify WebSocket clients if subscribed
        notify_websocket_clients(job)
END FUNCTION
```

---

## Implementation Pseudocode

### Main Server Loop

```pseudocode
FUNCTION main():
    // Load configuration
    config = load_configuration()
    
    // Initialize components
    database = PlatformDatabase.new(config.database_path)
    registry = ToolRegistry.new()
    cache = CacheManager.new(database)
    metrics = MetricsCollector.new()
    rate_limiter = RateLimiter.new(config.rate_limit_rps)
    
    // Load persisted tools
    persisted_tools = database.get_all_tools()
    FOR EACH tool IN persisted_tools:
        registry.register(tool)
    
    // Start background tasks
    START_THREAD(schedule_periodic_health_checks)
    START_THREAD(process_job_worker)
    START_THREAD(cleanup_expired_cache_entries)
    
    // Build HTTP router
    router = Router.new()
        .route("GET", "/v1/health", health_check_handler)
        .route("POST", "/v1/tools/register", register_tool_handler)
        .route("GET", "/v1/tools", list_tools_handler)
        .route("GET", "/v1/tools/{tool_name}", get_tool_handler)
        .route("DELETE", "/v1/tools/{tool_name}", delete_tool_handler)
        .route("POST", "/v1/tools/{tool_name}/call", call_tool_handler)
        .route("GET", "/v1/stats", stats_handler)
        .route("GET", "/v1/metrics", metrics_handler)
        .route("WS", "/v1/ws", websocket_handler)
    
    // Apply middleware
    router = router
        .middleware(authentication_middleware)
        .middleware(rate_limiting_middleware)
        .middleware(request_logging_middleware)
        .middleware(error_handling_middleware)
        .middleware(cors_middleware)
        .middleware(timeout_middleware)
        .middleware(concurrency_limit_middleware)
    
    // Start HTTP server
    server = HTTPServer.new(router, config.port)
    server.start()
    
    // Wait for shutdown signal
    WAIT_FOR_SHUTDOWN_SIGNAL()
    
    // Graceful shutdown
    server.stop()
    database.close()
END FUNCTION
```

---

## Error Response Examples

### 400 Bad Request

```json
{
  "error": {
    "code": "VALIDATION_FAILED",
    "message": "Request validation failed",
    "details": {
      "field": "name",
      "reason": "Tool name must match pattern: ^[a-z0-9-]+$"
    },
    "request_id": "req_1234567890",
    "timestamp": "2025-12-21T10:30:00Z",
    "path": "/v1/tools/register",
    "method": "POST"
  },
  "metadata": {
    "version": "v1",
    "documentation_url": "https://docs.pyro-platform.local/errors/VALIDATION_FAILED"
  }
}
```

### 401 Unauthorized

```json
{
  "error": {
    "code": "AUTH_REQUIRED",
    "message": "Authentication required",
    "request_id": "req_1234567890",
    "timestamp": "2025-12-21T10:30:00Z",
    "path": "/v1/tools",
    "method": "GET"
  },
  "metadata": {
    "version": "v1",
    "documentation_url": "https://docs.pyro-platform.local/errors/AUTH_REQUIRED"
  }
}
```

### 429 Rate Limit Exceeded

```json
{
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "Rate limit exceeded. Please retry after 5 seconds.",
    "request_id": "req_1234567890",
    "timestamp": "2025-12-21T10:30:00Z",
    "path": "/v1/tools/totalimage/call",
    "method": "POST"
  },
  "metadata": {
    "version": "v1",
    "documentation_url": "https://docs.pyro-platform.local/errors/RATE_LIMIT_EXCEEDED"
  }
}
```

---

## Success Response Examples

### Tool Registration

```json
{
  "data": {
    "tool_id": "totalimage",
    "status": "registered",
    "health": "healthy",
    "registered_at": "2025-12-21T10:30:00Z"
  },
  "meta": {
    "request_id": "req_1234567890",
    "timestamp": "2025-12-21T10:30:00Z"
  }
}
```

### Tool Call Success

```json
{
  "data": {
    "vault_type": "VHD Dynamic",
    "vault_size": 10737418240,
    "partitions": [
      {
        "index": 0,
        "type": "FAT32 (LBA)",
        "offset": 1048576,
        "length": 10736369664
      }
    ]
  },
  "meta": {
    "request_id": "req_1234567890",
    "timestamp": "2025-12-21T10:30:00Z",
    "execution_time_ms": 245,
    "cached": false
  }
}
```

---

## Conclusion

This pseudocode specification provides a comprehensive foundation for implementing the Fire Marshal API following industry best practices. Key principles:

1. **Security First**: Authentication, authorization, input validation, path traversal prevention
2. **Performance**: Caching, connection pooling, async processing, rate limiting
3. **Reliability**: Error handling, health checks, monitoring, graceful degradation
4. **Usability**: Clear error messages, HATEOAS links, pagination, versioning
5. **Observability**: Metrics, logging, request tracing, health endpoints

Implementation should follow this pseudocode structure while adapting to the specific Rust/Axum framework patterns used in the TotalImage project.

---

**Document Version:** 1.0.0  
**Last Updated:** December 21, 2025  
**Status:** Ready for Implementation
