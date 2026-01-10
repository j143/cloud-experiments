#!/bin/bash

# CAP Theorem Verification - Azure Cosmos DB Setup Script
# This script sets up the infrastructure for CAP theorem testing

set -e

# Configuration
RESOURCE_GROUP="cap-theorem-rg"
ACCOUNT_NAME="cosmos-cap-theorem"
DATABASE_NAME="cap-test-db"
CONTAINER_NAME="cap-container"
REGION_PRIMARY="northeurope"
REGION_SECONDARY="eastus"
THROUGHPUT=400

echo "======================================="
echo "CAP Theorem - Cosmos DB Setup"
echo "======================================="

echo "\n[1/5] Creating resource group: $RESOURCE_GROUP"
az group create --name $RESOURCE_GROUP --location $REGION_PRIMARY

echo "\n[2/5] Creating Cosmos DB account: $ACCOUNT_NAME"
az cosmosdb create \
  --name $ACCOUNT_NAME \
  --resource-group $RESOURCE_GROUP \
  --locations regionName=$REGION_PRIMARY failoverPriority=0 \
  --locations regionName=$REGION_SECONDARY failoverPriority=1 \
  --enable-multiple-write-locations true \
  --default-consistency-level Session

echo "\n[3/5] Creating database: $DATABASE_NAME"
az cosmosdb sql database create \
  --account-name $ACCOUNT_NAME \
  --resource-group $RESOURCE_GROUP \
  --name $DATABASE_NAME \
  --throughput $THROUGHPUT

echo "\n[4/5] Creating container: $CONTAINER_NAME"
az cosmosdb sql container create \
  --account-name $ACCOUNT_NAME \
  --resource-group $RESOURCE_GROUP \
  --database-name $DATABASE_NAME \
  --name $CONTAINER_NAME \
  --partition-key-path /pk \
  --throughput $THROUGHPUT

echo "\n[5/5] Retrieving connection key"
KEY=$(az cosmosdb keys list \
  --name $ACCOUNT_NAME \
  --resource-group $RESOURCE_GROUP \
  --type keys \
  --query 'primaryMasterKey' \
  -o tsv)

echo "\n======================================="
echo "Setup Complete!"
echo "======================================="
echo "Endpoint: https://$ACCOUNT_NAME.documents.azure.com:443/"
echo "Database: $DATABASE_NAME"
echo "Container: $CONTAINER_NAME"
echo "Primary Region: $REGION_PRIMARY"
echo "Secondary Region: $REGION_SECONDARY"
echo "Throughput: $THROUGHPUT RU/s"
echo ""
echo "To run tests:"
echo "export COSMOS_KEY='$KEY'"
echo "export COSMOS_ENDPOINT='https://$ACCOUNT_NAME.documents.azure.com:443/'"
echo "python cap_test_simple.py"
