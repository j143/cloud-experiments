# Azure IoT Hub: Network Security and Operations Guide

## Overview
This guide covers the network security, operational aspects, and system configuration of Azure IoT Hub, including networking policies, certificate management, pricing tiers, and resource properties.

## 1. Network Access and Security

### Public Network Access
The IoT Hub supports configurable public network access policies:
- **All Networks**: Default setting - allows access from all public networks
- **Selected IP Ranges**: Restrict access to specific IP addresses or CIDR ranges
- **Disabled**: Completely disable public endpoint access (requires private endpoints)

### Current Configuration
- **Setting**: All networks
- **Status**: Public endpoint accessible from internet
- **Requirement**: Proper authorization still required via shared access policies

## 2. Network Security Features

### Key Features
- **Min TLS Version**: 1.2 (enforced)
- **Disable Local Auth**: false (local authentication enabled)
- **Disable Device SAS**: Not configured
- **Disable Module SAS**: Not configured
- **Restrict Outbound Network Access**: Not restricted
- **Public Endpoint**: iot-exp-hub.azure-devices.net

### Networking Options
- **Private Endpoints**: Supported for private connectivity
- **Virtual Network Integration**: Can restrict to VNet via private link
- **IP Filtering**: Can configure IP whitelist/blacklist rules
- **Network Rules**: Default action configurable for built-in endpoints

## 3. Certificate Management

### Certificate Features
- Upload and manage X.509 certificates for device authentication
- Automatic certificate verification available
- Support for certificate chains
- Thumbprint-based certificate tracking
- Expiration monitoring

### Certificate Upload Process
1. Navigate to Certificates section
2. Click "Add" to upload new certificate
3. Select PEM or DER format certificate file
4. Verify certificate (optional - can be done automatically)
5. Certificate becomes available for device authentication

## 4. Pricing and Capacity

### Standard Tier Configuration
- **Tier**: Standard (S1)
- **Daily Message Limit**: 400,000 messages
- **Message Size**: 4 KB per message
- **Monthly Cost**: ₹2,079.87 (approximately)

### Features Available
- Device-to-Cloud (D2C) Messaging
- Cloud-to-Device (C2D) Commands
- Device Twin Management
- IoT Edge Support
- Message Routing
- File Upload
- Blob Storage Integration

### Scaling Considerations
- Standard tier scales up to support higher message volumes
- Upgrade path: S1 → S2 → S3
- Each tier doubles message throughput
- Billing based on daily message count

## 5. System Properties and Configuration

### Hub Essentials
- **Name**: iot-exp-hub
- **Type**: Microsoft.Devices/IotHubs
- **Region**: East US (eastus)
- **Resource Group**: iot-exp-rg
- **Subscription**: Standard Azure subscription

### Hub Status
- **Provisioning State**: Succeeded
- **State**: Active
- **Features**: RootCertificateV2

### Endpoints
- **Built-in Event Hub**: Compatible with Event Hubs protocol
- **Device Telemetry Endpoint**: AMQP, MQTT, HTTPS protocols
- **Cloud-to-Device Endpoint**: HTTPS for C2D messages
- **Device Streaming**: Supported (currently ---)

## 6. Cloud-to-Device Configuration

### C2D Message Settings
- **Max Delivery Count**: 10 (maximum retry attempts)
- **Default TTL**: PT1H (1 hour in ISO8601 format)
- **Feedback Queue**: Available for delivery confirmations
- **Protocol Support**: HTTPS, AMQP, MQTT

### Use Cases
- Send commands to devices
- Firmware updates
- Configuration changes
- Remote control operations

## 7. Data Residency and Compliance

### Configuration
- **Enable Data Residency**: false (data may be replicated across regions)
- **Location**: Data stored in East US region
- **Encryption**: Managed by Azure (default)
- **Key Vault**: Not configured (using default encryption)

## 8. Authorization and Access Control

### Shared Access Policies
Pre-configured policies available:
1. **iothubowner**
   - Registry Read, Registry Write
   - Service Connect, Device Connect
   - Full administrative access

2. **service**
   - Service Connect permission only
   - Used for backend services

3. **device**
   - Device Connect permission only
   - Limited to device operations

4. **registryRead**
   - Registry Read permission only
   - Device registry query access

5. **registryReadWrite**
   - Registry Read and Write permissions
   - Device registration and management

## 9. File Upload Configuration

### Features
- **Enable File Upload Notifications**: false (not enabled)
- **Storage Account**: Can be configured
- **SAS Token TTL**: Configurable
- **Default TTL**: 1 hour

### Use Cases
- Large telemetry data uploads
- Device logs and diagnostics
- Firmware update packages
- Binary data transfers

## 10. Monitoring and Operational Insights

### Available Monitoring Options
- **Activity Log**: Track all operations on the IoT Hub
- **Metrics**: Monitor message throughput, latency, connections
- **Diagnostic Logs**: Enable logging for devices, connections, operations
- **Event Grid Integration**: React to IoT Hub events
- **Application Insights**: Integrate for application monitoring

## 11. Best Practices

### Security
- Use managed identities where possible
- Rotate access keys regularly
- Implement least-privilege access policies
- Use TLS 1.2 or higher for all connections
- Enable IP filtering for restricted access
- Regularly audit certificate validity

### Performance
- Monitor daily message consumption
- Plan capacity upgrades based on growth trends
- Use message routing to distribute load
- Implement message batching to reduce message count

### Operations
- Set up monitoring and alerting
- Create automated backup procedures
- Document all configuration changes
- Test disaster recovery procedures
- Regular security audits

## 12. Troubleshooting Common Issues

### Connectivity Issues
- Check public network access settings
- Verify IP whitelisting rules
- Confirm certificate validity and format
- Check TLS version compatibility

### Message Delivery
- Verify message size (max 4 KB)
- Check daily message quota
- Monitor C2D delivery count settings
- Review message routing configurations

### Authentication Failures
- Verify shared access policy permissions
- Check connection string format
- Validate device credentials
- Review certificate chain configuration

## References
- Azure IoT Hub Documentation: https://docs.microsoft.com/en-us/azure/iot-hub/
- IoT Hub Pricing: https://azure.microsoft.com/en-us/pricing/details/iot-hub/
- Security Best Practices: https://docs.microsoft.com/en-us/azure/iot-hub/iot-hub-security-best-practices
