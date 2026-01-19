use ironclad_db::KVStore;
use std::env;
use std::time::Instant;
use rand::Rng;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("\n🌊 RESERVOIR MECHANICS SIMULATION (The Flood) 🌊");
    println!("==================================================");

    // 1. Initialize Connection
    let connection_string = match env::var("AZURE_STORAGE_CONNECTION_STRING") {
        Ok(val) => val,
        Err(_) => {
            eprintln!("❌ AZURE_STORAGE_CONNECTION_STRING not set. Source the .env file first.");
            return Ok(());
        }
    };

    println!("▶ Connecting to Azure (The Deep Ocean)...");
    let store = KVStore::new(&connection_string).await?;
    println!("✓ Connected.\n");

    // 2. The Flood Parameters
    let total_drops = 1000; // Number of keys
    let drop_size = 1024;    // 1KB per value
    
    println!("▶ Initiating Inflow: {} drops of {} bytes each...", total_drops, drop_size);
    println!("  (This mimics filling the reservoir with water)\n");

    let start = Instant::now();
    let mut rng = rand::thread_rng();

    // 3. Pump Water
    for i in 0..total_drops {
        let key = format!("drop:{}", i);
        let mut value = vec![0u8; drop_size];
        rng.fill(&mut value[..]);
        let value_str = base64::encode(&value); // Store as string for simplicity

        store.set(&key, &value_str).await?;

        if (i + 1) % 100 == 0 {
            let stats = store.stats();
            println!("  💧 Level: {} keys | Reservoir: {}/{} MB used | WAL Height: {}", 
                i + 1, 
                stats.buffer_pool_used_mb, 
                stats.buffer_pool_total_mb,
                stats.wal_entries
            );
        }
    }

    let duration = start.elapsed();
    println!("\n✅ Flood Complete in {:.2?}", duration);
    println!("   Rate: {:.2} drops/sec", total_drops as f64 / duration.as_secs_f64());

    // 4. Inspect Reservoir State
    let stats = store.stats();
    println!("\n📊 Final Reservoir Status:");
    println!("  • Water Level (Keys): {}", stats.num_keys);
    println!("  • Pressure (WAL):     {} entries", stats.wal_entries);
    println!("  • Retention (Cache):  {}/{} MB", stats.buffer_pool_used_mb, stats.buffer_pool_total_mb);

    // 5. Release Water (Flush)
    println!("\n▶ Opening Spillway (Flushing to Ocean)...");
    store.flush().await?;
    println!("✓ Flushed to Azure Page Blobs.");

    println!("\n🌊 Simulation Ends. The structure held firm.");
    
    Ok(())
}
