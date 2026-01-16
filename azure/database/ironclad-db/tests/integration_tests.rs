/// Integration tests for the complete IronClad-DB system
/// 
/// These tests verify end-to-end functionality across all layers:
/// AzureDisk, BufferPool, WAL, and KVStore

use ironclad_db::{KVStore, BufferPool, WAL, WalEntry};

#[tokio::test]
async fn test_end_to_end_basic_operations() {
    let store = KVStore::new("test-connection").await.unwrap();
    
    // Test basic CRUD operations
    store.set("user:1", "Alice").await.unwrap();
    assert_eq!(store.get("user:1").await.unwrap(), Some("Alice".to_string()));
    
    store.set("user:2", "Bob").await.unwrap();
    assert_eq!(store.get("user:2").await.unwrap(), Some("Bob".to_string()));
    
    store.delete("user:1").await.unwrap();
    assert_eq!(store.get("user:1").await.unwrap(), None);
}

#[tokio::test]
async fn test_end_to_end_durability() {
    // Test that WAL provides durability
    let store = KVStore::new("test-durability").await.unwrap();
    
    // Write some data
    store.set("key1", "value1").await.unwrap();
    store.set("key2", "value2").await.unwrap();
    store.set("key3", "value3").await.unwrap();
    
    let stats = store.stats();
    assert_eq!(stats.num_keys, 3);
    assert!(stats.wal_entries > 0); // WAL should have entries
}

#[tokio::test]
async fn test_end_to_end_checkpoint_flow() {
    let store = KVStore::new("test-checkpoint").await.unwrap();
    
    // Add data
    for i in 0..10 {
        store.set(&format!("key:{}", i), &format!("value:{}", i)).await.unwrap();
    }
    
    let stats_before = store.stats();
    assert_eq!(stats_before.num_keys, 10);
    assert!(stats_before.wal_entries > 0);
    
    // Create checkpoint
    store.checkpoint().await.unwrap();
    
    let stats_after = store.stats();
    assert_eq!(stats_after.num_keys, 10); // Keys still there
    assert_eq!(stats_after.wal_entries, 0); // WAL cleared after checkpoint
}

#[tokio::test]
async fn test_buffer_pool_integration() {
    let bp = BufferPool::new();
    
    // Test putting multiple pages
    for i in 0..100 {
        let data = vec![i as u8; 4096];
        bp.put_page(i, data).unwrap();
    }
    
    let stats = bp.stats();
    assert_eq!(stats.used_frames, 100);
    
    // Test getting cached pages
    for i in 0..100 {
        let data = bp.get_page(i);
        assert!(data.is_some());
    }
}

#[tokio::test]
async fn test_wal_integration() {
    let wal = WAL::new("test-conn", "test-container", "test-wal").await.unwrap();
    
    // Test logging operations
    wal.append_entry(WalEntry::Set {
        key: "k1".to_string(),
        value: "v1".to_string(),
    }).await.unwrap();
    
    wal.append_entry(WalEntry::Set {
        key: "k2".to_string(),
        value: "v2".to_string(),
    }).await.unwrap();
    
    wal.append_entry(WalEntry::Delete {
        key: "k1".to_string(),
    }).await.unwrap();
    
    assert_eq!(wal.entry_count(), 3);
    
    // Test replay
    let entries = wal.replay().await.unwrap();
    assert_eq!(entries.len(), 3);
}

#[tokio::test]
async fn test_concurrent_operations() {
    use std::sync::Arc;
    use tokio::task;
    
    let store = Arc::new(KVStore::new("test-concurrent").await.unwrap());
    
    // Spawn multiple tasks performing operations concurrently
    let mut handles = vec![];
    
    for i in 0..10 {
        let store_clone = Arc::clone(&store);
        let handle = task::spawn(async move {
            let key = format!("concurrent:key:{}", i);
            let value = format!("value:{}", i);
            store_clone.set(&key, &value).await.unwrap();
            
            let retrieved = store_clone.get(&key).await.unwrap();
            assert_eq!(retrieved, Some(value));
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    let stats = store.stats();
    assert_eq!(stats.num_keys, 10);
}

#[tokio::test]
async fn test_large_dataset() {
    let store = KVStore::new("test-large").await.unwrap();
    
    // Insert 1000 key-value pairs
    for i in 0..1000 {
        let key = format!("large:key:{:04}", i);
        let value = format!("large_value_{:04}", i);
        store.set(&key, &value).await.unwrap();
    }
    
    // Verify all data is retrievable
    for i in 0..1000 {
        let key = format!("large:key:{:04}", i);
        let expected_value = format!("large_value_{:04}", i);
        let actual_value = store.get(&key).await.unwrap();
        assert_eq!(actual_value, Some(expected_value));
    }
    
    let stats = store.stats();
    assert_eq!(stats.num_keys, 1000);
}

#[tokio::test]
async fn test_scan_functionality() {
    let store = KVStore::new("test-scan").await.unwrap();
    
    // Insert test data
    store.set("apple", "fruit").await.unwrap();
    store.set("banana", "fruit").await.unwrap();
    store.set("carrot", "vegetable").await.unwrap();
    
    // Scan all entries
    let entries = store.scan().await.unwrap();
    assert_eq!(entries.len(), 3);
    
    // Verify all keys are present
    let keys: Vec<String> = entries.iter().map(|(k, _)| k.clone()).collect();
    assert!(keys.contains(&"apple".to_string()));
    assert!(keys.contains(&"banana".to_string()));
    assert!(keys.contains(&"carrot".to_string()));
}

#[tokio::test]
async fn test_update_operations() {
    let store = KVStore::new("test-update").await.unwrap();
    
    // Initial set
    store.set("counter", "0").await.unwrap();
    assert_eq!(store.get("counter").await.unwrap(), Some("0".to_string()));
    
    // Multiple updates
    for i in 1..=10 {
        store.set("counter", &i.to_string()).await.unwrap();
    }
    
    // Verify final value
    assert_eq!(store.get("counter").await.unwrap(), Some("10".to_string()));
}

#[tokio::test]
async fn test_delete_and_recreate() {
    let store = KVStore::new("test-delete-recreate").await.unwrap();
    
    // Create
    store.set("temp", "temporary").await.unwrap();
    assert_eq!(store.get("temp").await.unwrap(), Some("temporary".to_string()));
    
    // Delete
    store.delete("temp").await.unwrap();
    assert_eq!(store.get("temp").await.unwrap(), None);
    
    // Recreate with different value
    store.set("temp", "new_value").await.unwrap();
    assert_eq!(store.get("temp").await.unwrap(), Some("new_value".to_string()));
}

#[tokio::test]
async fn test_empty_scan() {
    let store = KVStore::new("test-empty-scan").await.unwrap();
    
    // Scan empty store
    let entries = store.scan().await.unwrap();
    assert_eq!(entries.len(), 0);
}

#[tokio::test]
async fn test_stats_accuracy() {
    let store = KVStore::new("test-stats").await.unwrap();
    
    // Initial stats
    let stats = store.stats();
    assert_eq!(stats.num_keys, 0);
    assert_eq!(stats.wal_entries, 0);
    
    // Add some data
    store.set("k1", "v1").await.unwrap();
    store.set("k2", "v2").await.unwrap();
    
    let stats = store.stats();
    assert_eq!(stats.num_keys, 2);
    assert!(stats.wal_entries >= 2);
    
    // Delete one
    store.delete("k1").await.unwrap();
    
    let stats = store.stats();
    assert_eq!(stats.num_keys, 1);
}
