# Lectus — preparação da aceitação NSIS isolada

21 outubro 2026. **Preflight e recusa segura verificados; instalação NSIS ainda não executada.** Nenhuma instalação/configuração/conta/serviço do utilizador foi alterado. Fonte e artefactos anteriores preservados.

## Capacidades renovadas

Inventário local em modo só leitura, cerca de 20:12–20:19 UTC:

| Consulta | Observação | Consequência |
| --- | --- | --- |
| `Win32_ComputerSystem` | Fabricante ASUS; modelo `System Product Name`; `HypervisorPresent=true` | O host físico usa virtualização; isto não demonstra um Windows descartável. O runner recusa-o. |
| `Win32_OptionalFeature` | `VirtualMachinePlatform` e `Microsoft-Windows-Subsystem-Linux`: `InstallState=1`; `Containers-DisposableClientVM`, Hyper-V e respetivos tools/services: `InstallState=2` | WSL está ativado; Sandbox/Hyper-V estão desativados. Não afirmar que os recursos são inexistentes ou que basta obter outro token. |
| Ficheiros de servicing | Pacotes `.mum`/`.cat` de DisposableClientVM presentes, incluindo idiomas | Presença de metadados de instalação não demonstra que Sandbox está ativado ou que o payload de execução está disponível. Não habilitado nem reparado por inferência. |
| `root/virtualization/v2:Msvm_ComputerSystem` | Namespace inexistente | Nenhuma VM Hyper-V enumerável neste estado. Não equivale a prova sobre todas as máquinas/hipervisores externos. |
| Serviços e WSL | `vmcompute` Running; `vmms` não enumerado; Ubuntu Stopped, docker-desktop Running, ambos WSL2 | Linux WSL/Docker não valida NSIS, registo, WebView2 e desinstalação Windows. Não foi iniciado Ubuntu. |

O inventário prévio desta continuação não encontrou WindowsSandbox.exe, cmdlets Hyper-V, VirtualBox, VMware ou QEMU nos caminhos consultados, nem ISO em Downloads. Não é inventário exaustivo de todos os discos. `Get-WindowsOptionalFeature -Online` foi tentado por funcionalidade, mas requer elevação Windows/UAC; a elevação do comando na ferramenta não elevou o token Windows. A consulta WMI acima forneceu os estados sem alterar o sistema. Nenhum pedido UAC, feature enable, download de ISO, reinício ou instalação de hipervisor ocorreu.

## Kit concreto preparado

Ficheiros em `scripts/diagnostics/installer-acceptance/`:

- `run.ps1`: Preflight por defeito; clean install de baseline, seed de dados sintéticos, upgrade, tentativa observada de interrupção do processo NSIS, repair por reinstalação, uninstall com retenção e verificação posterior de remoção interativa explícita.
- `guards.ps1`: definições sem efeitos laterais; exige SHA256 não vazio/formato válido, ficheiro existente e bytes coincidentes antes de considerar uma baseline/desinstalador verificados.
- `guard-checks.ps1`: regressões isoladas dos predicados e normalização JSON PS5/PS7, sem escrever fixtures ou chamar fases mutantes.
- `artifacts.json`: whitelist de hashes propositalmente incompleta. NSIS antigo identificado, mapping do executável embebido ainda desconhecido; candidato atual tem só executável dev, não NSIS. Nenhum ramo de instalação pode executar com este manifesto.
- `README.md`: preparação do guest, snapshots e sequência em ramos separados; recolha dos recibos e limites de cada resultado.

A mutação requer modelo de hardware VM reconhecido **e** attestation local coincidente com MachineGuid/ComputerName/snapshot descartável **e** SHA256 explícito do manifesto revisto. Rejeita paths UNC/reparse e aliases de traversal no inventário; verifica hash do instalador antes de o iniciar. Verifica todos os ficheiros runtime/recursos enumerados no manifesto e a inscrição de desinstalação; nenhum dado atual é copiado do host. As sentinelas simulam configuração, histórico, cloud/API tokens desativados e modelos nos dois destinos Windows Roaming/Local. O kit não promete armazenamento no Windows Credential Manager: o código atual inspecionado guarda cloud/API credenciais em JSON.

Não lança Lectus após NSIS (`/R` omitido), aceita acordos reais, cria contas, ativa cloud ou altera gates legais. O ramo de interrupção só termina o PID do instalador que criou quando observa a substituição do exe; se não observar a janela devolve falha/inconclusivo. Não é power loss. Não há claim de recuperação automática do NSIS; `Repair` verifica a reinstalação de todos os ficheiros enumerados e dados sintéticos preservados. Remoção de dados usa a checkbox real interativa do NSIS, seguida de verificação: não é simulada por `Remove-Item`.

