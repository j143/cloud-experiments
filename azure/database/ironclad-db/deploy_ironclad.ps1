# Deploy IroncladDB to Azure using PowerShell
# Reads configuration from .env file

$ErrorActionPreference = "Stop"

function Load-Env {
    param($EnvFile = ".env")
    if (Test-Path $EnvFile) {
        Write-Host "Loading configuration from $EnvFile..." -ForegroundColor Cyan
        Get-Content $EnvFile | ForEach-Object {
            $line = $_.Trim()
            if ($line -and -not $line.StartsWith("#")) {
                $name, $value = $line -split '=', 2
                if ($name -and $value) {
                    Set-Variable -Name $name -Value $value -Scope Script
                    Write-Host "  loaded: $name" -ForegroundColor Gray
                }
            }
        }
    } else {
        Write-Error ".env file not found. Please create one based on .env.example"
    }
}

Load-Env

# --- Configuration ---
$ACR_LOGIN_SERVER = "$ACR_NAME.azurecr.io"
$FULL_IMAGE_TAG = "$ACR_LOGIN_SERVER/$IMAGE_NAME`:$IMAGE_TAG"

# Determine AZ CLI Path
if ($env:AZ_CLI_PATH) {
    $AZ_CMD = $env:AZ_CLI_PATH
} elseif ($Script:AZ_CLI_PATH) {
    $AZ_CMD = $Script:AZ_CLI_PATH
} else {
    # Default fallback locations or PATH
    if (Get-Command "az" -ErrorAction SilentlyContinue) {
        $AZ_CMD = "az"
    } elseif (Test-Path "C:\Program Files\Microsoft SDKs\Azure\CLI2\wbin\az.cmd") {
        $AZ_CMD = "C:\Program Files\Microsoft SDKs\Azure\CLI2\wbin\az.cmd"
    } else {
        Write-Error "Azure CLI ('az') not found in PATH or standard locations. Please install it or set AZ_CLI_PATH in .env."
    }
}

Write-Host "`nUsing Azure CLI at: $AZ_CMD" -ForegroundColor Cyan

# --- Steps ---

Write-Host "`n1. Logging into ACR ($ACR_LOGIN_SERVER)..." -ForegroundColor Green
$env:DOCKER_CLI_HINTS = "false" # suppress hints
echo $ACR_PASS | docker login $ACR_LOGIN_SERVER -u $ACR_USER --password-stdin

Write-Host "`n2. Building Docker Image..." -ForegroundColor Green
docker build -t "$IMAGE_NAME`:$IMAGE_TAG" .
docker tag "$IMAGE_NAME`:$IMAGE_TAG" $FULL_IMAGE_TAG

Write-Host "`n3. Pushing Image to ACR..." -ForegroundColor Green
docker push $FULL_IMAGE_TAG

Write-Host "`n4. Removing Existing Container (if any)..." -ForegroundColor Green
# Use Invoke-Expression or & to run the command, handling the batch file correctly
& $AZ_CMD container delete -g $RESOURCE_GROUP -n $CONTAINER_NAME --yes 2>&1 | Out-Null
Write-Host "   Done."

Write-Host "`n5. Deploying New Container Instance..." -ForegroundColor Green
$ConnectionString = "DefaultEndpointsProtocol=https;AccountName=$STORAGE_ACCOUNT;AccountKey=$STORAGE_KEY;EndpointSuffix=core.windows.net"

# Construct the command specifically for PowerShell to avoid parsing issues
$AzArgs = @(
    "container", "create",
    "--resource-group", $RESOURCE_GROUP,
    "--name", $CONTAINER_NAME,
    "--image", $FULL_IMAGE_TAG,
    "--registry-login-server", $ACR_LOGIN_SERVER,
    "--registry-username", $ACR_USER,
    "--registry-password", $ACR_PASS,
    "--os-type", "Linux",
    "--cpu", "2",
    "--memory", "2",
    "--environment-variables", "AZURE_STORAGE_CONNECTION_STRING=$ConnectionString", "RUST_LOG=info",
    "--ports", "8080",
    "--dns-name-label", $CONTAINER_NAME
)

# Execution
& $AZ_CMD $AzArgs

Write-Host "`nDeployment Complete!" -ForegroundColor Green
Write-Host "--------------------------------------------------"
Write-Host "View logs with:"
Write-Host "  & `"$AZ_CMD`" container logs -g $RESOURCE_GROUP -n $CONTAINER_NAME" -ForegroundColor Yellow
Write-Host "--------------------------------------------------"
