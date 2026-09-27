#Requires -Version 5.1
<#
.SYNOPSIS
    Spike 03a: mostra e esconde a pill de gravação do Fala -Cycles vezes e confere, a cada vez,
    se ela ficou visível, no topo (WS_EX_TOPMOST), sem roubar o foco, e se sumiu no cancelamento.

.DESCRIPTION
    Pré-requisito (docs/spikes/03-overlay-colagem.md): Fala rodando, com outra janela em foco
    (ex.: o Bloco de Notas). Cada ciclo roda `fala.exe --toggle-transcription` (chega à instância
    em execução pelo single-instance), sonda a janela de título `Recording` a cada -PollMs até
    -SettleMs, e roda `fala.exe --cancel`.

    stdout: só a tabela Markdown. stderr: progresso a cada 20 ciclos.
    Exit: 0 as quatro contagens iguais a -Cycles; 1 alguma menor; 2 fala.exe não roda ou a pill
    nunca apareceu nos 3 primeiros ciclos.
#>
param(
    [int]$Cycles = 200,
    [string]$FalaExe = 'fala.exe',
    [int]$SettleMs = 500,
    [int]$PollMs = 10
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

public static class Overlay03 {
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern IntPtr FindWindowW(string cls, string title);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll", EntryPoint = "GetWindowLongPtrW")]
    static extern IntPtr GetWindowLongPtr64(IntPtr h, int index);
    [DllImport("user32.dll", EntryPoint = "GetWindowLongW")]
    static extern int GetWindowLong32(IntPtr h, int index);
    // GetWindowLongPtrW só existe no user32 de 64 bits.
    public static long GetWindowLongPtr(IntPtr h, int index) {
        return IntPtr.Size == 8 ? GetWindowLongPtr64(h, index).ToInt64() : GetWindowLong32(h, index);
    }
}
'@

function Fail2([string]$why) {
    [Console]::Error.WriteLine("erro: $why")
    exit 2
}

if (-not (Get-Command $FalaExe -ErrorAction SilentlyContinue)) {
    Fail2 "não achei '$FalaExe' (passe -FalaExe com o caminho do fala.exe)"
}
$exe = (Get-Command $FalaExe).Source
# Sem uma instância rodando, `fala.exe --toggle-transcription` abriria o app e não voltaria.
if (-not (Get-Process -Name ([System.IO.Path]::GetFileNameWithoutExtension($exe)) -ErrorAction SilentlyContinue)) {
    Fail2 "o Fala não está rodando; abra o app antes do script"
}

function Invoke-Fala([string]$flag) {
    $p = Start-Process -FilePath $exe -ArgumentList $flag -PassThru -WindowStyle Hidden
    if (-not $p.WaitForExit(10000)) {
        $p.Kill()
        Fail2 "fala.exe $flag não voltou em 10 s"
    }
}

# Espera até -SettleMs pela pill no estado pedido; devolve os ms até lá, ou -1.
function Wait-Overlay([bool]$visible) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    while ($sw.ElapsedMilliseconds -le $SettleMs) {
        $h = [Overlay03]::FindWindowW([NullString]::Value, 'Recording')
        $now = ($h -ne [IntPtr]::Zero) -and [Overlay03]::IsWindowVisible($h)
        if ($now -eq $visible) { return @{ Ms = $sw.Elapsed.TotalMilliseconds; Hwnd = $h } }
        Start-Sleep -Milliseconds $PollMs
    }
    return @{ Ms = -1; Hwnd = [IntPtr]::Zero }
}

$visibleCount = 0; $topmost = 0; $focusKept = 0; $hidden = 0
$latencies = New-Object System.Collections.Generic.List[double]
for ($i = 1; $i -le $Cycles; $i++) {
    $focused = [Overlay03]::GetForegroundWindow()
    Invoke-Fala '--toggle-transcription'
    $shown = Wait-Overlay $true
    if ($shown.Ms -ge 0) {
        $visibleCount++
        $latencies.Add($shown.Ms)
        # GWL_EXSTYLE = -20; WS_EX_TOPMOST = 0x8.
        $ex = [Overlay03]::GetWindowLongPtr($shown.Hwnd, -20)
        if (($ex -band 0x8) -ne 0) { $topmost++ }
    }
    if ([Overlay03]::GetForegroundWindow() -eq $focused) { $focusKept++ }
    Invoke-Fala '--cancel'
    if ((Wait-Overlay $false).Ms -ge 0) { $hidden++ }
    if ($i -eq 3 -and $visibleCount -eq 0) {
        Fail2 "a janela 'Recording' nunca ficou visível nos 3 primeiros ciclos (o Fala está rodando? o modelo carregou?)"
    }
    if ($i % 20 -eq 0) { [Console]::Error.WriteLine("$i/$Cycles ciclos") }
    Start-Sleep -Milliseconds 300
}

$sorted = @($latencies | Sort-Object)
$p50 = '-'; $max = '-'
if ($sorted.Count -gt 0) {
    $p50 = '{0:F0}' -f $sorted[[int][math]::Floor(($sorted.Count - 1) / 2)]
    $max = '{0:F0}' -f $sorted[$sorted.Count - 1]
}
[Console]::Out.WriteLine('| cycles | overlay_visible | topmost | focus_kept | hidden_after_cancel | show_latency_ms_p50 | show_latency_ms_max |')
[Console]::Out.WriteLine('| ---: | ---: | ---: | ---: | ---: | ---: | ---: |')
[Console]::Out.WriteLine("| $Cycles | $visibleCount | $topmost | $focusKept | $hidden | $p50 | $max |")

if ($visibleCount -lt $Cycles -or $topmost -lt $Cycles -or $focusKept -lt $Cycles -or $hidden -lt $Cycles) { exit 1 }
exit 0
