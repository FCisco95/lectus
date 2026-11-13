# SNAPSHOT METADATA

- Created At: 2026-10-21T22:06:20+01:00
- Scope: repo
- Kind: handoff
- Source Canonical: C:\Users\joao_\code\lectus\docs\HANDOFF.md
- Workspace: n/a
- Workstream: n/a
- Repositories: lectus
- Reason: Pedido explícito de handoff-memory; estado nativo consolidado e teste USB pendente

---
# Lectus — handoff

## Metadata

- Project: Lectus
- Last Updated: 2026-10-21
- Updated By: Codex
- Branch: feat/remote-dictation
- Status: local, não committed/publicado; preservar toda a árvore suja.

## TL;DR

**Ditado humano e inserção direta passaram na instalação atual. O candidato novo não foi instalado nem aceite.** O teste USB terminou sem ausência/regresso observados; reconexão não validada. A próxima sessão deve coordenar prontidão humana antes de iniciar outro observador.

A ronda de desenvolvimento passou 244 testes nativos, 0 falhas, 10 ignorados e 2 exclusões de mutação OS. Frontend e executável compilaram; as 26 fontes e o binário foram depois conferidos inalterados, sem repetir a suite por rotina.

## Current Objective

Validar desconexão/reconexão física de microfone e recuperação, respeitando o pedido de aprovação antes de alterar captura, inserção ou atalhos. Suspensão/retoma e NSIS requerem ambiente controlado. Não substituir a instalação atual nem contornar gates legais.

## Current State

- Instalação observada: `chirp.exe` na pasta Lectus de LOCALAPPDATA; versão0.7.1, SHA256 `CD41E53A3A337B7770C99D9EA2E779546B679C39AD5EBA5C441E0DCCA117C406`. PID25604 à última consulta; renovar antes de agir.
- Candidato: `src-tauri/target/release/chirp.exe`, versão0.7.1, 77.940.736bytes, SHA256 `94E3BBAC07AEF58EE7A72FA516C2D6EBF40A3BCBD9AE54F8961E42DBCF210B6A`; não instalado/distribuído. Preview1.308ficheiros, Applied=false; manifesto `45FAD4C19E6AE2E2BE35FA557C127A72C83B542E343159DDD13C8CD1CE447284`.
- NSIS existente `src-tauri/target/release/bundle/nsis/Lectus_0.7.1_x64-setup.exe` é anterior, SHA256 `9A8CA4CBCF17CDD6F95C024704DA6C8F5DD0803B894F0014F0E1ED7DDB5FCA5C`. Não contém estas melhorias; nenhum novo instalador gerado.
- Configuração observada: reconhecimento local, cleanup/tradução/perfis desativados; hold em Ctrl direito, toggleF13. Chave cloud não configurada. Nenhuma configuração foi alterada pelos helpers.

## Recent Changes

1. Volume: correção especificamente autorizada conserva ID do endpoint original; nunca restaura no novo default. Não demonstra causa da queda Bluetooth intermitente.
2. Downloads: pausa/retoma persistente, Range/If-Range, validação de identidade/tamanho/SHA256, espaço livre e recuperação. Duplicados recusados sem fila; modelos incompletos não são promovidos. Gemma/Qwen legados verificados antes de carregar/reusar.
3. UX/cloud: erros úteis sem eco de chaves/body/URL, limites de resposta e proteção contra resultados antigos; guia CPU/RAM e controlos de progresso/pausa nos três tipos de modelo.
4. Instalação: journal, backup/recuperação por hash, substituição atómica por ficheiro e lock que inclui stop/install/restart. Journal pendente/corrompido bloqueia ações de processo. Kit NSIS separado tem cinco guards corrigidos, revistos e verificados em PS5/PS7.
5. Modelos: Base/Turbo em8gravações humanas e derivações,34casos; referências MINDS pt-PT são ASR/MT, não gold humano. Qwen4B/0.6B em8textos cada:0.6B omitiu/alterou conteúdo e não foi integrado;4B ainda acrescentou prefixos e português brasileiro. Qwen3-ASR não integrado.

## Known Issues / Watch List

- `PersistentCapture` só regista erro WASAPI: invalidar/reabrir stream é proposta não implementada nem especificamente aprovada. Não corrigir por hipótese. Captura/inserção/atalhos de produção não mudaram nesta continuação.
- Reconexão USB não aceite:90s sem ausência/regresso; nenhuma comparação original/fresh ou recuperação demonstrada. Manter cabo ligado até nova janela coordenada. Ditado aprovado é da instalação anterior, não do candidato.
- Sandbox/Hyper-V estão desativados por WMI; não há Windows guest operacional demonstrado. WSL ativo não substitui essa aceitação. NSIS clean/update/failure/recovery/uninstall não executado; kit e guards não provam o ciclo real. Recuperação de ficheiros é manual; conjunto não é atómico.
- Cloud real, suspensão/retoma, falha real de transcrição, clipboard, destinos elevados e Windows→Mac continuam pendentes. Corpus pequeno, ruído/concatenação sintéticos, VRAM e qualidade humana PT-PT/chinesa não avaliadas de forma abrangente.
- Publicação bloqueada por identidade pública do editor, documentos draft e revisão de terceiros/modelos. Não inventar empresa, publicar identidade privada, pedir morada/NIF ou chaves em chat, criar contas/serviços ou aceitar termos por inferência. Melhorias técnicas independentes podem continuar.

## Quick Reference

