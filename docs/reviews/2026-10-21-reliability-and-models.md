# Lectus — fiabilidade, downloads e comparação de modelos

Atualizado em 21 de outubro de 2026. Trabalho local em `feat/remote-dictation`, sem commit, publicação ou substituição da instalação. Preservadas as alterações anteriores de API, tradução, setup, termos e histórico. Este relatório distingue testes de código, HTTP real, inferência real, hardware e aceitação completa.

## Melhorias e provas

| Frente | Implementação / resultado | Prova e limite |
| --- | --- | --- |
| Ditado e volume | Correção especificamente autorizada: guardar o ID de saída original e restaurar só esse endpoint. Dispositivo ausente conserva a snapshot; não toca no novo default. | RED→GREEN com máquina de estados real e endpoints simulados; 18 testes do diagnóstico. Captura real de 2,013 s no microfone configurado, 8.000 amostras de pre-roll e 24.160 live, sem guardar áudio; volume antes/depois 3,94%. Não reproduz a queda Bluetooth intermitente. |
| Downloads | Pausa/retoma persistida, Range/If-Range, validação de ETag/Last-Modified/Content-Range, tamanho e SHA256 oficial obrigatório; verificação antes de promover `.part`. Espaço livre, falhas recuperáveis, progresso e recuperação após reabrir painel. Pedidos duplicados não ficam em fila para contrariar Pause. | HTTP loopback, regressões de cancelamento/interrupção/remoto alterado/checksum/disco; VAD oficial real pausado aos 8.192/885.098 bytes e retomado com digest válido. Processo auxiliar realmente terminado após 139.264 bytes; processo novo retomou desse offset e validou 2.000.000 bytes. Isto não fecha/reabre a app Tauri. |
| Modelos | Comparados Base e Turbo em 8 gravações humanas públicas e respetivas derivações: 34 casos de modelo, com CER/WER, latência e memória. Comparados Qwen 4B e 0.6B em 8 textos cada, separados do ASR. | FLEURS: referências de leitura externas; MINDS pt-PT: transcrições automáticas, não gold humano. Só 2 clipes/configuração. Qwen 0.6B alterou/omitiu conteúdo e fica apenas no benchmark. Qwen 4B também acrescentou prefixos e produziu português brasileiro. Nenhum modelo declarado melhor em geral; Qwen3-ASR continua não integrado. |
| Configuração e UX | Reutilizado `/models` para teste cloud; erros de chave/permissão/endpoint/quota/timeout/redirecionamento sem corpos remotos/URLs privados. Respostas limitadas e resultados antigos descartados. Guia de RAM/CPU e estimativas upstream, sem prometer GPU ou velocidade. UI comum de pausa/progresso nos 3 tipos de modelo. | 10 testes cloud locais; UI Chromium com IPC simulado. Pausa voluntária foi reproduzida como alerta indevido e corrigida consultando o estado nativo. Impeccable não estava instalado; revisão independente de código/acessibilidade e inspeção de capturas, sem alegar execução dessa skill ou UI nativa. |
| Instalação e recuperação | Journal persistente, cópias originais verificadas, substituição atómica por ficheiro, recuperação explícita por hash e proteção contra concorrência/aliases de caminhos. Lock cobre stop/install/restart; journal pendente/corrompido bloqueia essas ações. | 13 fixtures em PS5 e PS7 e processos filhos isolados; sem NSIS ou aplicação instalada. Configuração/histórico/credenciais/modelos representados por sentinelas sintéticas preservadas. O conjunto completo não é atómico; pode exigir recovery manual antes de voltar a abrir. |

Relatórios completos: [ditado](2026-10-21-dictation-diagnosis.md), [downloads](2026-10-21-download-recovery.md), [instalação](2026-10-21-install-recovery.md), [cloud/UX](2026-10-21-model-quality-ux.md), [corpus humano](../benchmarks/2026-10-21-human-corpus.md) e [tradução](../benchmarks/2026-10-21-translation-comparison.md).

## Verificação integrada

`scripts/verify-native.cmd` passou **244 testes, 0 falhas, 10 ignorados e 2 exclusões de mutação OS** (`test_set_clipboard_text` e `windows_mute_roundtrip_restores`). Os ignorados incluem benchmarks/rede que exigem execução explícita; os ensaios de modelo e VAD descritos acima foram executados separadamente. A ronda anterior desta sessão tinha 242 testes; as regressões posteriores de concorrência e leitura real de recursos também passaram. A última ronda inclui o novo harness de tradução apenas em `cfg(test)`.

`npm run build` passou, com 76 módulos. `scripts/verify-native.cmd --build` passou após o frontend final. Permanecem avisos do compilador; não é uma declaração de zero warnings. Não foi produzido novo NSIS.

Recibos de UI em `docs/reviews/*/results.json`:

