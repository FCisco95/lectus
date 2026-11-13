# Lectus — continuação da aceitação nativa

21 outubro 2026. Alterações apenas em diagnósticos e documentação; aplicação, configuração e instalação mantidas. A prova anterior de 244 testes nativos continua identificada no recibo da ronda de desenvolvimento; não foi repetida por rotina.

## Inserção real — passou

`scripts/diagnostics/insertion/run.ps1 -ActivationTimeoutMs 60000` passou no desktop Windows interativo. O executável auxiliar incluiu `src-tauri/src/injection/sendinput.rs` de produção e escreveu a frase sintética `Lectus: olá, English, 汉字.` numa TextBox WinForms própria. Foram verificados HWND, PID, foco do campo, ausência de modificadores premidos e inatividade de input antes de uma chamada batched SendInput. O próprio alvo confirmou `.Text` exato. Clipboard não foi usado.

O limite de espera pela ativação passou a ser parametrizável, mantendo 500 ms por omissão; o teste permitiu 60 s, mas terminou em aproximadamente 2,1 s. Não precisou de um clique Orca. Não foi desativado o guard de foco nem usada ativação forçada. As recusas anteriores de foreground continuam como prova histórica; a causa dessas recusas não foi estabelecida.

Esta prova testa inserção de texto do código atual, separada da aplicação Tauri completa. Não é prova do clipboard, de aplicações elevadas, de todas as aplicações desktop ou da instalação do candidato novo.

## Ditado humano pela instalação atual — passou

O teste seguinte foi realizado com intervenção humana, sem simular teclas ou áudio: hotkey → fala → transcrição → TextBox própria. O Lectus instalado escreveu `A reunião é na quinta-feira às 10 horas.` e acrescentou um timestamp novo ao histórico. A contagem continuou em 500, coerente com o limite do histórico. Configuração e executável conservaram os hashes.

Prova completa e recibo portátil: [aceitação humana](2026-10-21-live-dictation-acceptance.md) e [recibo](2026-10-21-live-dictation-receipt.json). O intervalo do teste inclui reação humana e fala; não é uma medição de latência do modelo. A configuração observada usa reconhecimento local, sem cleanup, tradução ou perfis, e não envia o áudio à cloud.

O teste cobre **a instalação anterior**, SHA256 `CD41E53A3A337B7770C99D9EA2E779546B679C39AD5EBA5C441E0DCCA117C406`, PID observado 25604, versão ainda 0.7.1. **Não aceita o candidato** `94E3BBAC07AEF58EE7A72FA516C2D6EBF40A3BCBD9AE54F8961E42DBCF210B6A` nem a sua correção de volume. Nenhuma app foi substituída, parada ou reiniciada por estes testes.

O primeiro lançamento do novo helper abortou antes da GUI devido à enumeração de arrays JSON em PowerShell 5.1. Corrigida a materialização de `ConvertFrom-Json`; fixtures com 0, 1 e 2 timestamps passaram antes do teste humano real. Não foi modificado o formato de histórico ou a aplicação para compensar uma falha do diagnóstico.

## Capacidades de teste e limites

- Desktop observado: sessão 1, estação `WinSta0`, thread e input desktop `Default`, foreground não nulo. A ferramenta `@oai/sky` inicializou mas não tinha native pipe; a via Orca anunciou capacidades Windows e listou as aplicações. Não se concluiu que o computador estava inacessível por falhar uma ferramenta.
- Cloud: chave não configurada, provider host `api.groq.com`, processamento cloud desativado. Não foi pedida chave em chat nem criada conta. Os diagnósticos de HTTP já verificados não provam inferência numa conta real.
- Instalação: HypervisorPresent não demonstra guest Windows. WMI confirmou Sandbox/Hyper-V desativados e WSL ativo; não há VM Windows operacional demonstrada. O token da shell não é elevado para UAC. Não foram ativadas funcionalidades, serviços ou reinícios por inferência.
- Preparado kit de aceitação NSIS em `scripts/diagnostics/installer-acceptance/`, com verificação de guest, manifesto de hashes e sentinelas sintéticas. [Relatório do kit](2026-10-21-native-install-acceptance.md). Preflight/guards são distintos da execução real de instalação/update/recovery/uninstall, que continua pendente.

## Observador de desconexão — executado, não aceite

Foi executado o modo `--watch-seconds 90` do diagnóstico de captura, com o módulo `PersistentCapture` de produção incluído diretamente. Após 2 s de baseline emitiu `ready_for_human_unplug_replug`, e só então foi solicitada intervenção física. O período terminou sem ausência ou regresso do dispositivo observados. O operador não confirmou a intervenção durante a janela; não se infere que tenha ocorrido uma desconexão.

Resultado: **`not_accepted_no_physical_absence_observed`**, apesar de o programa ter concluído normalmente. Baseline: 36.320 frames; janela: 1.440.320 frames, 90.020 ms de áudio em memória, sem amostras inválidas. Não houve comparação de stream original com captura nova, pelo que não foi demonstrada recuperação ou falha de recuperação. Configuração permaneceu igual; volume observado nesta janela foi 23% antes/depois, sem mutações pelo helper. Nenhum áudio foi guardado.

[Recibo físico redigido](2026-10-21-microphone-reconnect-receipt.json), sem nome/ID do microfone ou endpoint. O teste não abriu, fechou ou substituiu Lectus. Foi publicada a instrução de manter o cabo ligado após o fim da janela para evitar uma ação física tardia sem observador. Numa repetição, coordenar prontidão humana antes de iniciar o relógio e só dar a instrução de retirar/religar após readiness.

Desligação física do microfone, suspensão/retoma, falhas reais de transcrição e aceitação da versão nova continuam a precisar de experiências controladas. Nenhuma alteração de produção à captura/inserção/atalhos foi autorizada ou aplicada nesta continuação; não se corrige um risco de captura por hipótese. Publicação/packaging continuam bloqueados pelos dados do editor e revisão dos termos/licenças.
