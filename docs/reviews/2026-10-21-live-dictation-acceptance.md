# Lectus — aceitação humana de ditado em campo próprio

Estado: **PASS REAL em 21-10-2026**: pessoa → hotkey → fala → transcrição → campo WinForms próprio, usando a instalação anterior. Primeiro lançamento abortou antes da GUI; o helper foi corrigido antes da execução bem-sucedida.

## Resultado observado

A pessoa confirmou que iria ditar e realizou o teste na janela própria. O recibo registou início `2026-10-21T20:24:30.4037640+00:00` e conclusão `2026-10-21T20:24:46.7651583+00:00`, preservados como strings UTC originais. O texto final de teste foi `A reunião é na quinta-feira às 10 horas.\r\n`, 42 caracteres, SHA256 `3E04EBE293A1D0C501D9641305A860A7193D5A13A75954D5B8C40E5C19528615`.

Foreground, PID próprio e controlo TextBox foram confirmados. Não chegou texto fora do foco próprio. Houve timestamp novo de histórico posterior ao início; a contagem continuou em **500 → 500**, coerente com o limite de 500 entradas. A prova não depende de aumento da contagem, e nenhum texto anterior do histórico foi preservado.

Instalação 0.7.1, PID **25604**, SHA256 antes/depois `CD41E53A3A337B7770C99D9EA2E779546B679C39AD5EBA5C441E0DCCA117C406`. Configuração SHA256 antes/depois `5E967BB6A651402C74912248209E2AEDE1C649B748EEE578989AD0BB4332AB2E`. Sem input sintético, captura adicional pelo helper ou substituição da app. Não houve alterações à pipeline de captura/inserção/atalhos.

O intervalo de **16,36 s inclui reação humana e fala**; não é medição de latência do modelo nem das etapas isoladas. A conclusão cobre a instalação anterior, não o candidato de desenvolvimento nem a sua correção de volume.

Fonte local ignorada pelo Git: `scripts/diagnostics/live-dictation/receipts/2026-10-21-installed-baseline.json`, SHA256 `AF0ABDA87A4503C08504050179174DC2922C2DFDFE63D26F86DA4998B1FB3FA0`. [Recibo portátil redigido](2026-10-21-live-dictation-receipt.json), sem caminhos pessoais nem texto anterior do histórico. A única frase bruta é a frase canónica conhecida deste teste.

## Âmbito e segurança

`scripts/diagnostics/live-dictation/run.ps1` abre um único campo WinForms descartável. A pessoa clica nesse campo, segura Ctrl direito, diz **“A reunião é na quinta-feira às dez horas”** e solta a tecla. O helper não simula voz/teclas, não captura áudio, não altera definições, não lê janelas alheias e não abre, fecha ou substitui Lectus.

O teste usou a app **já instalada e em execução**, hash revisto `CD41E53A3A337B7770C99D9EA2E779546B679C39AD5EBA5C441E0DCCA117C406`. Não aceita o candidato de desenvolvimento `94E3BBAC07AEF58EE7A72FA516C2D6EBF40A3BCBD9AE54F8961E42DBCF210B6A` nem a correção de volume nele existente.

## Critérios do recibo

1. Confirmação do foreground, PID próprio e controlo TextBox. Perda de foco apenas aguarda a pessoa; chegada de texto sem foco comprovado aborta.
2. Configuração local sem cloud/cleanup/perfis/tradução e atalhos previstos; configuração não modificada durante o teste. Processo instalado e hash do executável preservados.
3. Frase reconhecível no campo próprio, palavras esperadas e timestamp de histórico posterior ao início. Metadados do histórico são lidos, mas o texto anterior não é inspecionado nem registado.
4. Prazo máximo de 180 s. Texto alternativo fica apenas como comprimento/SHA256/keywords. Conteúdo bruto só é guardado quando corresponde exatamente à frase canónica conhecida, tolerando pontuação, acentos e “10”.

