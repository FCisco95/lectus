# Lectus — instalação e recuperação isoladas, 21 outubro 2026

## Falha reproduzida e alteração técnica

O helper anterior conservava backups mas o rollback dependia de alcançar o bloco `catch` da mesma execução. Uma interrupção do processo depois da primeira substituição podia deixar uma mistura de ficheiros, sem estado persistente de commit/recuperação e sem operação explícita para recuperar.

`scripts/install-local.ps1` passa a escrever um manifesto de recuperação imutável e journal `Installing` antes de substituir ficheiros. Os originais são verificados e sincronizados; cada promoção é preparada num ficheiro temporário no mesmo volume, verificada por SHA256 e substituída atomicamente. O journal termina em `Committed` apenas após verificar todos os ficheiros. Um retry recusa uma transação `Installing` pendente para não tratar a instalação parcial como original válido.

`Invoke-LocalInstallRecovery` exige o SHA256 exato do manifesto de recuperação, valida contenção de caminhos, backups originais e todos os ficheiros atuais antes de escrever. Só aceita bytes antigos/novos conhecidos da transação. Bytes desconhecidos ou backup adulterado são preservados para revisão manual. Recuperar uma transação interrompida restaura originais e remove apenas novos ficheiros enumerados; repetir a recuperação é idempotente. Uma transação `Committed` não é revertida por esta operação. Backups são conservados; diretórios vazios de uma interrupção abrupta podem permanecer.

Revisão encontrou duas lacunas adicionais: concorrência entre processos e aliases de caminhos com segmentos internos. Corrigidas com lock `FileShare.None` conservado durante instalação/recuperação inteira, recusa de segmentos `.`/`..`, vazios, nomes Windows ambíguos/reservados e normalização das barras nas verificações de duplicados. O próprio lock e a árvore `backups` são destinos reservados. A recuperação interna após erro reutiliza o lock já adquirido.

A revisão final encontrou também uma fronteira de lock incompleta no CLI `-Apply -Restart`: um concorrente podia falhar ao adquirir o lock da cópia mas entrar no `catch` externo e relançar a aplicação enquanto a primeira execução substituía ficheiros. O CLI agora adquire o lock **antes de consultar/parar processos**, conserva-o até concluir relançamento ou tratamento do erro e chama o núcleo da transação sem adquirir um segundo lock. Falha de aquisição fica fora do bloco de ações e do seu `catch`; portanto não pode parar nem relançar processos. A recuperação CLI consulta processos igualmente sob lock.

Um journal `Installing` de uma execução anterior é recusado pelo gate comum `Assert-NoPendingLocalInstall`, ainda sob lock e **antes de entrar no bloco de consulta/paragem/erro/relançamento**. Journals inválidos ou corrompidos também recusam esse bloco. Isto impede que um apply novo pare a app e depois relance um conjunto de ficheiros parcialmente instalado ao descobrir a interrupção antiga. O núcleo repete a mesma validação para chamadas diretas. A recuperação continua **manual e explícita**, com preview/hash; não há auto-recovery nem relançamento automático do candidato misto.

As escritas JSON usam `WriteThrough`, `Flush(true)` e substituição atómica. Isto reduz janelas de interrupção; **não demonstra resistência a uma perda física de energia ou a falhas do disco/controlador**. Não foi feita nenhuma interrupção do computador.

## Provas e âmbito

**13 verificações isoladas passaram no Windows PowerShell 5 e no PowerShell 7**:

1. Manifesto inclui recursos/notices e backends correspondentes, excluindo DLLs antigas arbitrárias; atualização e deploy num diretório vazio verificam os hashes finais.
2. Erro de cópia recupera executable/recursos e remove ficheiros novos; colisões com diretórios, traversal e alterações posteriores do candidato são recusadas. Aliases internos, barras duplicadas, variantes `/`/`\` e colisões com o ficheiro de lock são recusados no install/recovery.
3. Processo filho terminou com `Environment.Exit(73)` depois de substituir o executable e antes de concluir, sem executar o `catch/finally` do instalador. Ficou journal `Installing`; novo apply foi recusado. Marcadores comprovam que journal pendente ou corrompido impede tanto stop como o catch/restart, preservando os ficheiros interrompidos para recuperação manual. Esta é uma interrupção real do processo do helper, com payload de fixture.
4. Hash de recuperação incorreto foi recusado sem alterações; recovery explícito restaurou executable/termos antigos, removeu ficheiros novos e passou numa segunda execução. Backup adulterado e edição independente posterior foram recusados.
5. Um segundo processo real tentou instalação, recuperação e o bloco lifecycle do CLI enquanto o primeiro detinha o lock; os três foram recusados, sem criar transação concorrente. Marcadores abstratos `stop/restart` comprovam que só a primeira execução entrou nas ações; o concorrente não entrou sequer no seu bloco de erro/relançamento. **Nenhum processo real da app foi parado/relançado por este teste.** Sentinelas sintéticas de configuração, histórico, credenciais e modelos num diretório separado de app data conservaram os hashes durante deploy, interrupção e recovery. Não foram usadas credenciais reais nem lidos conteúdos do histórico do utilizador.

```powershell
powershell -NoProfile -File scripts/install-local.tests.ps1
pwsh -NoProfile -File scripts/install-local.tests.ps1
```

O teste cria apenas uma árvore GUID em `.context/`, confirma a contenção antes de limpar essa árvore e não aponta ao destino real. O subprocesso interrompido executa apenas o helper de ficheiros, não o NSIS nem a aplicação.

## Utilização futura do recovery

Preview (somente leitura) usa `-RecoverBackup <diretório exato do backup>`. Uma execução com `-Apply` exige `-ExpectedRecoveryManifestSHA256 <hash do preview>`. O CLI recusa recovery enquanto o executable exato da instalação estiver a executar; não termina processos automaticamente. Não executar estas operações na instalação do utilizador como parte da verificação.

## Limitações e estado da distribuição

Estas provas dizem respeito à transação de ficheiros do helper local. **Não validam instalação limpa/atualização/reparação/desinstalação do NSIS**, entradas do registo, integração Windows, permissões nativas ou limpeza real de credenciais. O host não dispõe de Windows Sandbox nem de um ambiente VM comprovado; não foi instalado um hypervisor ou executado o instalador no host como substituto. Uma validação completa continua a precisar de VM ou máquina descartável aprovada.

A atomicidade é **por ficheiro**, não do conjunto completo. Uma interrupção pode deixar versões misturadas até à recuperação manual. O arranque da aplicação/autostart não consulta este journal nem faz recovery automático; depois de uma interrupção é preciso recuperar antes de voltar a abrir a aplicação. O teste não demonstra um conjunto instalável atomicamente nem autoriza entregar este helper como substituto dessa aceitação.

O instalador antigo existente é um artefacto anterior, não representa este candidato. Nenhum novo instalador foi gerado por este trabalho. Publicação permanece bloqueada por identidade pública do editor, documentos legais draft e revisão de terceiros pendentes. Nenhum gate foi contornado, identidade inventada, conta criada ou serviço contratado. A instalação atual não foi parada nem substituída.
