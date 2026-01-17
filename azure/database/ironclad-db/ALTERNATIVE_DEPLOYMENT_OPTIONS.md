# Alternative Deployment Options for IronClad-DB

## Problem Statement

When attempting to deploy IronClad-DB through Azure Cloud Shell, we encountered two blockers:

1. **ACR Tasks not permitted**: "ACR Tasks requests are not permitted for this subscription"
2. **No Docker daemon in Cloud Shell**: Cannot build Docker images locally

This is a **common limitation** with Azure for Students subscriptions and Cloud Shell in general.

## Azure for Students Limitations

Based on research and testing:

❌ **ACR Tasks/Builds**: Not available (requires support request)  
❌ **Docker in Cloud Shell**: Not supported  
✅ **Azure Container Registry**: Available (Basic SKU)  
✅ **Container Instances**: Available (3 clusters max per region)  
✅ **GitHub Actions**: Available (free for public repos)  
✅ **Service Principals**: Can be created  
✅ **Azure App Service**: Available  

## Solution: GitHub Actions (RECOMMENDED) ✅

### Why This Works

- **Free Docker Build Environment**: GitHub provides Ubuntu runners with Docker pre-installed
- **No Local Requirements**: No need for Docker or Azure CLI on your machine
- **Compatible with Azure for Students**: No subscription limitations
- **Automated CI/CD**: Builds and deploys on every push
- **Build Logs Preserved**: See exactly what happened

### How It Works

```
GitHub Repository
      |
      v
GitHub Actions Runner (Ubuntu + Docker)
      |
      ├─> Build Docker Image (Dockerfile)
      ├─> Push to Azure Container Registry
      └─> Deploy to Azure Container Instances
            |
            v
      IronClad-DB Running
```

### Setup Time: ~10 minutes

1. Create Azure Service Principal (3 min)
2. Add GitHub Secret (2 min)
3. Push workflow file (1 min)
4. Wait for deployment (5-10 min)

### Files Created

- `.github/workflows/deploy-ironclad-db.yml` - GitHub Actions workflow
- `azure/database/ironclad-db/GITHUB_ACTIONS_SETUP.md` - Setup guide

##  Alternative Options Explored

### Option 2: Azure DevOps Pipelines

**Status**: ✅ Viable Alternative

**Pros**:
- Similar to GitHub Actions
- Native Azure integration
- Free tier available

**Cons**:
- Requires Azure DevOps project setup
- More complex configuration
- Less familiar for GitHub users

**Use Case**: If you prefer Azure-native tools

### Option 3: Azure App Service (Web App for Containers)

**Status**: ✅ Works but requires image first

**Pros**:
- Easier scaling
- Built-in SSL/custom domains
- Better for web applications

**Cons**:
- Still need to build image somewhere
- More expensive ($13-55/month)
- Overkill for demo application

**Command**:
```bash
az webapp create \
  --resource-group ironclad-rg \
  --plan ironclad-plan \
  --name ironclad-db-app \
  --deployment-container-image-name ironcladadr.azurecr.io/ironclad-db:latest
```

### Option 4: Azure Container Apps

**Status**: ✅ Modern alternative

**Pros**:
- Serverless container platform
- Auto-scaling to zero
- Better than ACI for production

**Cons**:
- Still needs image built
- Slightly more complex
- Newer service (less documentation)

**Command**:
```bash
az containerapp create \
  --name ironclad-db \
  --resource-group ironclad-rg \
  --environment ironclad-env \
  --image ironcladadr.azurecr.io/ironclad-db:latest \
  --target-port 8080 \
  --ingress external
```

### Option 5: Local Build + Manual Push

**Status**: ⚠️ Requires local Docker

**Pros**:
- Full control
- Works with any subscription

**Cons**:
- Requires Docker installed locally
- Manual process
- Not automated

**Use Case**: One-time deployment or testing

### Option 6: Azure VM with Docker

**Status**: ✅ Works but expensive

**Pros**:
- Full Docker environment
- Complete control

**Cons**:
- Costs $30-100/month
- Overkill for simple build
- Requires VM management

**Not Recommended**: Use GitHub Actions instead

### Option 7: Docker Hub Public Registry

**Status**: ✅ Quick workaround

**Pros**:
- Can build locally and push to Docker Hub
- Free for public images
- No Azure dependency for image storage

**Cons**:
- Image is public
- Requires local Docker still
- Not using ACR we already have

**Command**:
```bash
# Build and push to Docker Hub
docker build -t yourusername/ironclad-db:latest .
docker push yourusername/ironclad-db:latest

# Deploy from Docker Hub
az container create \
  --image yourusername/ironclad-db:latest \
  ...
```

## Comparison Matrix

| Option | Build Location | Cost | Automation | Setup Time | Recommended |
|--------|----------------|------|------------|------------|--------------|
| **GitHub Actions** | GitHub Runners | Free | ✅ Yes | 10 min | ✅ **YES** |
| Azure DevOps | Azure Pipelines | Free | ✅ Yes | 20 min | ⚠️ If needed |
| Local Build | Your Machine | Free | ❌ No | 5 min | ⚠️ One-time |
| Azure VM | Azure VM | $30+/mo | ⚠️ Custom | 30 min | ❌ No |
| Docker Hub | Local | Free | ❌ No | 10 min | ⚠️ Quick test |
| App Service | GitHub/Local | $13+/mo | ⚠️ Varies | 15 min | ⚠️ Production |
| Container Apps | GitHub/Local | $5+/mo | ⚠️ Varies | 20 min | ⚠️ Production |

## Why GitHub Actions Wins

1. **Zero Cost** - Free for public repositories
2. **Zero Local Requirements** - No Docker or tooling needed
3. **Automated** - Deploys on every push
4. **Fast Setup** - 10 minutes total
5. **Works with Azure for Students** - No subscription limitations
6. **Industry Standard** - Most popular CI/CD platform
7. **Great Documentation** - Easy to debug and customize

## Next Steps

1. Follow `GITHUB_ACTIONS_SETUP.md` to set up automated deployment
2. Push code changes
3. Watch GitHub Actions build and deploy
4. Verify application output
5. Set up monitoring and alerts

## Lessons Learned

### What Doesn't Work in Azure Cloud Shell
- ❌ ACR Build Tasks (subscription limitation)
- ❌ Docker builds (no daemon)
- ❌ Cargo/Rust builds (not installed)

### What Works
- ✅ Azure CLI commands
- ✅ Creating Azure resources
- ✅ Git operations
- ✅ Bash scripts

### Best Practices
- Use CI/CD platforms (GitHub Actions, Azure DevOps) for builds
- Keep Cloud Shell for infrastructure management
- Automate deployments, don't build manually
- Use service principals for CI/CD authentication

## Cost Summary

**With GitHub Actions (Recommended)**:
- GitHub Actions: Free (public repo)
- Azure Resources: ~$10-15/month
- **Total**: $10-15/month

**With Azure VM**:
- VM (B2s): ~$30/month
- Azure Resources: ~$10-15/month
- **Total**: $40-45/month

**Savings**: $25-30/month using GitHub Actions!

## Support

For issues or questions:
- GitHub Actions Setup: See `GITHUB_ACTIONS_SETUP.md`
- General Deployment: See `DEPLOYMENT.md`
- Infrastructure Status: See `DEPLOYMENT_STATUS.md`
