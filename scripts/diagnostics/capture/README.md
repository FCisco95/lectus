# Diagnóstico isolado de captura

Este crate usa `src-tauri/src/audio/mod.rs` e `playback.rs` diretamente, com target próprio. Não compila a app Tauri nem a substitui. Sem argumentos, o binário executa a captura de 2 s já existente; **não o executar por rotina nem como preview**.

## Observação física de desligação/retoma USB

Executado uma vez: **`not_accepted_no_physical_absence_observed`**, sem ausência/regresso físico enumerados nos 90 s. Exit code 0 não significa PASS de reconexão. [Prova portátil](../../../docs/reviews/2026-10-21-microphone-reconnect-receipt.json).

Repetir apenas após nova prontidão humana, em janela coordenada para desligar/religar o microfone USB. A remoção também afeta a captura da app instalada; pode ser preciso voltar a abrir o mesmo executável instalado depois. O diagnóstico não reinicia a app. A pessoa foi avisada de que a primeira janela terminou e que não deveria retirar o cabo tardiamente.

```powershell
& .\scripts\diagnostics\capture\target\debug\lectus-capture-diagnostic.exe --watch-seconds 90 --receipt .context\2026-10-21-microphone-unplug-replug.json
```

1. Esperar por `stage=ready_for_human_unplug_replug` no JSON stdout. Antes desse evento, o helper confirma frames finitos numa baseline de 2 s.
2. Só então pedir à pessoa para retirar o cabo USB do microfone selecionado. Esperar `configured_input_absent`.
3. Pedir à pessoa para religar o mesmo cabo/dispositivo. Após `configured_input_returned_observing_original_stream`, aguardar o recibo final.

O modo nativo é `--watch-seconds 90 --receipt <ficheiro-novo.json>`. A presença é enumerada a cada 250 ms; falha de enumeração aborta, não prova ausência. O nome configurado tem de ser explícito e único. Nome diferente/fallback e alterações da configuração abortam. A identidade é por nome único, não por ID estável do hardware.

Após observar ausência e regresso, o helper mede 2 s de frames no stream original, regista os resultados e só então descarta esse stream e abre uma nova captura do mesmo nome para medir outros 2 s. `reproduced_original_stream_stalled_fresh_capture_live` significa original com zero frames e captura nova com frames, sem amostras não finitas. Não prova ditado completo nem recuperação da app instalada.

O wrapper opcional `watch.ps1` foi preparado e passou parsing PS5, mas **não foi executado e não é requisito do comando direto acima**. Acrescenta vigilância de PID/start time/path da app instalada e hash da configuração a cada 250 ms. Qualquer alteração aborta somente o filho diagnóstico. No comando nativo direto, a coordenação confirma a identidade da instalação antes/depois, separadamente; não se declara recuperação da app instalada a partir deste teste de componente.

Sem hotkeys, input, alterações de dispositivos/volume/configuração, áudio guardado ou transcrição. Volume é apenas lido antes/depois; uma mudança observada não é automaticamente atribuída a Lectus.

O recibo contém métricas, timestamps e hashes das fontes/binário/configuração/nome de input. Não contém áudio, fala bruta, chaves, nome do dispositivo ou caminhos pessoais. Sem evento físico observado, o resultado é `not_accepted`, nunca PASS.

Compilação isolada: `scripts\diagnostics\capture\build.cmd` (offline, target do próprio crate). Compilação passada em 21-10-2026; binário SHA256 `EF512BE1EA64AEEA3A0ACB624B8ADF9D9CB767388FFD87A31F88CBFFFFF080FE`. O binário ficou congelado para a execução coordenada. A compilação, por si só, não demonstra unplug/reconexão; o resultado da janela real está indicado acima.
