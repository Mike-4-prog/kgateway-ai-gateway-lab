param(
    [Parameter(Mandatory=$false)]
    [ValidateSet("amd64", "arm64", "both")]
    [string]$Arch = "amd64",
    
    [Parameter(Mandatory=$false)]
    [string]$EnvoyImage = "envoyproxy/envoy:v1.36.4"
)

$OutputDir = "C:\Users\HP ELITEBOOK 840 G3\k\_output\cmd\envoyinit"
$ProjectRoot = "C:\Users\HP ELITEBOOK 840 G3\ai-gateway-lab"

Write-Host "🚀 Building AI Gateway Lab" -ForegroundColor Cyan
Write-Host "==========================" -ForegroundColor Cyan

function Test-Prerequisites {
    Write-Host "
🔍 Checking prerequisites..." -ForegroundColor Yellow
    
    # Check if Docker is running
    docker info 2>$null | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ Docker is not running. Please start Docker Desktop." -ForegroundColor Red
        exit 1
    }
    
    # Check if binaries exist
    if ($Arch -eq "amd64" -or $Arch -eq "both") {
        if (-not (Test-Path "$OutputDir\envoyinit-linux-amd64")) {
            Write-Host "❌ Missing amd64 binary: envoyinit-linux-amd64" -ForegroundColor Red
            exit 1
        }
    }
    
    if ($Arch -eq "arm64" -or $Arch -eq "both") {
        if (-not (Test-Path "$OutputDir\envoyinit-linux-arm64")) {
            Write-Host "⚠️  arm64 binary not found, will skip arm64 build" -ForegroundColor Yellow
            if ($Arch -eq "arm64") { exit 1 }
        }
    }
    
    Write-Host "✅ Prerequisites check passed" -ForegroundColor Green
}

function Build-Image {
    param([string]$TargetArch)
    
    Write-Host "
🐳 Building for $TargetArch..." -ForegroundColor Yellow
    
    # Set Rust build arch
    $RustArch = if ($TargetArch -eq "arm64") { "aarch64" } else { "x86_64" }
    
    # Navigate to the build context
    Push-Location $OutputDir
    
    # Use the fixed Dockerfile
    docker build 
        -f Dockerfile.envoyinit.fixed 
        --build-arg ENVOY_IMAGE="$EnvoyImage" 
        --build-arg TARGETPLATFORM="linux/$TargetArch" 
        --build-arg RUST_BUILD_ARCH="$RustArch" 
        --build-arg GOARCH="$TargetArch" 
        -t "ai-gateway:$TargetArch-latest" 
        .
    
    $result = $LASTEXITCODE
    Pop-Location
    
    if ($result -eq 0) {
        Write-Host "✅ Successfully built for $TargetArch" -ForegroundColor Green
        return $true
    } else {
        Write-Host "❌ Failed to build for $TargetArch" -ForegroundColor Red
        return $false
    }
}

# Main execution
Test-Prerequisites

switch ($Arch) {
    "both" {
        $success = $true
        if (-not (Build-Image "amd64")) { $success = $false }
        if (Test-Path "$OutputDir\envoyinit-linux-arm64") {
            if (-not (Build-Image "arm64")) { $success = $false }
        }
        if ($success) {
            Write-Host "
✅ All builds completed successfully!" -ForegroundColor Green
        } else {
            Write-Host "
❌ Some builds failed" -ForegroundColor Red
        }
    }
    default {
        if (Build-Image $Arch) {
            Write-Host "
✅ Build completed successfully!" -ForegroundColor Green
        }
    }
}
