# Advanced Azure IoT Hub Features & Configuration

## Introduction

This guide covers advanced IoT Hub features discovered through exploration of the Azure portal, including message routing, built-in endpoints, shared access policies, and security configurations.

## 1. Message Routing Architecture

### Overview

Message routing enables intelligent distribution of device-to-cloud (D2C) messages to multiple endpoints based on matching queries.

**Key Characteristics**:
- Up to 100 custom routes per IoT Hub
- Messages matched against routing queries
- Multiple endpoints per route
- Fallback route for unmatched messages
- Messages stop flowing to built-in endpoint unless explicitly routed

### Routing Components

#### 1.1 Routes

**What it does**: Defines rules for message distribution

**Configuration**:
- Route name (unique identifier)
- Data source (device telemetry, device lifecycle events, etc.)
- Routing query (filters messages)
- Endpoint (where to send matching messages)
- Enabled/disabled status

**Example Route**:
```
Name: temperature-high
Data Source: Device Messages
Routing Query: $body.temperature > 30
Endpoint: HighTempStorageAccount
Enabled: true
```

#### 1.2 Custom Endpoints

**Supported Endpoint Types**:

1. **Event Hubs**
   - Recommended for: High volume, high throughput scenarios
   - Use case: Real-time event streaming
   - Configuration: Namespace, Event Hub name, authentication

2. **Service Bus Queues**
   - Recommended for: Guaranteed delivery, ordered processing
   - Use case: Command queuing
   - Configuration: Service Bus namespace, queue name

3. **Service Bus Topics**
   - Recommended for: Pub/sub messaging
   - Use case: Multiple subscribers, fan-out patterns
   - Configuration: Service Bus namespace, topic name

4. **Storage (Blob/Table)**
   - Recommended for: Data archival and analytics
   - Use case: Long-term storage, batch processing
   - Configuration: Storage account, container/table, batch settings

5. **Cosmos DB**
   - Recommended for: NoSQL data persistence
   - Use case: Complex queries, semi-structured data
   - Configuration: Database, collection/container

#### 1.3 Message Enrichment

**Purpose**: Automatically add metadata to messages

**Example Enrichments**:
- `$iothub.enqueuedtime` - Timestamp
- `$iothub.connection.device.id` - Device ID
- `$iothub.connectiondevice.authmethod` - Auth type
- Custom properties from device twin

## 2. Built-in Endpoints Configuration

### Event Hub-Compatible Endpoint

**Purpose**: Default endpoint for all device messages

**Key Settings**:

**Partitions**: 4 (default)
- Controls parallelism for message consumption
- Each partition can be consumed independently
- Higher partitions = better throughput
- Trade-off: More complex consumer coordination

**Event Hub-Compatible Name**: `{IoT-Hub-Name}` (e.g., iot-exp-hub)
- Used for connecting Event Hubs consumers
- Enables compatibility with Event Hubs SDKs

**Retention Time**: 1 day (default, configurable 1-7 days)
- How long messages are retained if not consumed
- Allows consumers to re-read messages within this window

**Consumer Groups**: $Default (standard, can add more)
- Isolate message consumption
- Multiple consumers can have different read positions
- Recommended: One consumer group per consuming application

### Cloud-to-Device (C2D) Messaging Configuration

**Default TTL (Time-to-Live)**: 1 hour
- How long cloud-to-device messages wait in queue
- After TTL expires, message is dropped
- Configurable: 1-48 hours

**Feedback Retention Time**: 1 hour
- How long feedback about C2D message delivery is retained
- Allows backend to check delivery status
- Configurable: 1-48 hours

**Maximum Delivery Count**: 10 attempts
- Number of times IoT Hub attempts to deliver C2D message
- After max attempts, message is moved to dead letter queue
- Configurable: 1-100 attempts

## 3. Security & Authentication

### Shared Access Policies

**Purpose**: Control permissions for accessing IoT Hub

**Default Policies**:

| Policy Name | Permissions | Use Case |
|-------------|-------------|----------|
| **iothubowner** | Registry R/W, Service Connect, Device Connect | Administrative access, management |
| **service** | Service Connect | Backend services, management plane |
| **device** | Device Connect | Device connections |
| **registryRead** | Registry Read | Read-only access to device registry |
| **registryReadWrite** | Registry R/W | Full device registry management |

**Each Policy Includes**:
- Primary Connection String
- Secondary Connection String
- Primary Key
- Secondary Key

**Best Practice**: Use least-privilege principle
- Devices: Use "device" policy only
- Backend services: Use "service" policy
- Never expose "iothubowner" credentials

### Authentication Methods

#### 3.1 Symmetric Key (Shared Access Key)

**How it works**:
- Device has primary + secondary key
- Creates SAS token from connection string
- Token expires and must be refreshed

**Pros**:
- Simple to implement
- Fast performance
- Suitable for development

**Cons**:
- Key compromise = device compromise
- Must securely store keys
- Key rotation can be complex

#### 3.2 X.509 Certificates

**How it works**:
- Device presents certificate for mutual TLS
- Azure verifies certificate chain
- Mutual authentication

**Pros**:
- Higher security
- Certificate-based revocation
- Production recommended
- Supports hierarchical PKI

