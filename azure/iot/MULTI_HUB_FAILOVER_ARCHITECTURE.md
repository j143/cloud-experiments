# Azure IoT Hub Multi-Hub Failover Architecture

## Executive Summary

This document describes the principal engineer-level multi-region IoT Hub deployment architecture with active-passive failover capabilities. The deployment spans **East US (Primary)** and **West US (Secondary)** regions for high availability and disaster recovery.

## Architecture Overview

```
Primary Region (East US)          Secondary Region (West US)
┌──────────────────────┐         ┌──────────────────────┐
│ iot-exp-hub          │         │ iot-exp-hub-west     │
│ (Primary Hub)        │         │ (Failover Hub)       │
│ Standard Tier        │         │ Standard Tier        │
│ 400K msgs/day        │         │ 400K msgs/day        │
└──────────────────────┘         └──────────────────────┘
         │                                │
         └─────────────┬───────────────────┘
                       │
                ┌──────▼─────────┐
                │   Device/Client │
                │   Logic Layer   │
                │   (Failover)    │
                └────────────────┘
```

## 1. Deployment Architecture

### Hub Configuration

#### Primary Hub: iot-exp-hub
- **Region**: East US (eastus)
- **Tier**: Standard (S1)
- **Daily Message Limit**: 400,000 messages
- **Cost**: ₹2,079.87/month per hub
- **Status**: Active (receives all production traffic)
- **Role**: Primary ingestion point

#### Secondary Hub: iot-exp-hub-west
- **Region**: West US (westus)
- **Tier**: Standard (S1)
- **Daily Message Limit**: 400,000 messages
- **Cost**: ₹2,079.87/month per hub
- **Status**: Warm Standby (ready to assume traffic)
- **Role**: Failover target

### Network Configuration

**Connectivity**: Both hubs configured with Public Access (All Networks)
- **TLS Version**: 1.2 minimum (enforced)
- **Public Endpoints**:
  - Primary: `iot-exp-hub.azure-devices.net`
  - Secondary: `iot-exp-hub-west.azure-devices.net`

**Connection String Pattern**:
```
HostName={hub-name}.azure-devices.net;SharedAccessKeyName={policy};SharedAccessKey={key}
```

## 2. Failover Strategy

### Failover Triggers

1. **Primary Hub Unavailability**
   - Connection timeouts (>30 seconds)
   - Hub returning 503 Service Unavailable
   - Persistent authentication failures
   - Regional outage detection

2. **Primary Hub Health Degradation**
   - Message latency exceeding threshold (>5 seconds)
   - Message loss detection (ACK failures)
   - Connection drop rate spike (>10%)

### Failover Decision Logic

```python
def should_failover(primary_health):
    """
    Decision logic for failover activation
    """
    if primary_health.is_unreachable():
        return True  # Immediate failover
    
    if primary_health.error_rate > 0.1:  # >10% errors
        if primary_health.consecutive_errors > 5:
            return True  # Grace period + errors = failover
    
    if primary_health.message_latency > 5000:  # 5 seconds
        if primary_health.duration > 60:  # 60 second threshold
            return True
    
    return False
```

### Failover Execution

**RTO (Recovery Time Objective)**: 1-2 minutes
- DNS propagation: 0-5 minutes (can be pre-cached)
- Client reconnection: 30-60 seconds
- Message queue drain: 10-30 seconds

**RPO (Recovery Point Objective)**: <60 seconds
- Messages in-flight: ~100-200 messages
- Processed data: Zero loss (idempotent operation required)

## 3. Device Connection Management

### Connection String Strategy

**Option 1: Hardcoded Fallback (Simple)**
```csharp
string connectionString = primaryHub;
try {
    client = new DeviceClient.CreateFromConnectionString(connectionString);
    await client.OpenAsync();
} catch (Exception ex) {
    // Fallback to secondary
    connectionString = secondaryHub;
    client = new DeviceClient.CreateFromConnectionString(connectionString);
    await client.OpenAsync();
}
```

