# Azure IoT Hub: Message Routing and IoT Edge Guide

## Overview
This guide covers advanced message routing capabilities and IoT Edge integration for Azure IoT Hub, including data processing, endpoint configuration, and edge computing scenarios.

## 1. Message Routing Architecture

### Routing Fundamentals
Message routing enables intelligent distribution of telemetry data to multiple endpoints based on matching criteria:
- **Data Source**: Which messages to route (Device Telemetry, Device Twin Updates, Lifecycle Events)
- **Routing Query**: Conditions based on message properties and body content
- **Endpoint**: Where to send the matched messages
- **Fallback Route**: Default destination for non-matching messages

### Key Concepts
1. **Routes**: Up to 100 routes can be configured per IoT Hub
2. **Endpoints**: Up to 10 custom endpoints per hub
3. **Query Language**: SQL-like syntax for message filtering
4. **Message Enrichment**: Add dynamic properties before routing
5. **Fallback Behavior**: Automatic routing to built-in endpoint if no match

## 2. Custom Endpoints

### Supported Endpoint Types

#### a) Azure Blob Storage
**Use Cases**:
- Archive telemetry data for long-term storage
- Store large binary files from devices
- Create data lake for analytics

**Configuration**:
- Storage account name
- Container name
- File format (AVRO, JSON)
- Batch size and timeout settings

#### b) Event Hubs
**Use Cases**:
- Real-time stream processing with Azure Stream Analytics
- Integration with Apache Kafka ecosystems
- Multi-consumer scenarios

**Configuration**:
- Event Hub namespace and name
- Shared access policy
- Partition key selection

#### c) Service Bus Queue
**Use Cases**:
- Reliable message delivery guarantees
- Dead-letter queue handling
- Message deferral and scheduling

**Configuration**:
- Service Bus namespace and queue name
- Authorization rule
- Message session enable/disable

#### d) Service Bus Topic
**Use Cases**:
- Pub/Sub messaging pattern
- Multiple subscribers to same data
- Topic-based filtering

**Configuration**:
- Service Bus namespace and topic name
- Session requirements
- Subscription configuration

#### e) Cosmos DB
**Use Cases**:
- NoSQL database storage
- Real-time analytics queries
- Multi-region replication

**Configuration**:
- Cosmos DB account endpoint
- Database and collection names
- Partition key strategy
- Write-throughput (RUs)

## 3. Routing Query Examples

### Basic Examples
```sql
-- Route all messages
SELECT * 
FROM messages

-- Route based on message property
SELECT *
FROM messages
WHERE properties.system.iothub-enqueuedtime < @now - INTERVAL '1' MINUTE

-- Route based on device property
SELECT *
FROM messages
WHERE $connectionModuleId IS NULL
```

### Advanced Filtering
```sql
-- Route high-priority messages
SELECT *
FROM messages
WHERE properties.user.priority = 'high'

-- Route messages from specific device
SELECT *
FROM messages
WHERE $deviceId = 'sensor-01'

-- Route based on message content
SELECT *
FROM messages
WHERE temperature > 30 OR humidity < 40
```

## 4. Message Enrichment

### Enrichment Capabilities
Add up to 10 custom properties to messages before routing:

**Static Value**:
```
Property Name: location
Property Value: factory-floor-1
Endpoint(s): Storage
```

**Dynamic Value from System**:
```
Property Name: hub_name
Property Value: ${{iotHubname}}
Endpoint(s): All
```

**Dynamic Value from Device Twin**:
```
Property Name: device_version
Property Value: ${{twin.properties.desired.firmwareVersion}}
Endpoint(s): Event Hubs

Property Name: device_location
Property Value: ${{twin.tags.location}}
Endpoint(s): Cosmos DB
```

### Use Cases
- Add device metadata for downstream processing
- Include timestamp and correlation IDs
- Add device location and facility information
- Include desired configuration version

## 5. IoT Edge Overview

### What is IoT Edge?
Azure IoT Edge brings cloud intelligence to edge devices for:
- **Local Processing**: Analyze data on-premises
- **Reduced Bandwidth**: Filter data before cloud transmission
- **Offline Capability**: Continue operation without cloud connection
- **Real-time Response**: Act on data immediately

