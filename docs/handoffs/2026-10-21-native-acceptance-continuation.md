# Lectus — fiabilidade, downloads e modelos

## Metadata

- Project: Lectus
- Last Updated: 2026-10-21
- Branch: feat/remote-dictation
- Status: trabalho local, não committed/publicado/instalado; preservar toda a árvore suja e o trabalho anterior.
- Snapshot kind: handoff; reason: aceitação humana da instalação anterior, inserção real e preparação NSIS isolada.

## TL;DR

**244 testes nativos passaram, 0 falhas, 10 ignorados e 2 exclusões de mutação OS; frontend e executável de desenvolvimento compilados. A instalação atual continua intacta.** Relatório central: `docs/reviews/2026-10-21-reliability-and-models.md`.

Pausa/retoma verificadas com VAD oficial e transporte terminado/reiniciado em processos reais. Volume A→B corrigido após aprovação específica; captura real de 2 segundos passou. A continuação já validou inserção SendInput real e ditado humano completo pela instalação anterior; o candidato novo continua não instalado. O observador USB terminou sem ausência/regresso detetados, sem aceitar reconexão. Desligação/suspensão e NSIS em VM continuam pendentes.

## Continuação nativa — 2026-10-21

- Inserção direta de produção passou no campo WinForms próprio, incluindo PT/EN/chinês, sem clipboard.
- Ditado humano passou pela app instalada: a frase conhecida ficou no campo próprio e houve timestamp novo de histórico (500→500 pelo limite). Binário/configuração inalterados; nenhuma tecla/voz simulada.
- Provas: `docs/reviews/2026-10-21-live-dictation-acceptance.md`, `docs/reviews/2026-10-21-live-dictation-receipt.json` e `docs/reviews/2026-10-21-native-acceptance-continuation.md`. Estes testes não aceitam o candidato novo.
- Kit NSIS preparado/revisto em `scripts/diagnostics/installer-acceptance/`; cinco guards corrigidos e checks PS5/PS7 passaram. NSIS não executado: Sandbox/Hyper-V desativados, sem guest Windows operacional nem instaladores baseline/candidato elegíveis completos.
- Observador USB90s executado, mas NOT_ACCEPTED: não houve ausência/regresso observados. Não há reprodução física para justificar corrigir a captura; obter prontidão humana antes de repetir. Recibo: `docs/reviews/2026-10-21-microphone-reconnect-receipt.json`. Manter o cabo ligado até nova janela coordenada.
- Recibo final desta continuação: `docs/reviews/2026-10-21-native-acceptance-receipt.json`; app/candidato e 26 fontes da ronda anterior conferidos inalterados. Não foi repetida a suite nativa sem mudança de produção.
- Cloud real pendente: não existe chave configurada; nenhuma chave pedida em chat, conta ou serviço criado. Captura de produção permanece sem novas alterações não aprovadas.

## Current Objective

Prosseguir por uma desconexão/reconexão física coordenada e pela aceitação do candidato quando elegível. Ditado humano e inserção em campo próprio já passaram na instalação anterior; NSIS exige um Windows guest descartável e packages elegíveis. Não substituir a instalação atual nem contornar gates legais. A queda Bluetooth intermitente não foi reproduzida; não a declarar corrigida.

## Current State

- Candidato: `src-tauri/target/release/chirp.exe`, ainda versão 0.7.1, 77.940.736 bytes; SHA256 `94E3BBAC07AEF58EE7A72FA516C2D6EBF40A3BCBD9AE54F8961E42DBCF210B6A`. Não instalado, não aceite como distribuição.
- Instalação observada: `%LOCALAPPDATA%\Lectus\chirp.exe`, SHA256 `CD41E53A3A337B7770C99D9EA2E779546B679C39AD5EBA5C441E0DCCA117C406`, PID 25604 à última consulta. Renovar PID antes de agir. Nenhum Apply usado.
- Preview final: 1.308 ficheiros, ManifestSHA256 `45FAD4C19E6AE2E2BE35FA557C127A72C83B542E343159DDD13C8CD1CE447284`, Applied=false.
- NSIS existente `src-tauri/target/release/bundle/nsis/Lectus_0.7.1_x64-setup.exe` é ANTERIOR: SHA256 `9A8CA4CBCF17CDD6F95C024704DA6C8F5DD0803B894F0014F0E1ED7DDB5FCA5C`. Não contém esta ronda; nenhum novo instalador produzido.
- Setup/tradução/legal/histórico/API anteriores mantidos. Evidência anterior: `docs/reviews/2026-10-21-license-history-hardening.md` e snapshot `docs/handoffs/2026-10-21-license-history-hardening.md`.

## Recent Changes

