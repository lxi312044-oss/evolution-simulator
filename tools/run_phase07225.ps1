param(
    [ValidateSet('Pilot','FollowUp')][string]$Stage = 'Pilot',
    [int]$Ticks = 50000,
    [int]$StartSeed = 12345,
    [int]$Interval = 100,
    [double]$GerminationProbability = 0.005,
    [double]$SoilThreshold = 20,
    [int]$CheckInterval = 20,
    [int]$MaxBirths = 20,
    # FollowUp: 10 extra runs by default (x50: 6 seeds, x20: 4 seeds).
    # Adjust after inspecting Pilot results, keeping the sum <= 10.
    [hashtable]$FollowUpCounts = @{ 50 = 6; 20 = 4 },
    [switch]$DryRun
)
$ErrorActionPreference = 'Stop'
if ($Ticks -le 0 -or $Interval -le 0 -or ($Ticks % $Interval) -ne 0) { throw 'Invalid Ticks/Interval' }
if ($GerminationProbability -le 0 -or $GerminationProbability -gt 1 -or $CheckInterval -le 0 -or $MaxBirths -le 0) { throw 'Invalid germination parameters' }
$allowed = @(0, 4, 10, 20, 50)
$pilotCount = 4
$plan = @()
if ($Stage -eq 'Pilot') {
    foreach ($m in $allowed) {
        for ($i = 0; $i -lt $pilotCount; $i++) {
            $plan += [pscustomobject]@{ Multiplier = $m; Seed = ($StartSeed + $i) }
        }
    }
} else {
    $total = 0
    foreach ($key in $FollowUpCounts.Keys) {
        $m = [int]$key
        $count = [int]$FollowUpCounts[$key]
        if ($m -notin $allowed -or $count -lt 0) { throw "Invalid follow-up multiplier/count: $key=$count" }
        $total += $count
        for ($i = 0; $i -lt $count; $i++) {
            $plan += [pscustomobject]@{ Multiplier = $m; Seed = ($StartSeed + $pilotCount + $i) }
        }
    }
    if ($total -ne 10) { throw "FollowUp requires exactly 10 additional runs; specified $total" }
}
$root = Split-Path -Parent $PSScriptRoot
$out = Join-Path $root 'results/phase07225/runs'
$culture = [System.Globalization.CultureInfo]::InvariantCulture
$utf8 = New-Object System.Text.UTF8Encoding($false)
Write-Host "Phase 07.2.25 stage=$Stage planned=$($plan.Count) runs; germination=$GerminationProbability"
if (-not $DryRun) { New-Item -ItemType Directory -Path $out -Force | Out-Null }
foreach ($item in $plan) {
    $mult = $item.Multiplier
    $seed = $item.Seed
    $condition = if ($mult -eq 0) { 'B_no_toxin_cost' } else { 'F_strong_cost' }
    $level = if ($mult -eq 0) { 'control' } else { "x$mult" }
    $prefix = Join-Path $out "$condition-seed$seed-$level"
    $resource = "$prefix.resource-reproduction.csv"
    $life = "$prefix.life-events.csv"
    if ($DryRun) { Write-Host "DRY RUN $level seed=$seed"; continue }
    if ((Test-Path -LiteralPath $resource) -and (Test-Path -LiteralPath $life)) { Write-Host "SKIP $level seed=$seed"; continue }
    $cfg = Get-Content -LiteralPath (Join-Path $root "config/phase0722-$condition.json") -Raw -Encoding UTF8 | ConvertFrom-Json
    $cfg.seed = $seed
    if ($mult -ne 0) { $cfg.toxin.hp_damage_per_excess_unit = [double]$cfg.toxin.hp_damage_per_excess_unit * $mult }
    $cfgpath = "$prefix.json"
    [System.IO.File]::WriteAllText($cfgpath, ($cfg | ConvertTo-Json -Depth 100), $utf8)
    & cargo run --release -p sim-core --example resistance_life_07222 -- $cfgpath $Ticks $Interval $resource p50 $SoilThreshold.ToString($culture) $GerminationProbability.ToString($culture) $CheckInterval $MaxBirths $life
    if ($LASTEXITCODE -ne 0) { throw "Simulation failed: $level seed=$seed" }
}
if (-not $DryRun) {
    $counts = @{}
    foreach ($m in $allowed) { $level = if ($m -eq 0) { 'control' } else { "x$m" }; $counts[$level] = 4 }
    if ($Stage -eq 'FollowUp') {
        foreach ($key in $FollowUpCounts.Keys) {
            $m = [int]$key; $level = if ($m -eq 0) { 'control' } else { "x$m" }
            $counts[$level] += [int]$FollowUpCounts[$key]
        }
    }
    $countSpec = ($counts.Keys | Sort-Object | ForEach-Object { "$_=$($counts[$_])" }) -join ','
    & python (Join-Path $root 'tools/analyze_phase07225.py') --root $out --out (Join-Path $root 'results/phase07225/analysis') --ticks $Ticks --expected-counts $countSpec
    if ($LASTEXITCODE -ne 0) { throw 'Analysis failed' }
}