| Modo | Cenários de comportamento | Capturas / âmbito |
| --- | ---: | --- |
| `2026-10-21-download-ui-smoke` | 6 | 4 capturas, light/dark e 720/1100 px, sem overflow; fonte final `95e6d3727a331e60dc105bdcdd31843091c7fd01cc525fd1540c6c7079f18599`. |
| `2026-10-21-ui-smoke` | 12 | 16 capturas e controlos de teclado; mesma fonte final. |
| `2026-10-21-guided-smoke` | 11 | 16 capturas; snapshot anterior `aa9630f061ffd52ba8da90c468b4beb8a016c2136cf6309a124fe98eee35c107`, antes da última correção de mensagem na ação Pause de Models. Setup/tradução não mudaram depois dessa prova. |
| `2026-10-21-model-license-smoke` | 9 | 8 capturas; mesmo snapshot anterior. O gate Gemma não mudou depois dessa prova. |

São **38 cenários e 44 capturas simuladas**, não 38 fluxos nativos. Os testes não fazem chamadas a fornecedores reais nem aceitam termos reais. A pausa voluntária usa uma promise pendente no fixture para reproduzir o resultado de cancelamento do comando original, em vez de apenas mudar a barra de progresso. Os detalhes de fonte/timestamp estão em cada JSON.

Revisões independentes encontraram e fizeram corrigir ocultação de erros em parciais, bypass de checksum na seleção/carregamento legado, concorrência no helper de instalação, aliases de caminhos e o pedido de download em espera que anulava a pausa. A aprovação do utilizador abrangeu só a correção do volume; captura, inserção e atalhos de produção não foram alterados.

## Artefactos e instalação

Executável de desenvolvimento: `src-tauri/target/release/chirp.exe`, versão ainda **0.7.1**, SHA256 **`94E3BBAC07AEF58EE7A72FA516C2D6EBF40A3BCBD9AE54F8961E42DBCF210B6A`**. Não instalado, não aceite como distribuição e não executado como substituto da app atual. A compilação final embebe o frontend verificado.

A instalação observada continua a executar o ficheiro em `%LOCALAPPDATA%/Lectus/chirp.exe`, SHA256 **`CD41E53A3A337B7770C99D9EA2E779546B679C39AD5EBA5C441E0DCCA117C406`**, PID observado 25604. Renovar PID antes de qualquer operação futura. Nenhum `Apply` foi usado nela.

Preview final somente de leitura: 1.308 ficheiros, ManifestSHA256 `45FAD4C19E6AE2E2BE35FA557C127A72C83B542E343159DDD13C8CD1CE447284`, Applied=false. Recibo de hashes de fontes/artefactos e resultados: `docs/reviews/2026-10-21-reliability-receipt.json`.

O ficheiro existente `src-tauri/target/release/bundle/nsis/Lectus_0.7.1_x64-setup.exe` é o **instalador anterior**, de 86.262.239 bytes, SHA256 **`9A8CA4CBCF17CDD6F95C024704DA6C8F5DD0803B894F0014F0E1ED7DDB5FCA5C`**. Não contém estas melhorias. Não confundir a mesma versão 0.7.1 com o mesmo conteúdo.

Caches de modelos/corpus/resultados e builds auxiliares ficam sob `.context/`, com ignores específicos; resumos, proveniência e hashes portáveis ficam nos relatórios. O Qwen 0.6B é cache de benchmark, não recurso do instalador nem nova escolha da app. Não foram criadas credenciais, contas, certificados ou serviços. Os scripts de diagnóstico são distintos do executável da aplicação.

## Limitações por concluir

1. Ditado completo em aplicações reais, clipboard/SendInput, troca/desligação física do microfone, suspensão/retoma e recuperação de falha real de transcrição. O alvo WinForms de inserção não obteve foreground em duas tentativas e o guard abortou antes de enviar qualquer input; isto não demonstra defeito do SendInput. Há captura e inferência reais separadas, não aceitação end-to-end.
2. Causa da queda intermitente de volume Bluetooth não demonstrada. O defeito distinto de endpoint A→B está corrigido localmente; não aplicar alterações globais do Windows por inferência. Propagar falha WASAPI/reabrir captura foi apenas proposto, não implementado nem autorizado.
3. Fechar/reabrir a app real durante um download, falha física de disco/disco realmente cheio, benchmarks equilibrados por sotaque e revisão humana de PT-PT/chinês. Derivações de ruído/concatenação não acrescentam falantes; memória GPU não foi medida. Tradução genérica `pt` não garante a variante europeia.
4. Instalação limpa/atualização/reparação/desinstalação NSIS em VM, incluindo registo/autostart/dados reais. Windows Sandbox está ausente e não foi demonstrada VM disponível. Journal e recuperação de ficheiros não provam atomicidade do conjunto ou recuperação automática da app.
5. Publicação continua bloqueada. `node scripts/check-legal.mjs --release` devolveu `ready:false` pelas aprovações/editor/documentos draft/revisão de terceiros pendentes. Sem identidade empresarial inventada, dados pessoais publicados ou gates contornados; não foi pedida nova decisão legal para avançar nas melhorias técnicas.

## Repetição focada

Usar `scripts/verify-native.cmd`, `scripts/benchmark-model-quality.cmd`, `scripts/benchmark-translation-quality.cmd` e `scripts/fetch-quality-corpus.py --reuse` conforme o âmbito; os benchmarks exigem os caches verificados indicados nos relatórios. Não repetir fulls válidos sem mudança de código ou questão nova. Para instalação, usar apenas fixtures `scripts/install-local.tests.ps1`; não executar Apply na instalação do utilizador como rotina de verificação.
