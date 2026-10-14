# Oyunu test ortam değişkenleriyle çalıştırıp ekran görüntüsü alır, zaman aşımında öldürür.
# Kullanım: powershell -File tools\shot.ps1 -Out C:\yol\a.png -At "20" -Env "AZM_GALLERY=1;AZM_YAW=3.14"
param(
    [string]$Out,
    [string]$At = "25",
    [string]$Env = "",
    [int]$Timeout = 120
)
$root = Split-Path -Parent $PSScriptRoot
# Önceki çalıştırmalardan kalan AZM_* değişkenlerini temizle (sızmasın).
Get-ChildItem Env: | Where-Object { $_.Name -like "AZM_*" } | ForEach-Object { Remove-Item "env:$($_.Name)" }
$env:AZM_SHOT = $Out
$env:AZM_SHOT_AT = $At
foreach ($kv in ($Env -split ";")) {
    if ($kv -match "^(.+?)=(.*)$") { Set-Item -Path "env:$($Matches[1])" -Value $Matches[2] }
}
$log = Join-Path $root "run_shot.log"
$p = Start-Process -FilePath (Join-Path $root "target\release\azeroth-metin.exe") -WorkingDirectory $root `
    -RedirectStandardError $log -RedirectStandardOutput (Join-Path $root "run_shot.out.log") -PassThru
if (-not $p.WaitForExit($Timeout * 1000)) {
    Write-Output "TIMEOUT - killing"
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
}
Write-Output "exit=$($p.ExitCode)"
Get-Content $log | Select-String "panic|ERROR" | Where-Object { $_.Line -notmatch "world/wmo|keepwall" } |
    Select-Object -First 15 | ForEach-Object { $_.Line.Substring(0, [Math]::Min(300, $_.Line.Length)) }
# Bu çalıştırmanın değişkenlerini de geri al.
Get-ChildItem Env: | Where-Object { $_.Name -like "AZM_*" } | ForEach-Object { Remove-Item "env:$($_.Name)" }
