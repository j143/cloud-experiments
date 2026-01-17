#!/bin/bash
set -e

echo "IronClad-DB Deployment Script"
echo "=============================="

# Variables
RESOURCE_GROUP="ironclad-rg"
ACR_NAME="ironcladadr"
STORAGE_ACCOUNT="ironcladstor"
CONTAINER_NAME="ironclad-db-prod"
IMAGE_NAME="ironclad-db"
IMAGE_TAG="latest"

# Step 1: Get storage account key
echo "\n1. Getting storage account key..."
STORAGE_KEY=$(az storage account keys list -g $RESOURCE_GROUP -n $STORAGE_ACCOUNT --query '[0].value' -o tsv)

# Step 2: Get ACR credentials
echo "\n2. Getting ACR credentials..."
ACR_USER=$(az acr credential show --name $ACR_NAME --query 'username' -o tsv)
ACR_PASS=$(az acr credential show --name $ACR_NAME --query 'passwords[0].value' -o tsv)
ACR_LOGIN_SERVER="${ACR_NAME}.azurecr.io"

# Step 3: Login to ACR using docker
echo "\n3. Logging into ACR..."
echo $ACR_PASS | docker login $ACR_LOGIN_SERVER -u $ACR_USER --password-stdin

# Step 4: Build Docker image
echo "\n4. Building Docker image..."
docker build -t $IMAGE_NAME:$IMAGE_TAG .

# Step 5: Tag image for ACR
echo "\n5. Tagging image for ACR..."
docker tag $IMAGE_NAME:$IMAGE_TAG $ACR_LOGIN_SERVER/$IMAGE_NAME:$IMAGE_TAG
docker tag $IMAGE_NAME:$IMAGE_TAG $ACR_LOGIN_SERVER/$IMAGE_NAME:v1.0

# Step 6: Push image to ACR
echo "\n6. Pushing image to ACR..."
docker push $ACR_LOGIN_SERVER/$IMAGE_NAME:$IMAGE_TAG
docker push $ACR_LOGIN_SERVER/$IMAGE_NAME:v1.0

# Step 7: Delete existing container (if any)
echo "\n7. Deleting existing container instance..."
az container delete -g $RESOURCE_GROUP -n $CONTAINER_NAME --yes || true

# Step 8: Deploy new container instance
echo "\n8. Deploying container instance..."
az container create \\
  --resource-group $RESOURCE_GROUP \\
  --name $CONTAINER_NAME \\
  --image $ACR_LOGIN_SERVER/$IMAGE_NAME:$IMAGE_TAG \\
  --registry-login-server $ACR_LOGIN_SERVER \\
  --registry-username "$ACR_USER" \\
  --registry-password "$ACR_PASS" \\
  --os-type Linux \\
  --cpu 2 \\
  --memory 2 \\
  --environment-variables \\
    AZURE_STORAGE_ACCOUNT=$STORAGE_ACCOUNT \\
    AZURE_STORAGE_KEY="$STORAGE_KEY" \\
    RUST_LOG=info \\
  --ports 8080 \\
  --dns-name-label ironclad-db-prod

# Step 9: Get FQDN
echo "\n9. Getting container FQDN..."
FQDN=$(az container show -g $RESOURCE_GROUP -n $CONTAINER_NAME --query ipAddress.fqdn -o tsv)

echo "\n=============================="
echo "Deployment completed!"
echo "Container FQDN: $FQDN"
echo "Health check: curl http://$FQDN:8080/health"
echo "=============================="