1. Volume: snapshot conserva ID Core Audio; mute/restauro ficam no dispositivo original. A ausência dele não autoriza tocar no novo default. Apenas esta correção recebeu aprovação específica. Captura/inserção/atalhos de produção não mudaram.
2. Downloads: progresso persistido, pausa, Range/If-Range e validação de identidade/tamanho/checksum, espaço livre e mensagens de recuperação. Dez modelos conhecidos com digest; legacy Gemma/Qwen verificados antes de carregar/reusar. Duplicados recusados sem fila que anule a pausa. UI comum para ASR/tradução/cleanup; cancelamento voluntário não aparece como falha.
3. Cloud e recursos: diagnósticos seguros sem eco de chaves/body/URL; respostas limitadas; stale results e pedidos duplicados protegidos. Guia de CPU/RAM com estimativas upstream; não mede VRAM nem promete velocidade.
4. Recuperação: journal e backups verificados, promoção atómica por ficheiro, recovery manual por hash, lock interprocessos e recusa de aliases. CLI mantém lock durante stop/install/restart e rejeita journal pendente/corrompido antes de qualquer ação de processo. 13 fixtures em PS5 e PS7.
5. Benchmarks: Base/Turbo em 8 gravações humanas + derivações, 34 casos. MINDS pt-PT usa referências ASR/MT, não gold humano. Qwen 4B/0.6B em 8 textos cada: 0.6B omite/altera conteúdo e não foi integrado; 4B ainda acrescenta prefixos e usa português brasileiro. Qwen3-ASR não está integrado.

## Known Issues / Watch List

- Captura persistente só regista erro WASAPI; invalidar/reabrir o stream após desligação/retoma é proposta não implementada, sem aprovação específica para alterar captura. A correção do endpoint de volume não demonstra causa do sintoma Bluetooth.
- As duas recusas anteriores de foreground foram ultrapassadas numa nova execução no desktop Default: SendInput real passou em TextBox própria. Ditado humano também passou na instalação CD41... (config e binário inalterados, histórico limitado500 com timestamp novo). Isto não aceita o candidato94E... nem todos os destinos/clipboard. Não forçar foco, usar Notepad/sessões pessoais ou enviar texto às cegas.
- Windows Sandbox ausente; nenhuma VM demonstrada. Fixtures de cópia não validam NSIS, registo, autostart ou desinstalação. O conjunto de ficheiros não é atómico e a app não faz auto-recovery; recuperar manualmente antes de voltar a abrir após interrupção.
- Corpus pequeno: 2 gravações/configuração, ruído/concatenação sintéticos, sem avaliação equilibrada de sotaques ou revisão humana PT-PT/chinesa. Memória GPU não medida. Tradução genérica pt não garante variante europeia; prefixos indesejados continuam a precisar de melhoria verificada.
- Publicação continua bloqueada: identidade pública do editor, documentos draft e revisão das obrigações de terceiros/modelos. Não inventar empresa, publicar identidade privada, pedir morada/NIF, criar contas/serviços ou aceitar termos por inferência. O contacto público já está no manifesto; não resolve identidade. Nenhuma nova decisão legal é necessária para melhorias técnicas independentes.

## Quick Reference

- Provas: `docs/reviews/2026-10-21-reliability-and-models.md`, `2026-10-21-dictation-diagnosis.md`, `2026-10-21-download-recovery.md`, `2026-10-21-install-recovery.md`.
- Qualidade: `docs/benchmarks/2026-10-21-human-corpus.md`, `2026-10-21-quality-stress.md`, `2026-10-21-translation-comparison.md`; `docs/reviews/2026-10-21-model-quality-ux.md`.
- Código: `src-tauri/src/playback.rs`, `src-tauri/src/model_download.rs`, `src-tauri/src/transcription/model.rs`, `src-tauri/src/transcription/cloud.rs`, `src-tauri/src/resources.rs`, `src-tauri/src/model_quality_test.rs`.
- Ferramentas: `scripts/verify-native.cmd`, `scripts/fetch-quality-corpus.py`, `scripts/benchmark-model-quality.cmd`, `scripts/benchmark-translation-quality.cmd`, `scripts/install-local.tests.ps1`, `scripts/diagnostics/`.
- Impeccable não foi encontrada no catálogo/pesquisa instalada; não foi usada. Houve revisão independente de código/UX/acessibilidade e capturas. Caches e resultados brutos sob `.context/` estão ignorados; relatórios portáveis guardam resultados/hashes.

## Validation

- Full final: 244 pass / 0 fail / 10 ignored / 2 OS-mutation exclusions, 6,15 s após compilação. `npm run build`: 76 módulos; `verify-native.cmd --build` passou. Warnings permanecem.
- UI: 6 cenários downloads + 12 gerais, 20 capturas na fonte final `95e6d3727a331e60dc105bdcdd31843091c7fd01cc525fd1540c6c7079f18599`; 11 guided + 9 Gemma e 24 capturas no snapshot anterior, antes da última mensagem de pause em Models. 38 cenários/44 capturas, todos IPC simulado; não aceitação nativa.
- Captura física: 2,013 s, 8.000 pre-roll/24.160 live, sem WAV ou transcrição guardados, volume 3,94% inalterado. Leitura de endpoint real sem mutação; cenários de troca/erro em endpoints simulados.
- Retoma: VAD oficial pausado aos 8.192/885.098 bytes; transporte auxiliar terminado aos 139.264 bytes, novo processo retomou desse offset e validou 2.000.000 bytes. Não prova fechar a app real nem falha elétrica.
- `node scripts/check-legal.mjs --release` corretamente devolveu ready:false. Nenhum gate, configuração real, credencial, histórico ou instalação foi alterado pelos testes isolados.

