fn main() {
    println!("Project IronClad - Azure Page Blob KV Store");
    println!("\nThis is a persistent, crash-safe Key-Value Store built on Azure Page Blobs.");
    println!("\nArchitecture:");
    println!("  - AzureDisk: Treats Azure Page Blobs as raw block devices");
    println!("  - BufferPool: LRU memory management with eviction");
    println!("  - WAL: Write-Ahead Log for durability and crash recovery");
    println!("  - KVStore: Key-Value store engine on top of the layers");
    println!("\nFeatures:");
    println!("  - Durable writes with Write-Ahead Log");
    println!("  - Crash-safe design with automatic recovery");
    println!("  - 50MB buffer pool with LRU eviction");
    println!("  - Azure blob storage for persistence");
    println!("  - Full ACID compliance");
    println!("\n📍 Location: /home/pulivarthi/cloud-experiments/azure/database/ironclad-db");
    println!("🔧 To build: cargo build --release");
    println!("🚀 To run: cargo run --release");
}