### IoT Edge Architecture
```
[Device Sensors] 
    ↓
[IoT Edge Runtime]
    ├─ Edge Agent
    ├─ Edge Hub  
    └─ Custom Modules (Docker containers)
    ↓
[Azure IoT Hub / Local Storage]
    ↓
[Cloud Services]
```

## 6. IoT Edge Modules

### Module Types

**1. Pre-built Modules**
- Azure ML Module: Run machine learning models
- Stream Analytics: Real-time analytics
- SQL Server: Local database
- Temperature Sensor Simulator

**2. Custom Modules**
- Developed in any language (Python, Node.js, C#, Java)
- Containerized using Docker
- Deployed from Container Registry

### Module Communication
```
Module A (Input) → Edge Hub → Module B (Output)
         ↓
      Local Storage
         ↓
   IoT Hub (Cloud)
```

## 7. IoT Edge Deployment

### Deployment Manifest
Defines modules, routes, and properties:

```json
{
  "modulesContent": {
    "$edgeAgent": {
      "type": "docker",
      "settings": {
        "image": "mcr.microsoft.com/azureiotedge-agent:latest"
      }
    },
    "$edgeHub": {
      "type": "docker",
      "routes": {
        "sensorToAzure": "FROM /messages/modules/sensor/outputs/readings INTO $upstream"
      }
    }
  }
}
```

### Deployment Strategies

**1. Single Device Deployment**
- Target specific IoT Edge device
- Useful for testing and development

**2. Layered Deployment**
- Apply multiple deployments to single device
- Hierarchy of configurations
- Override capability

**3. Automatic Deployment**
- Target devices based on tags
- Device-level or module-level conditions
- Automatic updates

## 8. Edge Computing Scenarios

### Scenario 1: Real-time Analytics
```
Sensors → Edge Device → Stream Analytics → Alerts
                    ↓
                 Storage → Azure Synapse
```

### Scenario 2: Predictive Maintenance
```
Machine Data → IoT Edge → ML Model → Anomaly Detection
                                        ↓
                                    Maintenance Alert
```

### Scenario 3: Data Filtering and Aggregation
```
Multiple Sensors → Edge Device → Aggregate/Filter → Cloud
(High frequency)              (Reduced data volume)
```

## 9. Best Practices

### Message Routing
- Use specific routing queries (avoid SELECT *)
- Monitor endpoint availability and performance
- Plan endpoint capacity based on message volume
- Test routing queries with sample data
- Document routing logic and purposes
- Use enrichment for operational metadata
- Implement circuit breaker for endpoint failures

### IoT Edge
- Version control edge configurations
- Use device tags for deployment targeting
- Monitor edge device connectivity
- Implement local data retention policies
- Test module updates before production
- Use environment variables for configuration
- Monitor container resource usage
- Implement logging and diagnostics

## 10. Troubleshooting

### Message Not Routing
1. Verify endpoint credentials and connectivity
2. Check routing query syntax
3. Confirm message source selection
4. Test query with sample messages
5. Check fallback route configuration
6. Verify endpoint authentication type

### Edge Device Issues
1. Check device connectivity status
2. Verify module image availability
3. Check module deployment status
4. Review edge agent logs
5. Verify network connectivity between modules
6. Check storage and memory resources

## 11. Monitoring and Alerting

### Key Metrics
- Messages routed per endpoint
- Endpoint latency and failures
- Edge device connectivity status
- Module health and resource usage
- Data processing throughput

### Monitoring Setup
1. Enable diagnostic logs
2. Set up Application Insights integration
3. Configure Azure Monitor alerts
4. Create custom dashboards
5. Implement telemetry from modules

## References
- Azure IoT Hub Message Routing: https://docs.microsoft.com/en-us/azure/iot-hub/iot-hub-devguide-messages-d2c
- Azure IoT Edge Documentation: https://docs.microsoft.com/en-us/azure/iot-edge/
- IoT Edge Modules: https://docs.microsoft.com/en-us/azure/iot-edge/iot-edge-modules
- Routing Query Syntax: https://docs.microsoft.com/en-us/azure/iot-hub/iot-hub-devguide-routing-query-syntax