## Next Actions

1. Coordenar prontidão humana antes de lançar o observador USB e solicitar a retirada/religação apenas após readiness. A tentativa anterior terminou sem ausência/regresso observados; não repetir relógios sem a pessoa pronta. Não tocar no SO nem alterar a captura por hipótese.
2. Depois da reprodução física, avaliar o stream original versus captura nova e ditado pós-reconexão. Só após prova propor e obter aprovação concreta para alterações à captura. Suspensão/retoma requer ambiente controlado; não suspender o host partilhado por inferência nem alterar Bluetooth globalmente.
3. Preparar VM/máquina descartável para NSIS clean/update/failure/recovery/uninstall e dados reais de teste; manter o publisher/termos/licenças como gate de publicação. Não usar o host principal como substituto da VM.
4. Melhorar tradução de variante pt-PT e prefixos em trabalho separado, com referências humanas e reavaliação real. Não integrar o Qwen 0.6B com as falhas observadas nem declarar outro modelo melhor sem evidência suficiente.

## Resume Checklist

- Confirmar branch/dirty tree, hashes dos artefactos e versão instalada; preservar alterações de todas as rondas.
- Ler relatório agregado e a prova específica do próximo teste; distinguir fonte, fixture, hardware, instalado e aceite.
- Reutilizar verificações válidas quando não houve mudança/questão nova. Não repetir fulls por rotina.
- Confirmar isolamento e foco antes de mic/input/processos; manter captura/inserção/atalhos sem novas alterações não aprovadas.

## Suggested skills

handoff-memory; systematic-debugging; verify/computer-use para aceitação controlada; verification-before-completion; handoff. Usar agentes com ownership explícito para revisão. Impeccable só se for encontrada/instalada de forma explícita; não fingir utilização.

## Generated artifacts this session

| Artefacto | Localização | Estado |
| --- | --- | --- |
| Executável de desenvolvimento | `src-tauri/target/release/chirp.exe` | Compilado; não instalado/distribuído |
| Diagnósticos reutilizáveis | `scripts/diagnostics/` | Inserção direta e ditado instalado passaram; USB não aceite sem desconexão; kit NSIS preparado/revisto |
| Corpus/modelo/resultados locais | `.context/quality-corpus/`, `.context/translation-quality/`, `.context/text-models/` | Ignorados; Qwen 0.6B e áudio públicos só para teste, sem contas/segredos |
| Relatórios e recibo | `docs/reviews/2026-10-21-reliability-and-models.md`, `docs/reviews/2026-10-21-reliability-receipt.json`, `docs/benchmarks/` | Provas portáveis; não aprovação de release |
| Handoffs datados | `docs/handoffs/2026-10-21-reliability-downloads-models.md`, `docs/handoffs/2026-10-21-native-acceptance-continuation.md` | Rondas anteriores e atual preservadas; sem commit/publicação |

Nenhuma credencial, entidade empresarial, certificado, conta, serviço ou aceitação real criada. A aceitação Windows→Mac anterior permanece separada e pendente.

## Resume Prompt

Continuar pela primeira Next Action viável num desktop interativo seguro; não transformar limitações de ambiente em aprovação para substituir a instalação ou bypass legal. O desenvolvimento técnico independente pode continuar sem nova pergunta sobre identidade.

## Next-session prompt

```text
Continua Lectus preservando toda a árvore suja. Lê docs/HANDOFF.md e docs/reviews/2026-10-21-reliability-and-models.md. A ronda passou 244 testes nativos e gerou um executável de desenvolvimento não instalado; o NSIS existente é antigo. Há captura/modelos/transporte reais; ditado humano end-to-end passou na instalação anterior, sem aceitar o candidato. Desligação/suspensão e NSIS em VM continuam a precisar de validação controlada.
Files: docs/reviews/2026-10-21-native-acceptance-continuation.md, scripts/diagnostics/capture/README.md, scripts/diagnostics/live-dictation/run.ps1, scripts/diagnostics/installer-acceptance/README.md, src-tauri/src/audio/mod.rs
Model: modelo de codificação atual, esforço alto — integração e diagnóstico nativo exigem cruzar provas.
Skills: handoff-memory, systematic-debugging, verify/computer-use, verification-before-completion, handoff.
Não repetir o ditado humano já aceite sem mudança/questão nova. Coordenar prontidão antes da próxima janela USB e reproduzir o problema antes de corrigir. Só o fix de endpoint de volume foi aprovado; pedir aprovação concreta para novas alterações à captura/inserção/atalhos. Não instalar, publicar, inventar identidade ou contornar gates legais.
```
