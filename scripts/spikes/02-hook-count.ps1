#Requires -Version 5.1
<#
.SYNOPSIS
    Spike 02: conta os eventos do hook de teclado (handy-keys, WH_KEYBOARD_LL) que o Fala
    registra enquanto a própria janela do Fala (WebView2) tem o foco.

.DESCRIPTION
    Pré-requisitos (docs/spikes/02-hook-webview2.md): Fala rodando com --debug e o binding
    `transcribe` = F9 em push-to-talk. O script lê o fala.log enquanto ele cresce (o app apaga o
    arquivo ao passar de 500 KB) e conta as linhas
    `handy-keys event: binding=transcribe, ..., state=Pressed|Released` escritas depois do início.

    -Mode auto: injeta -Presses toques de F9 por SendInput com a janela do Fala em foco.
    -Mode manual: a pessoa digita na janela do Fala e toca F9 por -Minutes; compara com
    -ExpectedPresses.

    stdout: só a tabela Markdown. stderr: instruções e progresso.
    Exit: 0 contagens iguais ao esperado; 1 contagem diferente; 2 log ou janela não encontrados.
#>
param(
    [ValidateSet('auto', 'manual')][string]$Mode = 'auto',
    [int]$Presses = 300,
    [int]$IntervalMs = 2000,
    [int]$HoldMs = 50,
    [int]$Minutes = 10,
    [int]$ExpectedPresses = 0,
    [string]$LogDir = "$env:LOCALAPPDATA\br.com.augusto.fala\logs",
    [string]$WindowTitle = 'Fala'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class W {
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern IntPtr FindWindowW(string cls, string title);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    static extern int GetWindowTextW(IntPtr h, StringBuilder s, int n);

    [StructLayout(LayoutKind.Sequential)]
    struct KEYBDINPUT { public ushort wVk; public ushort wScan; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
    [StructLayout(LayoutKind.Sequential)]
    struct MOUSEINPUT { public int dx; public int dy; public uint mouseData; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
    [StructLayout(LayoutKind.Explicit)]
    struct INPUTUNION { [FieldOffset(0)] public MOUSEINPUT mi; [FieldOffset(0)] public KEYBDINPUT ki; }
    [StructLayout(LayoutKind.Sequential)]
    struct INPUT { public uint type; public INPUTUNION u; }
    [DllImport("user32.dll", SetLastError = true)]
    static extern uint SendInput(uint n, INPUT[] inputs, int size);

    // Um evento de teclado (INPUT_KEYBOARD = 1; KEYEVENTF_KEYUP = 2).
    public static void Key(ushort vk, bool up) {
        var i = new INPUT[1];
        i[0].type = 1;
        i[0].u.ki.wVk = vk;
        i[0].u.ki.dwFlags = up ? 2u : 0u;
        if (SendInput(1, i, Marshal.SizeOf(typeof(INPUT))) != 1)
            throw new System.ComponentModel.Win32Exception();
    }

    public static string Title(IntPtr h) {
        var sb = new StringBuilder(512);
        GetWindowTextW(h, sb, 512);
        return sb.ToString();
    }
}
'@

function Fail2([string]$why) {
    [Console]::Error.WriteLine("erro: $why")
    exit 2
}

$logPath = Join-Path $LogDir 'fala.log'
if (-not (Test-Path -LiteralPath $logPath)) {
    Fail2 "fala.log não encontrado em $LogDir (o Fala está rodando com --debug?)"
}
$hwnd = [W]::FindWindowW([NullString]::Value, $WindowTitle)
if ($hwnd -eq [IntPtr]::Zero) {
    Fail2 "janela '$WindowTitle' não encontrada (abra a janela do Fala)"
}
if ($Mode -eq 'manual' -and $ExpectedPresses -le 0) {
    Fail2 "no modo manual, informe -ExpectedPresses (quantos toques de F9 você vai dar)"
}

# Lê só o que o app escreveu depois do início, por um handle que fica aberto a execução inteira.
# O app apaga o fala.log ao passar de 500 KB (RotationStrategy::KeepOne) e cria outro; no Windows
# 10+ a exclusão tira o nome do diretório mas o handle aberto continua lendo o arquivo antigo.
# Na rotação, o handle antigo é drenado até o fim antes de abrir o novo, então nenhuma linha
# escrita entre duas leituras se perde.
$script:offset = (Get-Item -LiteralPath $logPath).Length
$script:pressed = 0
$script:released = 0
$script:rotations = 0
$script:pending = ''
$script:fs = $null
$script:reader = $null
function Open-Log {
    $script:fs = [System.IO.File]::Open($logPath, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read,
        [System.IO.FileShare]::ReadWrite -bor [System.IO.FileShare]::Delete)
    $script:fs.Seek($script:offset, [System.IO.SeekOrigin]::Begin) | Out-Null
    $script:reader = New-Object System.IO.StreamReader($script:fs, [System.Text.Encoding]::UTF8)
}
function Read-Log {
    $text = $script:pending + $script:reader.ReadToEnd()
    # Só linhas completas; uma linha pela metade fica para a próxima leitura.
    $cut = $text.LastIndexOf("`n")
    if ($cut -lt 0) { $script:pending = $text; return }
    $script:pending = $text.Substring($cut + 1)
    $script:offset += [System.Text.Encoding]::UTF8.GetByteCount($text.Substring(0, $cut + 1))
    foreach ($line in $text.Substring(0, $cut).Split("`n")) {
        if ($line -notlike '*handy-keys event: binding=transcribe,*') { continue }
        if ($line -like '*state=Pressed*') { $script:pressed++ }
        elseif ($line -like '*state=Released*') { $script:released++ }
    }
}
function Update-Counts {
    Read-Log
    # Arquivo no caminho menor que o que já lemos = o app o recriou.
    if ((Test-Path -LiteralPath $logPath) -and ((Get-Item -LiteralPath $logPath).Length -lt $script:offset)) {
        Read-Log
        $script:reader.Dispose()
        $script:fs.Dispose()
        $script:offset = 0
        $script:pending = ''
        $script:rotations++
        [Console]::Error.WriteLine("fala.log recriado pelo app (rotação $($script:rotations)); lendo o novo")
        Open-Log
        Read-Log
    }
}
Open-Log

$firstGap = '-'
if ($Mode -eq 'auto') {
    $sent = $Presses
    [W]::SetForegroundWindow($hwnd) | Out-Null
    Start-Sleep -Milliseconds 300
    if ([W]::GetForegroundWindow() -ne $hwnd) {
        Fail2 "não consegui pôr '$WindowTitle' em foco (o Windows bloqueou; clique na janela do Fala e rode de novo)"
    }
    [Console]::Error.WriteLine("injetando $Presses toques de F9 em '$WindowTitle' (~$([int]($Presses * $IntervalMs / 1000)) s); não mexa no teclado")
    for ($i = 1; $i -le $Presses; $i++) {
        $before = $script:pressed
        [W]::Key(0x78, $false)
        Start-Sleep -Milliseconds $HoldMs
        [W]::Key(0x78, $true)
        $at = (Get-Date).ToUniversalTime().ToString('HH:mm:ss.fff') + 'Z'
        $deadline = (Get-Date).AddMilliseconds($IntervalMs)
        while ((Get-Date) -lt $deadline) {
            Update-Counts
            Start-Sleep -Milliseconds 100
        }
        if ($firstGap -eq '-' -and $script:pressed -eq $before) { $firstGap = $at }
        if ($i % 20 -eq 0) {
            [Console]::Error.WriteLine("$i/$Presses enviados; pressed=$($script:pressed) released=$($script:released)")
        }
    }
}
else {
    $sent = $ExpectedPresses
    [Console]::Error.WriteLine("digite na janela do Fala e toque F9 a cada ~10 s, $ExpectedPresses vezes, por $Minutes min")
    $deadline = (Get-Date).AddMinutes($Minutes)
    while ((Get-Date) -lt $deadline) {
        Update-Counts
        Start-Sleep -Milliseconds 250
    }
}
Start-Sleep -Seconds 2
Update-Counts
$script:reader.Dispose()
$script:fs.Dispose()

$foreground = ([W]::Title([W]::GetForegroundWindow())) -replace '\|', '/'
[Console]::Out.WriteLine('| mode | presses_sent | pressed_logged | released_logged | first_gap_at | foreground_at_end |')
[Console]::Out.WriteLine('| --- | ---: | ---: | ---: | --- | --- |')
[Console]::Out.WriteLine("| $Mode | $sent | $($script:pressed) | $($script:released) | $firstGap | $foreground |")

if ($script:pressed -ne $sent -or $script:released -ne $sent) { exit 1 }
exit 0
