# Generates the updater's minisign key pair once (ADR-0018) and writes only the public key to
# apps/desktop/tauri.conf.json. The private key never leaves this machine: it goes to the file
# below, protected by the password `tauri signer` asks for, and from there to the password
# manager plus an offline copy. Losing it means no installed copy can ever update again.
#
# Usage (repo root, Windows PowerShell): .\scripts\release\new-updater-key.ps1
# Refuses to run when the key file already exists: a second key is a rotation, not a redo.

$ErrorActionPreference = 'Stop'

$keyPath = Join-Path $HOME '.tauri\fala-updater.key'
$pubPath = "$keyPath.pub"
$conf = Join-Path $PSScriptRoot '..\..\apps\desktop\tauri.conf.json' | Resolve-Path

if ((Test-Path $keyPath) -or (Test-Path $pubPath)) {
    throw "$keyPath already exists. Use that key; replacing it would strand every installed copy."
}
New-Item -ItemType Directory -Force (Split-Path $keyPath) | Out-Null

Write-Host 'tauri signer will ask for a password twice. Use a long one and store it in the password manager.'
# stdout is dropped so nothing the signer prints lands in this console's scrollback or a log;
# the password prompt goes to the console directly.
Push-Location (Join-Path $PSScriptRoot '..\..')
try {
    bun run tauri signer generate -w $keyPath | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "tauri signer generate failed ($LASTEXITCODE)" }
} finally {
    Pop-Location
}
if (-not (Test-Path $pubPath)) { throw "tauri signer did not write $pubPath" }

$pubkey = (Get-Content $pubPath -Raw).Trim()
$decoded = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($pubkey))
if (-not $decoded.StartsWith('untrusted comment: minisign public key')) {
    throw "$pubPath is not a minisign public key"
}

$text = [IO.File]::ReadAllText($conf)
$pattern = '("pubkey"\s*:\s*)"[^"]*"'
if (([regex]::Matches($text, $pattern)).Count -ne 1) {
    throw "expected exactly one `"pubkey`" in $conf"
}
$text = [regex]::Replace($text, $pattern, ('$1"' + $pubkey + '"'))
[IO.File]::WriteAllText($conf, $text, (New-Object Text.UTF8Encoding $false))

Write-Host "Public key written to $conf."
Write-Host "Private key: $keyPath (password-protected)."
Write-Host 'Now: store the key file and its password in the password manager, make the offline copy,'
Write-Host 'run python scripts/ci/test_release_config.py and commit tauri.conf.json.'
