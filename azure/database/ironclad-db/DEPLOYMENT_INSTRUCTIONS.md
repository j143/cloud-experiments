# IronClad-DB Deployment Instructions

## Current Status

### Azure Infrastructure (DEPLOYED ✅)
- **Resource Group**: `ironclad-rg` (South India)
- **Storage Account**: `ironcladstor` (Page Blobs)
- **Container Registry**: `ironcladadr` (ACR Basic SKU)
- **Container Instance**: `ironclad-db-prod` (placeholder)

### What's Missing

The Docker image for IronClad-DB needs to be built and pushed to the Azure Container Registry. This cannot be done in Azure Cloud Shell due to:
1. ACR Tasks are not permitted for this subscription
2. Docker daemon is not available in Cloud Shell

## Deployment Options

### Option 1: Local Deployment (Recommended)

Run the provided deployment script on your local machine (with Docker and Azure CLI installed):

```bash
cd ~/cloud-experiments/azure/database/ironclad-db
./deploy_ironclad.sh
```

This script will:
1. Get storage and ACR credentials from Azure
2. Build the Docker image locally
3. Push the image to Azure Container Registry
4. Deploy the container instance with proper configuration

**Prerequisites**:
- Docker installed and running
- Azure CLI installed (`az login` completed)
- Access to the `ironclad-rg` resource group

### Option 2: GitHub Actions (CI/CD)

Create a GitHub Actions workflow to automatically build and deploy:

1. Store Azure credentials in GitHub Secrets:
   - `AZURE_CREDENTIALS`
   - `ACR_USERNAME`
   - `ACR_PASSWORD`
   - `STORAGE_KEY`

2. Use the workflow file (to be created) at `.github/workflows/deploy-ironclad.yml`

### Option 3: Enable ACR Tasks

Contact Azure support to enable ACR Tasks for your subscription, then run:

```bash
az acr build --registry ironcladadr \\
  --image ironclad-db:latest \\
  --image ironclad-db:v1.0 \\
  .
```

## Manual Deployment Steps

If you prefer to run commands manually:

### Step 1: Build Docker Image Locally

```bash
cd ~/cloud-experiments/azure/database/ironclad-db
docker build -t ironclad-db:latest .
```

### Step 2: Login to ACR

```bash
az acr login --name ironcladadr
# OR
ACR_PASS=$(az acr credential show --name ironcladadr --query 'passwords[0].value' -o tsv)
echo $ACR_PASS | docker login ironcladadr.azurecr.io -u ironcladadr --password-stdin
```

### Step 3: Tag and Push Image

```bash
docker tag ironclad-db:latest ironcladadr.azurecr.io/ironclad-db:latest
docker tag ironclad-db:latest ironcladadr.azurecr.io/ironclad-db:v1.0
docker push ironcladadr.azurecr.io/ironclad-db:latest
docker push ironcladadr.azurecr.io/ironclad-db:v1.0
```

### Step 4: Deploy Container Instance

```bash
# Get credentials
STORAGE_KEY=$(az storage account keys list -g ironclad-rg -n ironcladstor --query '[0].value' -o tsv)
ACR_USER=$(az acr credential show --name ironcladadr --query 'username' -o tsv)
ACR_PASS=$(az acr credential show --name ironcladadr --query 'passwords[0].value' -o tsv)

# Delete existing container (if any)
az container delete -g ironclad-rg -n ironclad-db-prod --yes

# Create new container
az container create \\
  --resource-group ironclad-rg \\
  --name ironclad-db-prod \\
  --image ironcladadr.azurecr.io/ironclad-db:latest \\
  --registry-login-server ironcladadr.azurecr.io \\
  --registry-username "$ACR_USER" \\
  --registry-password "$ACR_PASS" \\
  --os-type Linux \\
  --cpu 2 \\
  --memory 2 \\
  --environment-variables \\
    AZURE_STORAGE_ACCOUNT=ironcladstor \\
    AZURE_STORAGE_KEY="$STORAGE_KEY" \\
    RUST_LOG=info \\
  --ports 8080 \\
  --dns-name-label ironclad-db-prod
```

### Step 5: Verify Deployment

```bash
# Get FQDN
FQDN=$(az container show -g ironclad-rg -n ironclad-db-prod --query ipAddress.fqdn -o tsv)
echo "Container FQDN: $FQDN"

# Check health
curl http://$FQDN:8080/health

# View logs
az container logs -g ironclad-rg -n ironclad-db-prod
```

## Troubleshooting

### Container Won't Start

```bash
# Check logs
az container logs --resource-group ironclad-rg --name ironclad-db-prod

# Check container state
az container show --resource-group ironclad-rg --name ironclad-db-prod
```

### Image Pull Errors

- Verify ACR credentials are correct
- Check that image exists: `az acr repository list --name ironcladadr`
- Verify admin user is enabled in ACR

### Storage Access Issues

- Verify storage account key is correct
- Check storage account exists and is accessible
- Ensure network rules allow container access

## Next Steps

1. ✅ Complete Docker image build and push
2. ✅ Deploy container instance with IronClad-DB
3. Set up monitoring and logging
4. Configure auto-scaling (Container Apps)
5. Implement CI/CD pipeline
6. Set up alerts and notifications

## Cost Estimate

- Storage Account: ~$1-2/month
- Container Registry: ~$5/month
- Container Instances (2 vCPU × 2GB): ~$3-5/month
- **Total**: ~$10-15/month

## Support

For issues or questions, refer to:
- [DEPLOYMENT.md](./DEPLOYMENT.md) - Full deployment guide
- [DEPLOYMENT_STATUS.md](./DEPLOYMENT_STATUS.md) - Infrastructure status
- [Azure Portal](https://portal.azure.com/)
