$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$coreDir = Join-Path $scriptDir "..\core"
Push-Location $coreDir
$output = cargo test -- --list 2>&1 | Out-String
Pop-Location
$lines = $output -split "`n"

$summary = @{}
$current = $null

foreach ($line in $lines) {
    if ($line -match "Running.*\\deps\\([^-]+)") {
        $current = $Matches[1]
    }
    elseif ($line -match "Running tests\\(.+?)\.rs") {
        $current = $Matches[1]
    }
    elseif ($line -match "Doc-tests") {
        $current = "doc-tests"
    }
    elseif ($line -match "(\d+) tests?, \d+ benchmarks") {
        if ($current -and [int]$Matches[1] -gt 0) {
            $summary[$current] = [int]$Matches[1]
        }
        $current = $null
    }
}

$total = 0
$summary.GetEnumerator() | Sort-Object Value -Descending | ForEach-Object {
    Write-Host ("{0,-30} {1,3}" -f $_.Key, $_.Value)
    $total += $_.Value
}
Write-Host ("-" * 34)
Write-Host ("{0,-30} {1,3}" -f "TOTAL", $total)

