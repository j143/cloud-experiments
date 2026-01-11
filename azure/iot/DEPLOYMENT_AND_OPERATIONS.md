# Azure IoT Hub: Deployment and Operations Guide

## Overview
This guide covers best practices for deploying, managing, and operating Azure IoT Hub solutions in production environments.

## 1. Pre-Deployment Planning

### Capacity Planning

**Message Volume Assessment**
- Estimate daily/hourly message count from devices
- Account for peak periods (2-3x average)
- Plan for growth (scale for next 12-24 months)
- Consider multiple event types (telemetry, events, alerts)

**Tier Selection**
```
Daily Messages    | Recommended Tier
< 400K           | Free (development only)
400K - 6M        | Standard S1
6M - 40M         | Standard S2
40M+             | Standard S3
```

**Scalability Considerations**
- Vertical scaling: Upgrade tier (simple but downtime)
- Horizontal scaling: Multiple hubs with routing
- Hybrid approach: Combine both strategies

### Resource Requirements

**Storage Accounts**
- Archive endpoint storage (1 GB minimum)
- File upload storage if needed
- Diagnostic logs storage (retention policy)

**Event Hub/Service Bus**
- For message routing endpoints
- Throughput units based on expected load
- Consumer groups for multiple readers

**Cosmos DB**
- If using for data storage
- Throughput Units (RUs) calculation
- Partition key strategy

## 2. Security Configuration

### Network Security

**IP Whitelisting**
```json
{
  "ipFilterRules": [
    {
      "filterName": "allowCorp",
      "action": "Accept",
      "ipMask": "203.0.113.0/24"
    },
    {
      "filterName": "blockMalicious",
      "action": "Reject",
      "ipMask": "198.51.100.0/24"
    }
  ]
}
```

**Private Endpoints**
- Restrict to VNet-only access
- Disable public endpoint
- Use Azure Private Link

### Authentication Strategy

**Device Authentication**
1. **Symmetric Key (SAS)**
   - Easy setup, suitable for most devices
   - Rotate keys every 90 days
   - Use separate keys per environment

2. **X.509 Certificates**
   - Better security for high-value devices
   - Certificate rotation procedures
   - Certificate revocation lists

3. **Managed Identity**
   - For Azure services
   - Service principal recommended

### Access Control (RBAC)

**Role Assignments**
```
Role                          | Who              | Scope
IoT Hub Data Contributor      | Backend apps     | Hub level
IoT Hub Data Reader          | Monitoring       | Hub level
IoT Hub Owner                | Admin            | Hub level
Access Policy Contributor     | DevOps           | Hub level
```

## 3. Monitoring and Observability

### Diagnostic Logs Setup

**Log Categories**
- Connections: Device connect/disconnect
- Device Telemetry: All D2C messages
- C2D Commands: Cloud-to-device messages
- File Uploads: File operation logs
- Routes: Message routing events
- Twin Operations: Device twin changes

**Retention Policy**
```json
{
  "diagnosticSettings": {
    "name": "SendToLogAnalytics",
    "logs": [
      {
        "category": "Connections",
        "enabled": true,
        "retentionPolicy": {
          "enabled": true,
          "days": 90
        }
      }
    ]
  }
}
```

### Key Metrics to Monitor

**Device Metrics**
- Active device connections
- Device authentication failures
- C2D message delivery rate
- Twin update latency

**Message Metrics**
- D2C ingress rate
- D2C messages sent
- Routing success rate
- Message latency by endpoint

**Hub Health**
- Throttling events
- Quota utilization
- Endpoint health status
- Command execution time

### Alert Configuration

**Critical Alerts**
```
Condition                          | Threshold | Action
Device Auth Failures              | > 10/min  | Page on-call
Throttling Events                 | Any       | Immediate alert
High Message Latency              | > 5 sec   | Escalate
Endpoint Failures                 | > 3       | Investigate
Quota Utilization                 | > 80%     | Plan upgrade
```

## 4. Device Management

### Device Lifecycle

**Provisioning**
1. Device registration in IoT Hub
2. Certificate upload (if using X.509)
3. Connection string provisioning
4. Initial configuration via twin

**Operation**
1. Regular health checks
2. Message validation
3. Connection monitoring
4. Performance tracking

**Decommissioning**
1. Disable device
2. Archive device data
3. Remove from hub
4. Clean up associated resources

### Device Twin Best Practices

**Structure Organization**
```json
{
  "properties": {
    "desired": {
      "firmware": {
        "version": "2.1.0",
        "updateTime": "2024-01-15T10:30:00Z"
      },
      "telemetry": {
        "interval": 60,
        "enabled": true
      }
    },
    "reported": {
      "firmware": {
        "version": "2.0.5",
        "lastUpdate": "2024-01-10T14:20:00Z"
      },
      "status": "operational"
    }
  }
}
```

**Configuration Management**
- Use desired properties for configuration
- Version track all updates
- Implement rollback mechanism
- Monitor reported state drift

## 5. Data Pipeline Deployment

### Message Routing Setup

**Routing Configuration Steps**
1. Create endpoints (Storage, Event Hubs, etc.)
2. Define routing queries
3. Test with sample messages
4. Monitor routing metrics
5. Set fallback route