## Verificação realizada e limites

| Verificação | Resultado |
| --- | --- |
| `powershell -NoProfile -File scripts/diagnostics/installer-acceptance/run.ps1` | PS5 passou: `PreflightOnly`, VM false, installerExecuted false, whitelist incompleta para baseline/candidate. |
| `pwsh -NoProfile -File scripts/diagnostics/installer-acceptance/run.ps1` | PS7 passou, mesmo resultado. |
| Mesmos dois hosts com `-Phase CleanInstall` | Ambos recusaram no guard de modelo de VM, antes de ler attestation, criar diretórios/recibos ou iniciar processos. Erro esperado, não falha de instalação. |
| `Test-Path C:\LectusAcceptance` após recusas | False; nenhuma árvore guest ou receipt criada no host. |
| SHA256 instalado após checks | `CD41E53A3A337B7770C99D9EA2E779546B679C39AD5EBA5C441E0DCCA117C406`, inalterado. |

No primeiro ensaio PS5, o valor por defeito de `ManifestPath` tentou avaliar `PSScriptRoot` demasiado cedo no bloco de parâmetros. Corrigido para resolver os defaults no corpo do script; os dois preflights finais passaram. Não houve execução NSIS nesse ensaio.

Revisão independente encontrou e fez corrigir cinco guardas antes de qualquer uso em VM: comparação `null`/`null` que podia aceitar baseline ausente em SeedData; desinstalador sem hash obrigatório; destino de receipts sem recusa de junction; recibo de sentinelas vazio/duplicado; e Upgrade sem exigir baseline instalada. Todas as fases mutantes passam agora pelas duas whitelists completas antes de qualquer escrita/receipt. SeedData/Upgrade/InterruptUpgrade verificam baseline/payload/uninstaller e inscrição; o NSIS uninstaller é comparado com hash explícito antes de iniciar. Diretório de receipts e state são verificados contra reparse points. O conjunto de sentinelas exige exatamente os oito caminhos esperados, sem duplicados, com hashes válidos.

Após as correções, preflight PS5 e PS7 voltou a passar às 20:23 UTC. `guard-checks.ps1` passou em ambos os hosts: recusou expected SHA null/vazio/inválido e ficheiro ausente, aceitou um ficheiro local com digest exato e recusou whitelist incompleta com o erro explícito esperado. O guard real de sentinelas, carregado isoladamente por AST com I/O substituído em memória, recusou `[]` e duplicados e aceitou as oito entradas esperadas. A leitura JSON passou a atribuir o resultado a uma variável antes de o normalizar em array: PS5 emitia o array JSON completo como um único item da pipeline. O check confirma oito objetos com `path` string em PS5/PS7. Isto verifica predicados de segurança, não uma instalação. Nenhum sentinel real foi criado.

Manifesto final de artefactos SHA256: `4A7F2879120AC3A0D14279A04AE7939CD76125E2963DD49A53789BD143E479D5`. O runner **não foi testado nos ramos mutantes** em guest Windows; os preflights e guards não validam silent install, interrupção/repair ou uninstall. Nenhum novo teste da suite Rust/frontend era necessário: os fontes da app não mudaram nesta frente.

## Artefactos e condição para executar

Revalidados nesta continuação:

- Dev `src-tauri/target/release/chirp.exe`: SHA256 `94E3BBAC07AEF58EE7A72FA516C2D6EBF40A3BCBD9AE54F8961E42DBCF210B6A`; não instalado.
- NSIS antigo `src-tauri/target/release/bundle/nsis/Lectus_0.7.1_x64-setup.exe`: SHA256 `9A8CA4CBCF17CDD6F95C024704DA6C8F5DD0803B894F0014F0E1ED7DDB5FCA5C`; não representa o candidato atual.

Para executar faltam **um Windows descartável operacional com snapshot** e **baseline/candidate NSIS elegíveis com mapping completo e revisto de hashes**. A publicação/packaging atual continua corretamente bloqueada por editor/termos/licenças; nenhum manifesto foi marcado aprovado e não foi invocado o bundler contornando `beforeBundleCommand`. Preparar este kit e diagnosticar as capacidades não dependia dessas decisões legais. Não foi substituída a instalação do utilizador para compensar a ausência do guest.
