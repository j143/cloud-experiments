# GitHub Actions Deployment Setup

This document explains how to deploy IronClad-DB automatically using GitHub Actions - the **recommended alternative** when you cannot build Docker images in Azure Cloud Shell.

## Why GitHub Actions?

✅ **Free Docker build environment** - GitHub provides free runners with Docker pre-installed  
✅ **Automated deployment** - Builds and deploys on every push to main branch  
✅ **No local setup required** - No need for Docker or Azure CLI on your machine  
✅ **Works with Azure for Students** - Compatible with all Azure subscription types  
✅ **Build logs preserved** - See exactly what happened during deployment  

## Setup Steps

### Step 1: Create Azure Service Principal

Run this in Azure Cloud Shell:

```bash
# Create a service principal for GitHub Actions
az ad sp create-for-rbac \
  --name "github-actions-ironclad" \
  --role contributor \
  --scopes /subscriptions/$(az account show --query id -o tsv)/resourceGroups/ironclad-rg \
  --sdk-auth
```

This will output JSON credentials like:

```json
{
  "clientId": "<GUID>",
  "clientSecret": "<GUID>",
  "subscriptionId": "<GUID>",
  "tenantId": "<GUID>",
  "activeDirectoryEndpointUrl": "https://login.microsoftonline.com",
  "resourceManagerEndpointUrl": "https://management.azure.com/",
  "activeDirectoryGraphResourceId": "https://graph.windows.net/",
  "sqlManagementEndpointUrl": "https://management.core.windows.net:8443/",
  "galleryEndpointUrl": "https://gallery.azure.com/",
  "managementEndpointUrl": "https://management.core.windows.net/"
}
```

**IMPORTANT:** Copy this entire JSON output - you'll need it in Step 2.

### Step 2: Add Secret to GitHub Repository

1. Go to your GitHub repository: https://github.com/j143/cloud-experiments
2. Click **Settings** (top menu)
3. Click **Secrets and variables** → **Actions** (left sidebar)
4. Click **New repository secret**
5. Name: `AZURE_CREDENTIALS`
6. Value: Paste the entire JSON from Step 1
7. Click **Add secret**

### Step 3: Commit and Push the Workflow

```bash
cd ~/cloud-experiments

# Add the workflow file
git add .github/workflows/deploy-ironclad-db.yml
git add azure/database/ironclad-db/GITHUB_ACTIONS_SETUP.md

# Commit
git commit -m "Add GitHub Actions workflow for IronClad-DB deployment"

# Push to GitHub
git push origin main
```

### Step 4: Monitor the Deployment

1. Go to your repository on GitHub
2. Click the **Actions** tab
3. You should see "Deploy IronClad-DB to Azure" workflow running
4. Click on the workflow to see live logs
5. Wait for it to complete (~5-10 minutes)

## Manual Trigger

You can also trigger the deployment manually:

1. Go to **Actions** tab on GitHub
2. Select "Deploy IronClad-DB to Azure" workflow
3. Click **Run workflow** button
4. Select branch (main)
5. Click **Run workflow**

## Verification

Once the workflow completes:

```bash
# Check the deployed container
az container show -g ironclad-rg -n ironclad-db-prod

# View application output
az container logs -g ironclad-rg -n ironclad-db-prod

# Get the FQDN
FQDN=$(az container show -g ironclad-rg -n ironclad-db-prod --query ipAddress.fqdn -o tsv)
echo "Application URL: http://$FQDN:8080"
```

## Troubleshooting

### Error: "Resource 'Microsoft.Network/publicIPAddresses' not found"

**Solution:** The container instance name conflicts. Delete the existing one:

```bash
az container delete -g ironclad-rg -n ironclad-db-prod --yes
```

Then re-run the workflow.

### Error: "Service principal not found"

**Solution:** Make sure the service principal was created correctly and the JSON is complete in GitHub secrets.

### Error: "Failed to pull image"

**Solution:** Check that ACR admin user is enabled:

```bash
az acr update -n ironcladadr --admin-enabled true
```

### Workflow doesn't start automatically

**Solution:** Make sure you pushed changes to the `azure/database/ironclad-db/**` path or use manual trigger.

## Cost Implications

- **GitHub Actions**: Free for public repositories (2000 minutes/month for private)
- **Azure Resources**: Same as before (~$10-15/month)
- **No additional costs** for using GitHub Actions

## Advantages Over Local Deployment

| Feature | Local Deployment | GitHub Actions |
|---------|-----------------|----------------|
| Requires Docker locally | ✅ Yes | ❌ No |
| Requires Azure CLI | ✅ Yes | ❌ No |
| Works from anywhere | ❌ No | ✅ Yes |
| Automated on push | ❌ No | ✅ Yes |
| Build logs preserved | ❌ No | ✅ Yes |
| Team collaboration | ❌ Limited | ✅ Easy |

## Next Steps

After successful deployment:

1. ✅ Monitor application logs
2. ✅ Set up health check monitoring
3. ✅ Configure alerts for failures
4. ✅ Add staging environment
5. ✅ Implement blue-green deployment

## Alternative: Azure DevOps Pipelines

If you prefer Azure DevOps instead of GitHub Actions, the process is similar:

1. Create an Azure DevOps project
2. Create a service connection to your Azure subscription
3. Create a pipeline using the provided YAML
4. Trigger the pipeline

However, GitHub Actions is recommended for its simplicity and free tier.