**Option 2: Smart Retry with Exponential Backoff (Recommended)**
```csharp
private async Task<DeviceClient> ConnectWithFailover()
{
    var hubs = new[] { primaryHub, secondaryHub };
    var retryPolicy = new ExponentialBackoffRetryPolicy(maxRetryAttempts: 3);
    
    foreach (var hub in hubs) {
        try {
            var client = new DeviceClient.CreateFromConnectionString(hub);
            await client.OpenAsync();
            return client;
        } catch (Exception ex) {
            _logger.LogWarning($"Failed to connect to {hub}: {ex.Message}");
            await Task.Delay(retryPolicy.NextDelay());
        }
    }
    
    throw new Exception("Failed to connect to both hubs");
}
```

**Option 3: Circuit Breaker Pattern (Enterprise)**
```csharp
public class HubConnectionPool
{
    private readonly CircuitBreaker _primaryCircuit;
    private readonly CircuitBreaker _secondaryCircuit;
    
    public async Task<DeviceClient> GetClient()
    {
        if (_primaryCircuit.IsHealthy()) {
            return await TryConnect(primaryHub, _primaryCircuit);
        }
        else if (_secondaryCircuit.IsHealthy()) {
            return await TryConnect(secondaryHub, _secondaryCircuit);
        }
        else {
            throw new AllHubsDownException();
        }
    }
}
```

## 4. Data Synchronization

### Device Twin Synchronization

**Challenge**: Device twins are hub-specific and not automatically replicated

**Solution: Shared State Backend**
```
┌────────────────────────┐
│ Azure Cosmos DB        │
│ (Global Backend)       │
│ - Device State         │
│ - Configuration        │
│ - Firmware Version     │
└────────────────────────┘
         ▲
         │ Sync
         │
    ┌────┴────┬────────────┐
    │          │            │
 Primary     Secondary    Other Hubs
 Hub         Hub
```

**Implementation**:
1. Primary hub syncs device twin to Cosmos DB
2. On failover, device reconnects to secondary hub
3. Secondary hub queries Cosmos DB for device state
4. State restored within seconds

### Message Deduplication

**Requirement**: Ensure no duplicate processing on failover

```csharp
public class DuplicateDetector
{
    private readonly IDistributedCache _cache;
    private readonly string _messageIdKey;
    
    public async Task<bool> IsProcessed(string messageId)
    {
        var key = $"msg:{messageId}";
        var processed = await _cache.GetAsync(key);
        return processed != null;
    }
    
    public async Task MarkAsProcessed(string messageId)
    {
        var key = $"msg:{messageId}";
        await _cache.SetAsync(key, new byte[] { 1 }, 
            new DistributedCacheEntryOptions {
                AbsoluteExpirationRelativeToNow = TimeSpan.FromMinutes(5)
            });
    }
}
```

## 5. Monitoring and Alerts

### Health Check Metrics

**Per-Hub Metrics**:
- D2C message ingestion rate
- Message processing latency (p50, p95, p99)
- Authentication failure rate
- Connection drop rate
- C2D message delivery rate

**Alert Thresholds**:
```
Metric                          | Threshold | Action
──────────────────────────────────────────────────────
Connection Errors               | >10%      | Page OnCall
Message Latency (p99)           | >5sec     | Warn
Authentication Failures         | >5/min    | Alert
Endpoint Health                 | Down      | Critical
Quota Utilization              | >80%      | Scale Alert
```

### Failover Metrics

- **Failover Events**: Count, timestamp, reason
- **Failover Duration**: Time to detect + time to failover
- **Message Loss**: Count of failed deliveries during failover
- **Recovery Time**: Time to restore primary and revert traffic

## 6. Failover Testing

### Test 1: Planned Failover
```bash
# 1. Notify stakeholders
# 2. Disable primary hub (via portal or CLI)
# 3. Monitor device reconnections
# 4. Verify secondary hub receiving messages
# 5. Check message latency
# 6. Enable primary hub
# 7. Monitor failback
```

