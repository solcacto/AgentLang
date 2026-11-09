param()

$ErrorActionPreference = "Stop"

$nl = "`n"

$sourceFile = $env:AGENTLANG_SOURCE_FILE
$diagFile = $env:AGENTLANG_DIAGNOSTICS_FILE

if (-not $sourceFile -or -not $diagFile) {
    Write-Host "missing env vars"
    exit 1
}

# Read source without BOM (avoids Set-Content UTF-8 corruption).
$source = [System.IO.File]::ReadAllText($sourceFile)

# Read diagnostics JSON.
$raw = [System.IO.File]::ReadAllText($diagFile)
$diags = $raw | ConvertFrom-Json

if (-not $diags) {
    exit 0
}

$changed = $false

foreach ($d in $diags) {
    $code = $d.code
    $msg = $d.message
    $symbol = $d.symbol
    $expected = $d.expected
    $repairExpected = if ($d.repair) { $d.repair.expected_type } else { $null }

    switch ($code) {
        "IMMUTABLE_ASSIGNMENT" {
            # let name = ...  →  let mut name = ...
            if ($symbol) {
                $pattern = "let $([regex]::Escape($symbol)) ="
                $replacement = "let mut $symbol ="
                if ($source -match $pattern) {
                    $source = $source -replace $pattern, $replacement
                    $changed = $true
                }
            }
        }

        "EFFECT_VIOLATION" {
            # Add missing effect(s) to the calling function's effects clause.
            # The message names the calling function, e.g. "Function `main` may not call ...".
            if ($repairExpected -and $msg -match "Function `(\w+)`") {
                $caller = $matches[1]
                $effects = $repairExpected -split ', ' | ForEach-Object { $_.Trim() }
                $effectList = ($effects -join ', ')
                $lines = $source -split $nl
                $newLines = @()
                $done = $false
                $funcMarker = 'function ' + $caller + '('
                foreach ($line in $lines) {
                    if (-not $done -and $line.Contains($funcMarker)) {
                        $effIdx = $line.IndexOf('effects(')
                        if ($effIdx -ge 0) {
                            # Function already has effects; add missing ones.
                            $closeIdx = $line.IndexOf(')', $effIdx)
                            if ($closeIdx -ge 0) {
                                $existing = $line.Substring($effIdx + 8, $closeIdx - $effIdx - 8)
                                $existingEffects = $existing -split ',' | ForEach-Object { $_.Trim() }
                                $allEffects = @()
                                foreach ($e in $existingEffects) { if ($e) { $allEffects += $e } }
                                foreach ($e in $effects) { if ($e -and $allEffects -notcontains $e) { $allEffects += $e } }
                                $newEffectStr = ($allEffects -join ', ')
                                $line = $line.Substring(0, $effIdx) + 'effects(' + $newEffectStr + ')' + $line.Substring($closeIdx + 1)
                                $done = $true
                            }
                        } else {
                            # No effects clause yet; add one before the opening brace.
                            $braceIdx = $line.LastIndexOf('{')
                            if ($braceIdx -ge 0) {
                                $prefix = $line.Substring(0, $braceIdx).TrimEnd()
                                $line = "$prefix effects(" + $effectList + ") {"
                                $done = $true
                            }
                        }
                    }
                    $newLines += $line
                }
                if ($done) {
                    $source = $newLines -join $nl
                    $changed = $true
                }
            }
        }

        "TYPE_MISMATCH" {
            # Return type mismatch: replace the returned expression with a
            # value of the expected type.
            if ($msg -like "*Return type mismatch*" -and $expected) {
                $exp = $expected.ToString()
                $replacement = switch ($exp) {
                    "i32" { "return 0;" }
                    "i64" { "return 0;" }
                    "Bool" { "return true;" }
                    "String" { 'return "";' }
                    default { "return 0;" }
                }
                if ($source -match 'return\s+.*?;') {
                    $source = $source -replace 'return\s+.*?;', $replacement
                    $changed = $true
                }
            }
            # Condition must be Bool: change `if 1` / `while i` → `if true` / `while true`
            if ($msg -match "condition must be.*Bool" -and $expected -eq "Bool") {
                $source = $source -replace '(if|while)\s+\w+\s*\{', '${1} true {'
                $changed = $true
            }
        }

        "MISSING_RETURN" {
            # Add `return 0;` before the last closing brace.
            if ($source -match '\}$') {
                $source = $source -replace '\}$', "    return 0;$nl}"
                $changed = $true
            }
        }

        "UNDEFINED_SYMBOL" {
            # Declare the missing symbol before the first return.
            if ($symbol -and $expected) {
                $exp = $expected.ToString()
                $init = switch ($exp) {
                    "i32" { 0 }
                    "i64" { 0 }
                    "Bool" { $false }
                    "String" { '""' }
                    default { 0 }
                }
                $decl = "    let $symbol = $init;$nl"
                if ($source -match 'return\s') {
                    $source = $source -replace '(return\s)', $decl + '$1'
                    $changed = $true
                }
            }
        }

        default {
            # No generic repair for this code.
        }
    }
}

if ($changed) {
    [System.IO.File]::WriteAllText($sourceFile, $source)
    exit 0
} else {
    exit 0
}
