# Project IronClad - Azure Page Blob KV Store

A persistent, crash-safe Key-Value Store built on Azure Page Blobs.
Inspired by Azure SQL and Rubrik's internal architecture.

## Architecture

- **AzureDisk**: Treats Azure Page Blobs as raw block devices
- **BufferPool**: LRU memory management with eviction
- **WAL**: Write-Ahead Log for durability and crash recovery
- **KVStore**: Key-Value store engine on top of the layers

## Building

```bash
cargo build --release
```

## Running

```bash
export AZURE_STORAGE_CONNECTION_STRING="DefaultEndpointsProtocol=https;AccountName=ironcladstor;AccountKey=...;EndpointSuffix=core.windows.net"
cargo run --release
```

## Features

- Durable writes with WAL
- Crash-safe design with automatic recovery
- 50MB buffer pool with LRU eviction  
- Azure blob storage for persistence
- Full ACID compliance

