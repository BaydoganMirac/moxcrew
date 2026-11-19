param(
    [Parameter(Mandatory)][string] $Setup,
    [Parameter(Mandatory)][string] $Msi,
    [Parameter(Mandatory)][string] $ExpectedHash
)

$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted' -or !$IsWindows) {
    throw 'Installer lifecycle checks require a disposable GitHub-hosted Windows runner.'
}

function Invoke-Installer([string] $File, [string[]] $Arguments) {
    $process = Start-Process -FilePath $File -ArgumentList $Arguments -WindowStyle Hidden -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw "$File returned exit code $($process.ExitCode)" }
}

function Assert-Payload([string] $File, [string] $Hash) {
    if ((Get-FileHash -LiteralPath $File).Hash -ne $Hash) { throw "Unexpected installed payload in $File" }
}

function Assert-Profiles {
    foreach ($file in $markers.Keys) {
        if ((Get-FileHash -LiteralPath $file).Hash -ne $markers[$file]) {
            throw "Installer modified profile fixture $file"
        }
    }
}

$markers = @{}
foreach ($profileName in @('com.patchfleet.desktop', 'Patchfleet', 'Moxcrew')) {
    $directory = Join-Path $env:APPDATA $profileName
    New-Item -ItemType Directory -Path $directory -Force | Out-Null
    $marker = Join-Path $directory ('installer-test-' + [guid]::NewGuid() + '.txt')
    [IO.File]::WriteAllText($marker, [guid]::NewGuid().ToString())
    $markers[$marker] = (Get-FileHash -LiteralPath $marker).Hash
}

$fixture = Join-Path $env:GITHUB_WORKSPACE 'previous-preview'
$legacySetup = Join-Path $fixture 'dist/native/patchfleet-desktop_0.13.1_x64-setup.exe'
$legacyHash = (Get-FileHash (Join-Path $fixture 'target/release/patchfleet-desktop.exe')).Hash
$legacyDir = Join-Path $env:RUNNER_TEMP 'Custom Patchfleet'
$setupDir = Join-Path $env:RUNNER_TEMP 'Custom Moxcrew'
$msiDir = Join-Path $env:RUNNER_TEMP 'Moxcrew MSI Install'
$legacyApp = Join-Path $legacyDir 'patchfleet-desktop.exe'
$setupApp = Join-Path $setupDir 'moxcrew-desktop.exe'
$msiApp = Join-Path $msiDir 'moxcrew-desktop.exe'
$uninstall = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall'
$legacyKey = Join-Path $uninstall 'Patchfleet'
$currentKey = Join-Path $uninstall 'Moxcrew'

# The old preview had a public template but no published Windows binary.
# Its installer fixture uses a small inert payload and the exact tagged NSIS configuration.
Invoke-Installer $legacySetup @('/S', '/NS', "/D=$legacyDir")
Assert-Payload $legacyApp $legacyHash
if (!(Test-Path $legacyKey)) { throw 'Previous install did not register Patchfleet' }
$attempt = Start-Process -FilePath $Setup -ArgumentList @('/S', '/NS', "/D=$setupDir") -WindowStyle Hidden -Wait -PassThru
if ($attempt.ExitCode -eq 0 -or (Test-Path $setupApp) -or (Test-Path $currentKey)) {
    throw 'Silent replacement of a registered legacy NSIS installation was accepted'
}
Assert-Payload $legacyApp $legacyHash
Assert-Profiles

# Cross-format upgrade through removal and installation, both in custom directories.
Invoke-Installer (Join-Path $legacyDir 'uninstall.exe') @('/S', "_?=$legacyDir")
if ((Test-Path $legacyApp) -or (Test-Path $legacyKey)) { throw 'Legacy uninstall left the product installed' }
Assert-Profiles
Invoke-Installer 'msiexec.exe' @('/i', ('"{0}"' -f $Msi), '/qn', '/norestart', ('INSTALLDIR="{0}"' -f $msiDir))
Assert-Payload $msiApp $ExpectedHash
Assert-Profiles
Invoke-Installer 'msiexec.exe' @('/x', ('"{0}"' -f $Msi), '/qn', '/norestart')
if (Test-Path $msiApp) { throw 'MSI uninstall left the installed application' }
Assert-Profiles

Invoke-Installer $Setup @('/S', '/NS', "/D=$setupDir")
Assert-Payload $setupApp $ExpectedHash
if (!(Test-Path $currentKey)) { throw 'Current install did not register Moxcrew' }
Assert-Profiles
Invoke-Installer (Join-Path $setupDir 'uninstall.exe') @('/S', "_?=$setupDir")
if ((Test-Path $setupApp) -or (Test-Path $currentKey)) { throw 'NSIS uninstall left the product installed' }
Assert-Profiles
Write-Output 'Legacy NSIS removal, MSI and NSIS installs, payloads and uninstall profile preservation passed.'