### Test 2: Unplanned Failover Simulation
```bash
# 1. Inject failures on primary hub
# 2. Measure time to failover
# 3. Verify automatic recovery
# 4. Check data consistency
```

### Test 3: Data Consistency Validation
```python
def validate_failover_consistency():
    """
    Ensure no data loss during failover
    """
    messages_sent = get_messages_sent()
    messages_received = get_messages_received()
    
    assert len(messages_received) >= len(messages_sent) * 0.99
    assert check_duplicates(messages_received) == 0
    assert check_ordering_preserved(messages_received)
```

## 7. Cost Analysis

### Monthly Costs

**Dual-Hub Setup**:
- Primary Hub (iot-exp-hub): ₹2,079.87/month
- Secondary Hub (iot-exp-hub-west): ₹2,079.87/month
- **Total Hub Cost**: ₹4,159.74/month

**Additional Services** (estimated):
- Cosmos DB (state sync): ₹50-100/month
- Event Hubs (routing): ₹100-150/month
- Storage (diagnostics): ₹20-50/month

**Total Monthly**: ~₹4,350-4,460

**Cost Optimization**:
- Use Free tier for non-production failover testing
- Implement message batching to reduce message count
- Archive old logs to reduce storage costs
- Use reserved instances if multi-year commitment

## 8. Operational Runbooks

### Failover Activation Runbook

**Trigger**: Primary hub health score < 0.5 or unreachable

1. **Alert**: Page on-call engineer
2. **Assess**: Check primary hub status in portal
3. **Notify**: Inform stakeholders of failover
4. **Activate**: Update device routing (DNS or client config)
5. **Monitor**: Track message ingestion to secondary
6. **Validate**: Confirm devices reconnected
7. **Investigate**: Root cause analysis
8. **Document**: Log incident details

### Failback Procedure

**Preconditions**: Primary hub restored and healthy

1. **Prepare**: Ensure primary hub fully operational
2. **Drain**: Stop new connections to secondary
3. **Sync**: Ensure state synchronized back to primary
4. **Migrate**: Gradually move devices back (10% per minute)
5. **Monitor**: Track metrics during migration
6. **Complete**: Full migration complete
7. **Report**: Generate failover report

## 9. Security Considerations

### Connection String Security

**Never**:
- Hardcode connection strings in source code
- Log full connection strings
- Commit to version control

**Always**:
- Store in Azure Key Vault
- Use managed identities where possible
- Rotate keys every 90 days
- Use least-privilege policies

### Hub Access Control

**RBAC Roles**:
- **IoT Hub Data Contributor**: Device operations
- **IoT Hub Data Reader**: Read-only access
- **IoT Hub Owner**: Full administrative access

**Shared Access Policies**:
- Separate keys per environment (dev, staging, prod)
- Read-only keys for monitoring
- Time-limited tokens when possible

## 10. Future Enhancements

### Planned Improvements

1. **Multi-Region Expansion**
   - Add third hub in Europe region
   - Implement round-robin load balancing
   - Cross-region device twin replication

2. **Automated Failover**
   - Remove manual intervention
   - Automatic device reconnection
   - Zero-downtime migration

3. **Advanced Analytics**
   - AI-based failure prediction
   - Anomaly detection
   - Capacity forecasting

4. **Enhanced Monitoring**
   - Real-time dashboard
   - Mobile alerts
   - Predictive alerting

## References

- [Azure IoT Hub High Availability](https://docs.microsoft.com/en-us/azure/iot-hub/iot-hub-ha-dr)
- [IoT Hub Failover Guide](https://docs.microsoft.com/en-us/azure/iot-hub/iot-hub-understand-ip-address)
- [Device Reconnection Logic](https://github.com/Azure-Samples/azure-iot-samples-csharp)
- [Best Practices](https://azure.microsoft.com/en-us/blog/best-practices-for-deploying-iot-on-azure/)

## Contact

- **Maintained By**: Principal IoT Engineer
- **Last Updated**: January 11, 2026
- **Status**: Production Ready
