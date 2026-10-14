# qa-transcribe-needs-audio — verification

Máquina: Windows 11 (Alienware 16), 2026-10-13. Verificado pelo próprio executor.

**PASS** em A1–A3: `transcribeAudio.test.tsx` A1 e A2 ok; `meetingKeys.test.tsx` ok; lint,
prettier, tsc e traduções limpos. `home.test.tsx`, `import.test.tsx` e `search.test.tsx` falham
igual na base (desde o #186), sem relação com este diff.

Não provado no app: mover para a lixeira uma reunião com áudio, restaurá-la sem o áudio e ver
"Transcrever" desligado com "O áudio desta reunião foi apagado"; abrir uma importação do Granola e
ver o cabeçalho sem "00:00".
