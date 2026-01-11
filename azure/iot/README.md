# Azure IoT Hub Experiment

## Overview
End-to-end IoT system built on Azure using IoT Hub, connecting simulated devices for telemetry collection and cloud-to-device messaging.

## Architecture

### Components
- **Azure IoT Hub**: Central hub for device communication (iot-exp-hub, Standard tier)
- **IoT Devices**: Simulated devices using Azure IoT SDK (e.g., sensor-01)
- **Message Routing**: Routes telemetry to downstream services
- **Device Twins**: For managing device state and cloud-to-device commands

### Flow
```
Device (sensor-01)
    |
    v (MQTT/AMQP)
Azure IoT Hub (iot-exp-hub)
    |
    +---> Device Telemetry (D2C)
    +---> Device Twins (state)
    +---> Cloud Commands (C2D)
```

## Setup

### Prerequisites
- Azure subscription
- Python 3.8+
- Azure IoT SDK for Python

### Installation

```bash
pip install azure-iot-device
```

### Configuration

1. Create IoT Hub in Azure Portal
2. Register devices in Device Management
3. Get connection strings from device properties
4. Update config files with connection details

## Files

- `README.md` - This file
- `device_simulator.py` - Python script for simulated device
- `device_config.json` - Device connection configuration
- `requirements.txt` - Python dependencies

## Running the Experiment

### Start Device Simulator

```bash
python device_simulator.py
```

The simulator will:
- Connect to IoT Hub
- Send telemetry messages at regular intervals
- Listen for cloud-to-device commands
- Update device twin properties

## Key Learnings

1. **Device Identity**: Each device needs unique credentials
2. **Message Patterns**: D2C (telemetry) vs C2D (commands) messaging
3. **Device Twins**: Persistent device state in the cloud
4. **Connectivity**: Resilient connections with retry logic
5. **Security**: Shared key authentication, connection string management

## Next Steps

1. Create storage account for telemetry archiving
2. Set up Event Hubs for real-time streaming
3. Configure message routing rules
4. Implement telemetry consumer dashboard
5. Add device-to-cloud processing rules
6. Create ARM templates for IaC deployment