**Testing Before Production**
```
Phase 1: Local testing with simulator
Phase 2: Test hub with small subset of devices
Phase 3: Gradual rollout (10% → 50% → 100%)
Phase 4: Production monitoring
```

### Data Archival Strategy

**Hot Storage (Last 7 days)**
- Azure Data Explorer
- Real-time analytics
- Quick access

**Warm Storage (Last 90 days)**
- Azure Blob Storage
- Standard tier
- Monthly trends analysis

**Cold Storage (Archive)**
- Blob Archive tier
- Compliance/audit purposes
- Rare access

## 6. High Availability and Disaster Recovery

### High Availability Configuration

**Multi-Region Setup**
```
Primary Region (East US)
  ↓
  [IoT Hub] → [Routing] → [Event Hubs]
     ↓
Secondary Region (West US)  
  ↓
  [IoT Hub] → [Routing] → [Event Hubs]
     ↓
  [Failover]
```

**Failover Strategy**
- DNS-based routing to failover hub
- Device connection string with fallback
- Shared event hub consumer groups
- Message deduplication logic

### Disaster Recovery Plan

**Recovery Time Objective (RTO)**
- Manual failover: 5-10 minutes
- Automated failover: 1-2 minutes
- DNS propagation: 5-10 minutes

**Recovery Point Objective (RPO)**
- In-flight messages: < 60 seconds
- Processed data: No loss (idempotent operations)
- Configuration: Replicated in real-time

**Backup Procedures**
1. Export device registry regularly
2. Backup routing configurations
3. Archive important messages
4. Document critical settings

## 7. Cost Optimization

### Message Optimization

**Reduce Message Count**
- Increase telemetry interval
- Filter non-critical data at edge
- Batch messages
- Compress payloads

**Example: Optimization Savings**
```
Before: 10 devices × 100 msg/day × 30 days = 30,000 messages
After: 10 devices × 50 msg/day × 30 days = 15,000 messages

Monthly Savings: 50% reduction in message costs
```

### Resource Rightsizing

**Tier Analysis**
- Review actual vs. provisioned capacity
- Downgrade unused features
- Consolidate multiple hubs if possible
- Use free tier for development/testing

### Storage Optimization

**Tiering Strategy**
- Hot storage: 7-30 days
- Cool storage: 31-90 days
- Archive: 91+ days

## 8. Operational Procedures

### Maintenance Windows

**Planned Maintenance**
- Communicate schedule 2 weeks prior
- Perform during low-traffic periods
- Implement graceful degradation
- Have rollback procedure ready

**Certificate Renewal**
- Schedule 30 days before expiration
- Test new certificate before activation
- Coordinate with device updates
- Plan for devices offline during update

### Incident Response

**Troubleshooting Flowchart**
```
Symptom: Devices not connecting
  ↓
Check 1: Device credentials valid? 
  → No: Regenerate credentials
  → Yes: Continue
  ↓
Check 2: Network connectivity?
  → No: Check firewall/VPN
  → Yes: Continue
  ↓
Check 3: Hub throttling?
  → Yes: Scale up or distribute load
  → No: Check device logs
```

## 9. Compliance and Auditing

### Audit Logging

**Enable Activity Log**
- Track all administrative changes
- Monitor policy modifications
- Audit user access
- 90-day retention minimum

**Data Protection**
- Encryption at rest (default)
- TLS 1.2 minimum for transit
- Key rotation: 90-day minimum
- Separate keys per environment

### Compliance Checklist

- [ ] Authentication configured
- [ ] TLS 1.2+ enforced
- [ ] IP filtering configured
- [ ] Diagnostic logs enabled
- [ ] Alerts configured
- [ ] Backup procedure documented
- [ ] Disaster recovery tested
- [ ] Access control review completed
- [ ] Encryption keys rotated
- [ ] Compliance audit completed

## 10. Scaling Strategies

### Vertical Scaling (Tier Upgrade)

**Pros**
- Simple configuration
- Immediate capacity increase
- No application changes

**Cons**
- Potential brief downtime
- May over-provision

### Horizontal Scaling (Multiple Hubs)

**Pros**
- Unlimited scalability
- Resilience
- Workload isolation

**Cons**
- Complex device provisioning
- Routing complexity
- Cost of multiple hubs

**Partition Strategy**
```
By Location:     Hub per region
By Customer:     Hub per tenant
By Device Type:  Hub per category
Hybrid:          Combination approach
```

## 11. Documentation and Knowledge Base

### Critical Documentation

1. **Architecture Diagrams**
   - Hub topology
   - Data flow
   - Integration points

2. **Operational Procedures**
   - Backup/restore
   - Failover activation
   - Scaling procedures

3. **Runbooks**
   - Common issues and solutions
   - Step-by-step troubleshooting
   - Escalation procedures

4. **Device Configuration**
   - Connection string management
   - Certificate deployment
   - Configuration templates

## References
- Azure IoT Hub Documentation: https://docs.microsoft.com/en-us/azure/iot-hub/
- Best Practices Guide: https://docs.microsoft.com/en-us/azure/iot-hub/iot-hub-reliability-features-in-sdks
- Disaster Recovery: https://docs.microsoft.com/en-us/azure/iot-hub/iot-hub-ha-dr
- Azure Well-Architected Framework: https://docs.microsoft.com/en-us/azure/architecture/framework/