- Estado/provas recentes: `docs/reviews/2026-10-21-native-acceptance-continuation.md`, `docs/reviews/2026-10-21-native-acceptance-receipt.json`, `docs/reviews/2026-10-21-live-dictation-receipt.json`, `docs/reviews/2026-10-21-microphone-reconnect-receipt.json`.
- Desenvolvimento/modelos: `docs/reviews/2026-10-21-reliability-and-models.md`, `docs/reviews/2026-10-21-reliability-receipt.json`, `docs/benchmarks/2026-10-21-human-corpus.md`, `docs/benchmarks/2026-10-21-translation-comparison.md`.
- Diagnósticos: `scripts/diagnostics/capture/README.md`, `scripts/diagnostics/live-dictation/run.ps1`, `scripts/diagnostics/insertion/run.ps1`, `scripts/diagnostics/installer-acceptance/README.md`.
- Código: `src-tauri/src/audio/mod.rs`, `src-tauri/src/playback.rs`, `src-tauri/src/model_download.rs`, `src-tauri/src/transcription/model.rs`, `scripts/install-local.ps1`.

## Validation

- Suite anterior:244pass/0fail/10ignored/2OS-mutation exclusions; build frontend76módulos e release passaram. Warnings permanecem. Receipts indicam fontes exatas; não confundir testes de código com hardware.
- UI:38cenários/44capturas com IPC simulado em snapshots documentados. Não são aceitação nativa. Impeccable não foi encontrada; não foi usada.
- Inserção direta real passou PT/EN/chinês em TextBox própria, sem clipboard. Ditado humano passou: frase canónica reconhecida e timestamp novo no histórico500→500 pelo limite; configuração e binário inalterados. O tempo16,36s inclui reação/fala, não latência do modelo.
- VAD oficial retomou8.192/885.098bytes; transporte auxiliar terminou abruptamente e outro processo retomou139.264bytes até2.000.000verificados. Não prova fechar a app Tauri ou falha elétrica.
- Recovery:13fixtures PS5 e PS7. Kit NSIS:preflight/guards revistos apenas, sem execução do instalador. USB:NOT_ACCEPTED, sem ausência/regresso. Nenhum novo acordo jurídico, credencial ou conta criado.

## Next Actions

1. Esperar prontidão explícita da pessoa junto ao cabo USB. Só depois iniciar o observador e pedir retirada/religação após `ready_for_human_unplug_replug`; expirar a instrução quando terminar. Não repetir o ditado já aceite por rotina.
2. Se houver reprodução, comparar stream original com captura nova e validar ditado pós-reconexão. Propor correção concreta e obter aprovação antes de modificar a captura. Não reiniciar/substituir a app por inferência.
3. Usar Windows descartável com snapshot e instaladores baseline/candidato elegíveis e hashes completos para executar o kit NSIS. Não ativar funcionalidades/UAC/reiniciar o host nem contornar packaging legal por inferência.
4. Tratar variante pt-PT/prefixos de tradução em trabalho separado, com referências e verificação real; não integrar o0.6B com as falhas observadas.

## Resume Checklist

- Confirmar branch/dirty tree e hashes dos artefactos; preservar todo o trabalho local.
- Ler a prova da experiência seguinte e distinguir instalado/candidato, fixture/hardware e preparado/aceite.
- Usar Python3.13 (`py -3.13` ou `python3`) para os scripts handoff-memory: `python` por omissão é3.8 e falha em `str.removesuffix`. Não foi necessário alterar Python global.

## Suggested skills

handoff-memory; systematic-debugging; verify/computer-use para aceitação controlada; verification-before-completion; handoff. Agentes de revisão com ownership exclusivo. Não exigir Impeccable sem confirmar disponibilidade.

## Generated artifacts this session

Executável dev em `src-tauri/target/release/chirp.exe`, diagnósticos reutilizáveis em `scripts/diagnostics/`, relatórios/recibos em `docs/reviews/` e benchmarks em `docs/benchmarks/`. Caches públicos e recibos brutos sob `.context/`/receipts estão ignorados; provas portáveis estão nos relatórios. Snapshots anteriores em `docs/handoffs/` preservados. Não foi criada entidade empresarial, credencial, certificado, serviço ou aceitação jurídica real. O histórico recebeu a entrada normal do teste humano.

## Resume Prompt

Retomar pela prontidão humana para USB; não abrir outro relógio de teste sem a pessoa pronta. Não repetir verificações válidas sem mudança/questão nova. Preservar instalação atual e gates de publicação.

## Next-session prompt

```text
Continua Lectus a partir de docs/HANDOFF.md. Ditado humano e inserção real passaram na instalação anterior; candidato94E... não instalado. USB terminou sem ausência/regresso observados; não foi aceite. NSIS requer Windows descartável e packages elegíveis; cloud não tem chave configurada.
Files: docs/reviews/2026-10-21-native-acceptance-continuation.md, scripts/diagnostics/capture/README.md, scripts/diagnostics/live-dictation/run.ps1, scripts/diagnostics/installer-acceptance/README.md, src-tauri/src/audio/mod.rs.
Model: modelo de codificação atual, esforço alto — integração e diagnóstico nativo.
Skills: handoff-memory, systematic-debugging, verify/computer-use, verification-before-completion, handoff.
Coordenar prontidão antes do teste USB. Só o fix de volume foi aprovado; pedir aprovação concreta para novas alterações à captura/inserção/atalhos. Não instalar/publicar, inventar identidade ou contornar gates legais. Preservar todas as alterações locais.
```
