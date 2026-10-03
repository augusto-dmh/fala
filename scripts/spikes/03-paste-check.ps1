#Requires -Version 5.1
<#
.SYNOPSIS
    Spike 03b: conduz a matriz app × tipo de clipboard × repetição da colagem do Fala e registra
    se o clipboard voltou ao original e se a frase ditada apareceu no app.

.DESCRIPTION
    Pré-requisitos (docs/spikes/03-overlay-colagem.md): Fala rodando com paste_method = CtrlV,
    clipboard_handling = DontModify e o modelo carregado; os apps de -Apps abertos. Em cada
    combinação o script põe no clipboard um texto FALA-SPIKE-<guid> ou um bitmap 8x8 (sem texto),
    dá 5 s para você focar o campo de texto do app, liga a transcrição, espera -SpeakSeconds
    enquanto você fala -Phrase, desliga, espera a colagem, confere o clipboard e pergunta se a
    frase apareceu (volte ao terminal para responder).

    stdout: só a tabela Markdown. stderr: contagem regressiva e perguntas.
    Exit: 0 todo restored e pasted = y; 1 alguma linha n; 2 fala.exe não roda ou clipboard
    inacessível (antes da primeira combinação).
#>
param(
    [string[]]$Apps = @('chrome', 'Code', 'Teams', 'WindowsTerminal', 'notepad'),
    [int]$Repeats = 3,
    [string[]]$Kinds = @('text', 'image'),
    [string]$FalaExe = 'fala.exe',
    [string]$Phrase = 'teste um dois três',
    [int]$SpeakSeconds = 4,
    [int]$PasteWaitMs = 3000
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms, System.Drawing

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
try {
    if ([Threading.Thread]::CurrentThread.GetApartmentState() -ne 'STA') { throw 'a thread não é STA' }
    [void][Windows.Forms.Clipboard]::ContainsText()
}
catch {
    Fail2 "o clipboard não pode ser lido ($_); rode no powershell.exe ou no pwsh com -STA"
}

function Invoke-Fala([string]$flag) {
    $p = Start-Process -FilePath $exe -ArgumentList $flag -PassThru -WindowStyle Hidden
    if (-not $p.WaitForExit(10000)) {
        $p.Kill()
        throw "fala.exe $flag não voltou em 10 s"
    }
}

# Bytes ARGB de um bitmap, para comparar pixel a pixel.
function Get-Pixels([System.Drawing.Bitmap]$bmp) {
    $rect = New-Object System.Drawing.Rectangle 0, 0, $bmp.Width, $bmp.Height
    $data = $bmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly,
        [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    try {
        $bytes = New-Object byte[] ($data.Stride * $bmp.Height)
        [System.Runtime.InteropServices.Marshal]::Copy($data.Scan0, $bytes, 0, $bytes.Length)
        return , $bytes
    }
    finally {
        $bmp.UnlockBits($data)
    }
}

function New-Pattern {
    $bmp = New-Object System.Drawing.Bitmap 8, 8
    $rng = New-Object System.Random
    for ($x = 0; $x -lt 8; $x++) {
        for ($y = 0; $y -lt 8; $y++) {
            $bmp.SetPixel($x, $y, [System.Drawing.Color]::FromArgb(255, $rng.Next(256), $rng.Next(256), $rng.Next(256)))
        }
    }
    return $bmp
}

function Test-Restored([string]$kind, $original) {
    if ($kind -eq 'text') {
        return [Windows.Forms.Clipboard]::ContainsText() -and ([Windows.Forms.Clipboard]::GetText() -ceq $original)
    }
    if ([Windows.Forms.Clipboard]::ContainsText() -or -not [Windows.Forms.Clipboard]::ContainsImage()) { return $false }
    $now = New-Object System.Drawing.Bitmap ([Windows.Forms.Clipboard]::GetImage())
    if ($now.Width -ne $original.Width -or $now.Height -ne $original.Height) { return $false }
    $a = Get-Pixels $original
    $b = Get-Pixels $now
    return [System.Linq.Enumerable]::SequenceEqual([byte[]]$a, [byte[]]$b)
}

$rows = New-Object System.Collections.Generic.List[string]
$restoredYes = 0; $pastedYes = 0; $total = 0
foreach ($app in $Apps) {
    foreach ($kind in $Kinds) {
        for ($r = 1; $r -le $Repeats; $r++) {
            # Uma combinação que falha (clipboard ocupado, fala.exe sem resposta) vira linha n/n.
            try {
                if ($kind -eq 'text') {
                    $original = 'FALA-SPIKE-' + [guid]::NewGuid().ToString()
                    [Windows.Forms.Clipboard]::SetText($original)
                }
                else {
                    $original = New-Pattern
                    [Windows.Forms.Clipboard]::SetImage($original)
                }
                [Console]::Error.WriteLine("[$app · $kind · $r/$Repeats] foque $app num campo de texto")
                for ($s = 5; $s -ge 1; $s--) {
                    [Console]::Error.WriteLine("  $s...")
                    Start-Sleep -Seconds 1
                }
                Invoke-Fala '--toggle-transcription'
                [Console]::Error.WriteLine("  fale agora: '$Phrase'")
                Start-Sleep -Seconds $SpeakSeconds
                Invoke-Fala '--toggle-transcription'
                Start-Sleep -Milliseconds $PasteWaitMs

                $restored = if (Test-Restored $kind $original) { 'y' } else { 'n' }
                do {
                    $answer = Read-Host "  a frase apareceu em ${app}? (y/n)"
                } while ($answer -notin @('y', 'n'))
            }
            catch {
                [Console]::Error.WriteLine("  falhou: $_")
                $restored = 'n'
                $answer = 'n'
            }
            $total++
            if ($restored -eq 'y') { $restoredYes++ }
            if ($answer -eq 'y') { $pastedYes++ }
            $rows.Add("| $app | $kind | $r | $restored | $answer |")
        }
    }
}

[Console]::Out.WriteLine('| app | kind | repeat | restored | pasted |')
[Console]::Out.WriteLine('| --- | --- | ---: | --- | --- |')
foreach ($row in $rows) { [Console]::Out.WriteLine($row) }
[Console]::Out.WriteLine("| total | - | $total | $restoredYes | $pastedYes |")

if ($restoredYes -lt $total -or $pastedYes -lt $total) { exit 1 }
exit 0
