$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
Push-Location $repoRoot
try {
    $clangCommand = Get-Command clang.exe -ErrorAction SilentlyContinue
    if ($clangCommand) {
        $clang = $clangCommand.Source
    } else {
        $clang = Join-Path $env:ProgramFiles 'LLVM\bin\clang.exe'
    }
    if (-not (Test-Path -LiteralPath $clang -PathType Leaf)) {
        throw "Could not find native clang.exe on PATH or at '$clang'."
    }
    $clang = (Resolve-Path -LiteralPath $clang).Path
    Write-Host "Using native compiler: $clang"

    $outputRoot = Join-Path ([System.IO.Path]::GetTempPath()) "oot synthetic build $([guid]::NewGuid().ToString('N'))"
    Write-Host "Output root: $outputRoot"
    & cargo +1.100.0 run --locked --manifest-path crates/build/Cargo.toml --example smoke -- $clang $outputRoot
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo smoke example failed with exit code $LASTEXITCODE."
    }

    $entries = @(Get-ChildItem -LiteralPath $outputRoot -Force)
    $generations = @($entries | Where-Object { $_.PSIsContainer -and $_.Name -like 'generation-*' })
    $staging = @($entries | Where-Object { $_.Name -like 'staging-*' })
    if ($generations.Count -ne 1) {
        throw "Expected exactly one published generation; found $($generations.Count)."
    }
    if ($staging.Count -ne 0) {
        throw "Found staging entries after publication: $($staging.Name -join ', ')"
    }
    if ($entries.Count -ne 1) {
        throw "Expected the generation to be the only output-root entry; found $($entries.Count)."
    }

    $probe = Join-Path $generations[0].FullName 'probe.exe'
    if (-not (Test-Path -LiteralPath $probe -PathType Leaf)) {
        throw "Published executable is missing: $probe"
    }
    $probeOutput = @(& $probe)
    if ($LASTEXITCODE -ne 0) {
        throw "Published probe.exe failed with exit code $LASTEXITCODE."
    }
    if (($probeOutput -join "`n") -ne 'synthetic-build-ok') {
        throw "Published probe.exe returned unexpected output: $($probeOutput -join ', ')"
    }
    Write-Host "Published probe.exe passed: $($probeOutput -join '')"
    Write-Host "Generation count: $($generations.Count); staging entry count: $($staging.Count)"

    $originalCompiler = [Environment]::GetEnvironmentVariable('OOT_BUILD_TEST_COMPILER')
    try {
        $env:OOT_BUILD_TEST_COMPILER = $clang
        & cargo +1.100.0 test --locked -p oot-build-probe --test native_failures -- --ignored
        if ($LASTEXITCODE -ne 0) {
            throw "Native regression check failed with exit code $LASTEXITCODE."
        }
    } finally {
        [Environment]::SetEnvironmentVariable('OOT_BUILD_TEST_COMPILER', $originalCompiler)
    }
} finally {
    Pop-Location
}