O critério confirma um fluxo humano observado em aplicação real própria; não mede qualidade geral, latência de inferência isolada, microfone desligado, troca de dispositivo, suspensão/retoma, cloud ou Windows→Mac. A própria pessoa deve falar a frase, sem a escrever/colar.

## Execução coordenada

Não lançar antes de avisar a pessoa de que aparecerá a janela do teste e de que deverá ditar a frase.

```powershell
powershell.exe -NoProfile -STA -File .\scripts\diagnostics\live-dictation\run.ps1
```

Recibos ficam em `scripts/diagnostics/live-dictation/receipts/` (ignorados pelo Git). O resumo portátil deve conservar o resultado, hashes, timestamps e limites, sem conteúdo de fala alternativo.

## Verificação do helper

Windows PowerShell 5.1: parsing estático passou; AST sem comandos `Start-Process`, `Stop-Process` ou `SendKeys`. Normalização isolada da frase portuguesa com acentos/pontuação passou e frase alternativa não foi confundida com a canónica. Estas verificações não abriram o helper, não enviaram input e não são ditado real.

O guard de publicação do texto bruto usa a string original, com regex limitada à frase canónica. Foram recusados sufixos adicionais latinos, chineses e numéricos; a normalização de keywords, isoladamente, não autoriza guardar texto bruto.

O primeiro lançamento coordenado encontrou incompatibilidade de enumeração do array JSON em Windows PowerShell 5.1: `@(Get-Content | ConvertFrom-Json)` conservava o array raiz como um elemento e o cast de `.timestamp` falhava. Abortou antes de abrir GUI e antes de input ou novo histórico. Corrigido materializando o resultado de `ConvertFrom-Json` numa variável antes de `@($parsed)`. A função real extraída do AST passou fixtures isoladas de 0, 1 e 2 entradas, contendo apenas timestamps, confirmando contagem e timestamp máximo. Nenhum ditado foi simulado nesses checks.

Revisão das barreiras: as únicas importações nativas são `GetForegroundWindow`, `GetFocus` e `GetWindowThreadProcessId`, todas de leitura; não há chamadas de ativação forçada nem simulação de input. A pessoa obtém o foco clicando no próprio campo.

## Observação separada de desligação física do microfone

O observador isolado `scripts/diagnostics/capture` foi compilado e executado uma vez, usando a implementação de produção de `PersistentCapture`. **Não houve ausência física enumerada durante os 90 s; resultado `not_accepted_no_physical_absence_observed`**, apesar do exit code 0 do processo. Não é PASS de reconexão nem reprodução de falha.

Janela nativa `2026-10-21T20:31:51.513Z`–`2026-10-21T20:33:24.425Z` (UTC convertido dos milissegundos originais por Node). Baseline: 8.000 frames de pre-roll e 36.320 frames live; observação: 1.440.320 frames, equivalentes a 90.020 ms de áudio. Zero amostras não finitas. O microfone configurado continuou presente, sem evento de ausência/regresso; não se chegou à comparação stream original versus nova captura.

Configuração inalterada; leitura de volume antes/depois em 0,23000003397464752, mute false e mesmo endpoint, sem mudança observada. Sem WAV, transcrição, hotkeys/input sintético, alteração de dispositivos/configuração ou abertura/fecho/substituição da app instalada. O teste não demonstra recuperação da app instalada, suspensão/retoma ou causa da queda de volume.

[Recibo portátil redigido](2026-10-21-microphone-reconnect-receipt.json), sem hashes do nome do input, endpoint ou configuração. Fonte local `.context/2026-10-21-microphone-unplug-replug.json`, SHA256 `7FEC0BED939C308F13EA8C19DA0C47D6D5F48A68C9E5CA5B84C2E1C6B6B90E1B`. Nenhum áudio foi preservado.

A janela terminou e a pessoa foi avisada para **não desligar o cabo depois do prazo**. Outra tentativa exige nova prontidão humana e uma janela coordenada; não repetir automaticamente.