**Cons**:
- More complex to manage
- Certificate renewal cycles
- Certificate storage requirements

### Minimum TLS Requirements

- **TLS 1.2 minimum** (configurable in built-in endpoints)
- MQTT: Port 8883
- AMQP: Port 5671
- HTTPS: Port 443

## 4. Hub Configuration Settings

### File Upload

**Purpose**: Enable devices to upload large files directly to storage

**Configuration**:
- Storage account connection string
- Blob container
- File size limits
- TTL for SAS URIs

**Benefits**:
- Offload from message path
- Support for binary files
- Direct to storage transfers

### Failover

**Purpose**: Geographic redundancy and disaster recovery

**Options**:
- Manual failover to secondary region
- Automatic failover (with higher tier)

**Considerations**:
- Device reconnection required
- Connection string may need update
- Some data loss possible with automatic failover

### Pricing & Scale

**Tiers**:

**Free Tier**:
- 500 devices
- 8,000 messages/day
- Development/testing only

**Standard Tier**:
- Unlimited devices
- Messages based on units
- Production recommended

**Basic Tier**:
- Limited features
- Lower cost option

**Per-Unit Pricing**:
- Each unit handles specific message throughput
- 400,000 - 6,000,000 messages/day per unit
- Auto-scale available

## 5. Monitoring & Diagnostics

### Key Metrics to Track

1. **Message Volume**: D2C and C2D message counts
2. **Connection State**: Devices connected vs disconnected
3. **Message Latency**: Time from device send to cloud receive
4. **Failed Operations**: Authentication failures, routing failures
5. **Quota Usage**: Against Azure subscription limits

### Common Issues & Solutions

| Issue | Cause | Solution |
|-------|-------|----------|
| Messages not arriving | Routing misconfiguration | Verify routing queries and endpoints |
| High latency | Insufficient partitions | Increase partition count (up to 32) |
| Auth failures | Invalid credentials | Verify connection strings and keys |
| Device disconnections | Network issues or stale tokens | Implement reconnection logic with backoff |
| Quota exceeded | Too many devices/messages | Scale to higher tier or units |

## 6. Best Practices for Production

### Security
1. ✅ Use X.509 certificates for production devices
2. ✅ Implement certificate rotation
3. ✅ Use device-scoped keys, not hub-wide keys
4. ✅ Monitor failed authentication attempts
5. ✅ Rotate Shared Access Keys regularly
6. ✅ Use private endpoints for hybrid scenarios

### Reliability
1. ✅ Implement exponential backoff for reconnections
2. ✅ Use MQTT with QoS 1 for at-least-once delivery
3. ✅ Set appropriate message TTL values
4. ✅ Monitor connection state via device twin
5. ✅ Implement dead letter queue handling

### Performance
1. ✅ Use multiple partitions (4-32) for high throughput
2. ✅ Route non-critical messages to async endpoints
3. ✅ Use message compression when possible
4. ✅ Batch operations where applicable
5. ✅ Monitor message latency metrics

### Operations
1. ✅ Set up diagnostic logging
2. ✅ Configure alerts for key metrics
3. ✅ Implement proper error handling
4. ✅ Document your routing configuration
5. ✅ Test failover scenarios

## 7. Advanced Scenarios

### Scenario 1: Real-Time Telemetry Processing

**Architecture**:
```
Device → IoT Hub → Event Hubs → Stream Analytics → Power BI
                                              ↓
                                        Alerting Service
```

**Configuration**:
- Route to Event Hubs endpoint
- Query: All telemetry (`true`)
- Enable stream processing

### Scenario 2: Data Archival with Filtering

**Architecture**:
```
Device → IoT Hub → Storage (Blobs)
              ↓
           Routes (filter high-value data)
```

**Configuration**:
- Route high-priority to premium storage
- Route low-priority to cold storage
- Set batch size and frequency

### Scenario 3: Multi-Tenant Isolation

**Approach**:
- Use device twin `$metadata` for tenant identification
- Route based on device properties
- Separate storage accounts per tenant
- Use consumer groups for isolation

## 8. Troubleshooting Guide

### Debug Checklist

- [ ] Device authentication working (check connection state)
- [ ] Routing query syntax correct (validate in portal)
- [ ] Endpoint connectivity verified (test connection)
- [ ] Message format matches query expectations
- [ ] Retention policies adequate for consumers
- [ ] Partition count sufficient for throughput
- [ ] TLS version compatible with device SDK

### Debugging Tools

1. **Azure CLI**: 
   ```bash
   az iot hub monitor-events --hub-name iot-exp-hub
   ```

2. **IoT Explorer**: Visual device interaction
3. **Application Insights**: Integrated monitoring
4. **Message Tracing**: Enable diagnostic logs

## References

- [Azure IoT Hub Documentation](https://docs.microsoft.com/en-us/azure/iot-hub/)
- [Message Routing Guide](https://docs.microsoft.com/en-us/azure/iot-hub/iot-hub-devguide-messages-d2c)
- [Security Best Practices](https://docs.microsoft.com/en-us/azure/iot-hub/iot-hub-security-overview)
- [Quotas and Limits](https://docs.microsoft.com/en-us/azure/iot-hub/iot-hub-devguide-quotas-throttling)
