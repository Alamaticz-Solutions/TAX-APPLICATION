<#
.SYNOPSIS
  Runs the Tax backend (or the Box live checks) against the REAL Box, with the secrets typed
  into this terminal. Nothing secret is written to disk, to git, or to a log.

.DESCRIPTION
  -Mode check   Opens your Box key store and signs a login request. Calls nothing on Box.
  -Mode box     Also creates, uploads, reads, refuses a duplicate and deletes ONE throwaway folder
                inside a TEST folder you name, using the real Box API.
  -Mode run     Starts the backend with STORAGE_PROVIDER=box so the app files documents in Box.

  You are asked for: the Box client secret and the key store password (hidden as you type), and
  the non-secret values below if you did not pass them. Run it from tax-doc-routing\.

.EXAMPLE
  .\scripts\box-local.ps1 -Mode check -ClientId <id> -EnterpriseId <id> -PublicKeyId <id>
#>
param(
  [Parameter(Mandatory)][ValidateSet('check', 'box', 'run')][string]$Mode,
  [string]$KeystorePath = (Join-Path $env:USERPROFILE 'Downloads\pdsbox.p12'),
  [string]$ClientId,
  [string]$EnterpriseId,
  [string]$PublicKeyId,
  [string]$TestParentFolder,
  [string]$TempParentFolder,
  [switch]$ExternalOff
)

$ErrorActionPreference = 'Stop'
function Ask($label, $current) { if ($current) { return $current }; return (Read-Host $label) }
function AskSecret($label) {
  $s = Read-Host $label -AsSecureString
  $p = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($s)
  try { return [Runtime.InteropServices.Marshal]::PtrToStringBSTR($p) } finally { [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($p) }
}

if (-not (Test-Path $KeystorePath)) { throw "Key store not found at $KeystorePath (pass -KeystorePath)." }
$env:BOX_CLIENT_ID         = Ask 'Box client id' $ClientId
$env:BOX_CLIENT_SECRET     = AskSecret 'Box client secret (hidden)'
$env:BOX_ENTERPRISE_ID     = Ask 'Box enterprise id' $EnterpriseId
$env:BOX_PUBLIC_KEY_ID     = Ask 'Box public key id' $PublicKeyId
$env:BOX_KEYSTORE_PATH     = $KeystorePath
$env:BOX_KEYSTORE_PASSWORD = AskSecret 'Key store password (hidden)'

try {
  $repo = Split-Path $PSScriptRoot -Parent
  switch ($Mode) {
    'check' {
      Push-Location $repo
      cargo test --bin backend box_keystore_signs -- --ignored --nocapture
    }
    'box' {
      $env:BOX_TEST_PARENT_FOLDER_ID = Ask 'TEST Box folder id or URL (a throwaway folder is created inside it and deleted)' $TestParentFolder
      Push-Location $repo
      cargo test --bin backend box_live_round_trip -- --ignored --nocapture
    }
    'run' {
      $env:STORAGE_PROVIDER = 'box'
      # The Box folder (id or URL) inside which each request's TD-xxxx temp folder is created.
      # Blank = use each client's own Internal folder.
      $env:STORAGE_TEMP_PARENT_FOLDER = Ask 'Box folder id or URL for the TD-xxxx temp folders (blank = the client Internal folder)' $TempParentFolder
      # Each client's Internal and Client (External) folder ids come from the client record, so no
      # root ids are entered here. Pass -ExternalOff to skip filing to the Client folder.
      if ($ExternalOff) { $env:STORAGE_EXTERNAL_ENABLED = 'false' }
      $env:ENV_NAME = 'local'; $env:API_PORT = '8090'; $env:BACKEND_WORKER_STACK_MIB = '128'
      $env:OKTA_AUDIENCE = 'api://default'; $env:OKTA_ISSUER = 'https://example.okta.com/oauth2/default'; $env:OKTA_CLIENT_ID = 'example-client-id'
      # A 2 MB PDF is about 2.7 MB once base64 encoded; the default limit is 1 MiB. Bulk uploads send an
      # Excel file and a ZIP the same way, so 64 MiB (a ZIP of about 45 MB) is allowed.
      $env:APP_REQUEST_BODY_LIMIT_BYTES = '67108864'
      Push-Location (Join-Path $repo 'backend')
      & (Join-Path $repo 'target\debug\backend.exe')
    }
  }
}
finally {
  Pop-Location -ErrorAction SilentlyContinue
  foreach ($k in 'BOX_CLIENT_SECRET', 'BOX_KEYSTORE_PASSWORD', 'BOX_CLIENT_ID', 'BOX_ENTERPRISE_ID', 'BOX_PUBLIC_KEY_ID', 'BOX_KEYSTORE_PATH', 'BOX_TEST_PARENT_FOLDER_ID', 'STORAGE_PROVIDER', 'STORAGE_TEMP_PARENT_FOLDER', 'STORAGE_EXTERNAL_ENABLED') {
    Remove-Item "Env:$k" -ErrorAction SilentlyContinue
  }
}
