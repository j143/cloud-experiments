# IronClad-DB Deployment Guide

## Overview
This guide covers deploying IronClad-DB to Microsoft Azure using Docker containers and Azure Container Instances (ACI).

## Prerequisites
- Azure subscription with resource group: `ironclad-rg`
- Azure Storage account: `ironcladstor` (Page Blobs)
- Docker CLI or Azure Container Registry (ACR)
- Azure CLI installed

## Architecture
```
┌─────────────────────────────────┐
│   IronClad-DB Application       │
│  (Rust, Multi-stage Container)  │
└──────────────┬──────────────────┘
               │
     ┌─────────▼─────────┐
     │  Azure Container  │
     │   Instance (ACI)  │
     └─────────┬─────────┘
               │
     ┌─────────▼──────────────┐
     │  Azure Storage Account │
     │   (Page Blobs: 4KB)    │
     └────────────────────────┘
```

## Deployment Methods

### Method 1: Azure Container Instances (Recommended)

#### Step 1: Build Docker Image
```bash
docker build -t ironclad-db:latest .
docker build -t ironclad-db:v1.0 .
```

#### Step 2: Create Azure Container Registry (if needed)
```bash
az acr create \
  --resource-group ironclad-rg \
  --name ironcladadr \
  --sku Basic
```

#### Step 3: Login to ACR
```bash
az acr login --name ironcladadr
```

#### Step 4: Tag and Push Image
```bash
REGISTRY_URL=ironcladadr.azurecr.io

docker tag ironclad-db:latest $REGISTRY_URL/ironclad-db:latest
docker tag ironclad-db:v1.0 $REGISTRY_URL/ironclad-db:v1.0

docker push $REGISTRY_URL/ironclad-db:latest
docker push $REGISTRY_URL/ironclad-db:v1.0
```

#### Step 5: Deploy to ACI
```bash
az container create \
  --resource-group ironclad-rg \
  --name ironclad-db-instance \
  --image ironcladadr.azurecr.io/ironclad-db:latest \
  --registry-login-server ironcladadr.azurecr.io \
  --registry-username <acr-username> \
  --registry-password <acr-password> \
  --cpu 2 \
  --memory 4 \
  --environment-variables \
    AZURE_STORAGE_ACCOUNT=ironcladstor \
    AZURE_STORAGE_KEY=<storage-key> \
  --ports 8080 \
  --dns-name-label ironclad-db
```

#### Step 6: Verify Deployment
```bash
az container show \
  --resource-group ironclad-rg \
  --name ironclad-db-instance \
  --query ipAddress.fqdn
```

### Method 2: Azure Container Apps

#### Create environment
```bash
az containerapp env create \
  --name ironclad-env \
  --resource-group ironclad-rg \
  --location southindia
```

#### Deploy app
```bash
az containerapp create \
  --name ironclad-db \
  --resource-group ironclad-rg \
  --environment ironclad-env \
  --image ironcladadr.azurecr.io/ironclad-db:latest \
  --target-port 8080 \
  --ingress external \
  --min-replicas 1 \
  --max-replicas 3
```

## Environment Variables

| Variable | Description | Example |
|----------|-------------|----------|
| `AZURE_STORAGE_ACCOUNT` | Storage account name | `ironcladstor` |
| `AZURE_STORAGE_KEY` | Storage account key | `<key>` |
| `RUST_LOG` | Log level | `debug`, `info`, `warn` |
| `PORT` | HTTP port | `8080` |

## Configuration Files

### docker-compose.yml (Optional)
```yaml
version: '3.9'
services:
  ironclad-db:
    build: .
    ports:
      - "8080:8080"
    environment:
      AZURE_STORAGE_ACCOUNT: ironcladstor
      RUST_LOG: info
    volumes:
      - ./data:/app/data
```

## Monitoring

### View Container Logs
```bash
az container logs \
  --resource-group ironclad-rg \
  --name ironclad-db-instance
```

### Monitor Container Metrics
```bash
az container show \
  --resource-group ironclad-rg \
  --name ironclad-db-instance
```

## Health Checks

The application exposes a health endpoint at `/health` (port 8080):

```bash
curl http://<container-ip>:8080/health
```

## Cleanup

```bash
# Delete container instance
az container delete \
  --resource-group ironclad-rg \
  --name ironclad-db-instance \
  --yes

# Delete ACR (if not needed)
az acr delete \
  --resource-group ironclad-rg \
  --name ironcladadr
```

## Cost Estimation

- **ACI**: ~$0.0001527/vCPU-hour + ~$0.0000127/GB-hour
  - 2 vCPU × 4GB: ~$0.12/day (~$3.60/month)
- **Storage (Page Blobs)**: ~$0.015/GB/month
- **Total estimated**: ~$5-10/month

## Troubleshooting

### Container Won't Start
1. Check logs: `az container logs --resource-group ironclad-rg --name ironclad-db-instance`
2. Verify environment variables
3. Ensure storage credentials are correct

### Connection Refused
1. Verify port 8080 is exposed
2. Check firewall rules in Azure
3. Verify FQDN: `az container show --resource-group ironclad-rg --name ironclad-db-instance --query ipAddress.fqdn`

### Storage Access Issues
1. Verify storage account exists
2. Check storage account key
3. Ensure application has permissions

## Performance Tuning

- **CPU**: Minimum 0.5vCPU, Recommended 2vCPU
- **Memory**: Minimum 0.5GB, Recommended 4GB
- **Network**: Check region for latency
- **Storage**: Use Premium storage for better performance

## Security Best Practices

1. Store credentials in Azure Key Vault
2. Use managed identities when possible
3. Enable container image scanning
4. Implement network policies
5. Use HTTPS for external endpoints

## Next Steps

- Set up CI/CD pipeline with GitHub Actions
- Configure auto-scaling based on metrics
- Implement logging with Azure Monitor
- Set up alerts and notifications
