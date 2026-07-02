# feature-gate-check.ps1
# Verifies each non-default feature gate compiles individually.
# Exits 0 on all-pass, non-zero on any failure.

function Invoke-CargoCheck {
    param(
        [string[]]$Arguments,
        [string]$Label
    )

    Write-Host -NoNewline "  CHECK $Label ... "

    $output = & cargo @Arguments 2>&1
    $exitCode = $LASTEXITCODE

    if ($exitCode -eq 0) {
        Write-Host "PASS" -ForegroundColor Green
        return $true
    } else {
        Write-Host "FAIL (exit $exitCode)" -ForegroundColor Red
        return $false
    }
}

# Feature gate test matrix
$tests = @(
    # agent-diva-agent: default=[], mentle
    @{ crate = "agent-diva-agent"; feature = "mentle"; noDefault = $false },

    # agent-diva-core: default=[], mentle
    @{ crate = "agent-diva-core"; feature = "mentle"; noDefault = $false },

    # agent-diva-sandbox: default=["manager","platform"]
    @{ crate = "agent-diva-sandbox"; feature = "manager"; noDefault = $true },
    @{ crate = "agent-diva-sandbox"; feature = "orchestrator"; noDefault = $true },
    @{ crate = "agent-diva-sandbox"; feature = "platform"; noDefault = $true },
    @{ crate = "agent-diva-sandbox"; feature = "platform-windows"; noDefault = $true },
    @{ crate = "agent-diva-sandbox"; feature = "platform-macos"; noDefault = $true },
    @{ crate = "agent-diva-sandbox"; feature = "platform-linux"; noDefault = $true },
    @{ crate = "agent-diva-sandbox"; feature = "approval"; noDefault = $true },
    @{ crate = "agent-diva-sandbox"; feature = "guardian"; noDefault = $true },
    @{ crate = "agent-diva-sandbox"; feature = "filesystem"; noDefault = $true }
)

$passCount = 0
$failCount = 0
$failList = @()

Write-Host "=== Feature Gate Check ===" -ForegroundColor Cyan
Write-Host "Testing $($tests.Count) feature combinations`n"

foreach ($t in $tests) {
    $crate = $t.crate
    $feature = $t.feature
    $noDefault = $t.noDefault

    if ($noDefault) {
        $label = "$crate --no-default-features --features $feature"
        $args = @("check", "-p", $crate, "--no-default-features", "--features", $feature)
    } else {
        $label = "$crate --features $feature"
        $args = @("check", "-p", $crate, "--features", $feature)
    }

    $ok = Invoke-CargoCheck -Arguments $args -Label $label
    if ($ok) { $passCount++ } else { $failCount++; $failList += $label }
}

# Also test --no-default-features for crates that have non-empty defaults
Write-Host ""
$ok = Invoke-CargoCheck -Arguments @("check", "-p", "agent-diva-sandbox", "--no-default-features") -Label "agent-diva-sandbox --no-default-features"
if ($ok) { $passCount++ } else { $failCount++; $failList += "agent-diva-sandbox --no-default-features" }

Write-Host ""
Write-Host "=== Summary ===" -ForegroundColor Cyan
Write-Host "  Passed: $passCount"
Write-Host "  Failed: $failCount"

if ($failCount -gt 0) {
    Write-Host "`n  Failed checks:" -ForegroundColor Red
    foreach ($f in $failList) {
        Write-Host "    - $f" -ForegroundColor Red
    }
    Write-Host ""
    exit 1
}

Write-Host "`n  All feature gates passed!" -ForegroundColor Green
exit 0
