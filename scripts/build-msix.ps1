param(
    [string]$Version = "1.0.0.0",
    [string]$Publisher = "CN=fs works",
    [string]$CertPath = "",
    [string]$CertPassword = ""
)

$ErrorActionPreference = "Stop"
$ProjectName = "docspeaker-cli"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir

Push-Location $ProjectRoot
try {
    # 1. Rust アプリをビルド
    Write-Host "Building Rust application..." -ForegroundColor Cyan
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "Cargo build failed" }

    # 2. パッケージ用ディレクトリを作成
    $PackageDir = ".\target\msix-package"
    Remove-Item $PackageDir -Recurse -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Path $PackageDir -Force | Out-Null
    New-Item -ItemType Directory -Path "$PackageDir\Assets" -Force | Out-Null

    # 3. 実行ファイルをコピー
    Write-Host "Copying files..." -ForegroundColor Cyan
    Copy-Item ".\target\release\$ProjectName.exe" $PackageDir

    # 4. アセットをコピー
    if (Test-Path ".\msix\Assets\*") {
        Copy-Item ".\msix\Assets\*" "$PackageDir\Assets\"
    }
    else {
        Write-Host "Warning: No assets found in msix\Assets\" -ForegroundColor Yellow
        Write-Host "Please add Square44x44Logo.png, Square150x150Logo.png, StoreLogo.png" -ForegroundColor Yellow
    }

    # 5. マニフェストをコピーしてバージョンとPublisherを更新
    Write-Host "Preparing manifest..." -ForegroundColor Cyan
    $manifest = Get-Content ".\msix\AppxManifest.xml" -Raw
    $manifest = $manifest -replace 'Version="[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+"', "Version=`"$Version`""
    $manifest = $manifest -replace 'Publisher="CN=[^"]*"', "Publisher=`"$Publisher`""
    $manifest | Set-Content "$PackageDir\AppxManifest.xml" -Encoding UTF8

    # 6. MSIX パッケージを作成
    $OutputPath = ".\target\$ProjectName-$Version.msix"
    Write-Host "Creating MSIX package..." -ForegroundColor Cyan

    # makeappx.exe を探す
    $makeappx = Get-Command makeappx.exe -ErrorAction SilentlyContinue
    if (-not $makeappx) {
        $sdkPaths = @(
            "${env:ProgramFiles(x86)}\Windows Kits\10\bin\10.0.22621.0\x64\makeappx.exe",
            "${env:ProgramFiles(x86)}\Windows Kits\10\bin\10.0.19041.0\x64\makeappx.exe",
            "${env:ProgramFiles(x86)}\Windows Kits\10\bin\10.0.18362.0\x64\makeappx.exe"
        )
        foreach ($path in $sdkPaths) {
            if (Test-Path $path) {
                $makeappx = $path
                break
            }
        }
    }
    if (-not $makeappx) {
        throw "makeappx.exe not found. Please install Windows 10 SDK or add it to PATH."
    }

    & $makeappx pack /d $PackageDir /p $OutputPath /o
    if ($LASTEXITCODE -ne 0) { throw "MakeAppx failed" }

    # 7. 署名 (証明書が指定されている場合)
    if ($CertPath -ne "" -and (Test-Path $CertPath)) {
        Write-Host "Signing package..." -ForegroundColor Cyan

        $signtool = Get-Command signtool.exe -ErrorAction SilentlyContinue
        if (-not $signtool) {
            $sdkPaths = @(
                "${env:ProgramFiles(x86)}\Windows Kits\10\bin\10.0.22621.0\x64\signtool.exe",
                "${env:ProgramFiles(x86)}\Windows Kits\10\bin\10.0.19041.0\x64\signtool.exe",
                "${env:ProgramFiles(x86)}\Windows Kits\10\bin\10.0.18362.0\x64\signtool.exe"
            )
            foreach ($path in $sdkPaths) {
                if (Test-Path $path) {
                    $signtool = $path
                    break
                }
            }
        }
        if (-not $signtool) {
            throw "signtool.exe not found. Please install Windows 10 SDK or add it to PATH."
        }

        & $signtool sign /fd SHA256 /f $CertPath /p $CertPassword /td SHA256 /tr http://timestamp.digicert.com $OutputPath
        if ($LASTEXITCODE -ne 0) { throw "SignTool failed" }
    }
    elseif ($CertPath -ne "") {
        Write-Host "Warning: Certificate file not found: $CertPath" -ForegroundColor Yellow
    }

    Write-Host ""
    Write-Host "Done! Package created: $OutputPath" -ForegroundColor Green
    Write-Host ""

    if ($CertPath -eq "") {
        Write-Host "Note: Package is unsigned. To sign it, run:" -ForegroundColor Yellow
        Write-Host "  .\scripts\build-msix.ps1 -CertPath .\test-cert.pfx -CertPassword 'password123'" -ForegroundColor Yellow
    }
}
finally {
    Pop-Location
}
