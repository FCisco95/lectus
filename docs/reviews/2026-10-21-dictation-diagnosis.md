# Lectus — diagnóstico do ditado e volume, 21 outubro 2026

## Estado e limites

A instalação existente permanece intacta e a executar (PID observado 25604; renovar antes de agir). SHA256 confirmado: `CD41E53A3A337B7770C99D9EA2E779546B679C39AD5EBA5C441E0DCCA117C406`. Não foi iniciada outra instância, substituído executável, aceite termos, alterado o registo Windows, desligado microfone ou suspenso o computador. Nenhuma alteração à captura, inserção ou atalhos foi feita neste diagnóstico; o repositório exige aprovação para essas alterações. Após o diagnóstico, o utilizador autorizou especificamente a correção de binding do endpoint de volume descrita abaixo.

## Provas obtidas

1. Observação **real e somente de leitura** através da implementação Windows `PlatformOutput.get()`: 45 leituras em 10.106 ms, volume `3,94%`, mute `false`, sem mudanças observadas. O volume baixo atual é um facto; não prova que Lectus o tenha reduzido, nem reproduz a queda intermitente. `DisableAbsoluteVolume` continua `0`; não foi alterado.
2. Reprodução **isolada com endpoints simulados**, usando a máquina de estados de produção em `src-tauri/src/playback.rs`: após silenciar A a 70%, mudar o default para B previamente silenciado a 20% e restaurar, A fica silenciado e B passa a 70% e perde o mute do utilizador. Um teste passou por confirmar exatamente este comportamento incorreto, **não por o corrigir**. O adaptador Windows resolve novamente o endpoint default em cada leitura/escrita, portanto não conserva a identidade do endpoint da snapshot. Este defeito é distinto da queda Bluetooth intermitente e não estabelece a causa dela.
3. Configuração ativa consultada só nos campos necessários: hold, RControl, F13 para toggle, entrada SteelSeries Alias, processamento local, mute durante ditado e setup concluído. Chaves e conteúdo do histórico não foram lidos nem publicados.

Comandos reproduzíveis (sem instalar):

```powershell
cargo test --offline --manifest-path scripts/diagnostics/playback-endpoint/Cargo.toml diagnosis:: -- --nocapture
cargo run --offline --manifest-path scripts/diagnostics/playback-endpoint/Cargo.toml -- 10
```

O programa de observação só lê Core Audio. O teste específico só escreve em objetos fake. Não executar indiscriminadamente todos os testes do módulo incluído: o teste existente `windows_mute_roundtrip_restores` altera mute/volume reais.

## Riscos encontrados no código; ainda sem aceitação física

- `audio/mod.rs`, callback de erro em `build_16k_mono_stream`: apenas escreve em stderr; `PersistentCapture` não recebe estado de falha/heartbeat. Depois de um dispositivo invalidado, o objeto pode continuar `Some`, pelo que a captura seguinte prefere o stream antigo em vez de abrir o fallback. Não há tratamento de suspensão/retoma ou notificações de mudança do default encontrado. Isto é um risco confirmado por inspeção, ainda **não reproduzido com dispositivo físico**.
- `lib.rs`, captura em `run_pipeline`: conserva o mutex de PreRoll durante toda a gravação. Alteração de microfone fica à espera da gravação; não demonstra troca transparente a meio do ditado.
- Falhas de pipeline restauram estado/volume por `PipelineError`; as regressões existentes com mocks não provam que o microfone ou o destino real recuperem após suspensão.

## Correção de volume autorizada e implementada

`src-tauri/src/playback.rs` agora associa a snapshot ao ID estável do endpoint Core Audio. Mute, fallback de volume e restauro usam esse ID; um endpoint desaparecido produz erro e mantém a snapshot para nova tentativa. O novo default nunca é usado como substituto do endpoint original. Captura, inserção e atalhos permanecem inalterados.

Prova RED→GREEN: a asserção de comportamento correto falhou antes da correção (A continuava silenciado); passou depois. O teste de diagnóstico agora exige A restaurado e B intacto. A suite isolada, que inclui o módulo de produção sem o transformar, passou **18 testes**, excluindo explicitamente o teste existente que altera mute/volume reais. Inclui mudança de default entre snapshot/mute, ajuste manual independente em B, desaparecimento/regresso de A, falha de restauro seguida de retry e leitura real de uma snapshot associada ao endpoint Windows. Estes cenários de falha continuam simulados; a leitura do endpoint é real e não o modifica.

```powershell
cargo test --offline --manifest-path scripts/diagnostics/playback-endpoint/Cargo.toml -- --skip windows_mute_roundtrip_restores
```

A correção é local, não instalada. Não prova que o sintoma intermitente de Bluetooth tenha esta causa.

## Proposta de captura, ainda não aplicada

Para o stream, propagar erro/última entrega de áudio ao dono da captura, invalidar o objeto e reabrir o dispositivo selecionado/default antes do próximo ditado. Emitir erro acionável quando não existe entrada. Esta proposta altera captura; exige aprovação explícita do utilizador. Não implementar retries cegos nem declarar que Bluetooth é a causa.

## Aceitação ainda necessária

### Captura real isolada acrescentada

Após aviso prévio, `scripts/diagnostics/capture` abriu em modo partilhado **somente o microfone explicitamente configurado e enumerado por nome**, usando diretamente `src-tauri/src/audio/mod.rs` da produção. O harness recusa nome ausente/vazio e confirma novamente o dispositivo após abrir. Não modificou a pipeline, atalhos, configuração, permissões ou recibos. Não aceita prompts automaticamente.

Resultado real: stream aberto **2.013 ms**, pre-roll **8.000 amostras**, captura live **24.160 amostras / 1.510 ms**, RMS **0,00266248**, pico **0,02686695**, **0** amostras não finitas. As amostras foram consumidas/descartadas em memória; nenhum WAV ou transcrição foi guardado. Core Audio leu volume **3,94%**, mute `false`, antes e depois, sem alteração observada. A instalação existente permaneceu ativa.

```powershell
cargo run --offline --manifest-path scripts/diagnostics/capture/Cargo.toml
```

Isto prova que a captura partilhada, pre-roll, begin/drain/end e resampling atuais produziram amostras reais deste microfone nesta sessão. **Não prova reconhecimento da fala, precisão, inserção numa aplicação ou recuperação de desconexão/suspensão.** Não foi sintetizada voz nem tentada transcrição.

### Inserção real: tentativa abortada com segurança

Foi preparado e compilado `scripts/diagnostics/insertion`, que inclui diretamente `src-tauri/src/injection/sendinput.rs` de produção. O helper WinForms cria uma TextBox e processo exclusivos, sem Notepad nem documentos pessoais. Antes de lançar o injector exige HWND foreground da própria janela; o injector confirma PID, relação janela/campo, foco do campo, ausência de Ctrl/Shift/Alt/Windows/botões do rato premidos e 150 ms sem novo input. O texto de teste é fixo e inclui acento português, inglês e caracteres chineses. O próprio helper verifica `.Text`, sem clipboard, config ou histórico.

As duas tentativas reais de abrir o alvo abortaram **antes de lançar o injector**: a janela própria não conseguiu obter foreground, mesmo após uma ativação nativa normal e espera limitada. Nenhuma tecla/input sintético foi enviado. Só as janelas próprias foram fechadas; não foi usado `AttachThreadInput`, tecla Alt sintética ou bypass das restrições de foreground do Windows. **Não existe prova de inserção real nesta sessão.** A origem específica da recusa de foreground não foi estabelecida; não se atribui a falha ao algoritmo SendInput.

Preparação, para um futuro desktop interativo seguro:

```powershell
cargo build --offline --manifest-path scripts/diagnostics/insertion/Cargo.toml
powershell -NoProfile -STA -File scripts/diagnostics/insertion/run.ps1
```

O helper devolve foco apenas se ainda pertence à própria janela. Se o Windows não o conceder, aborta. Não executar o binário injector diretamente com HWND/PID de aplicações do utilizador.

Ditado em campos de aplicações reais, troca/desligação física do microfone, suspensão/retoma e falha real de transcrição continuam por validar nesta versão. Nenhum resultado simulado é aceitação nativa. A nova versão tem gates legais pendentes, pelo que iniciar o candidato com bypass não seria um teste válido. A instalação atual é usada pelo utilizador; não foi acionado o hotkey nem alterado o seu foco/documentos às cegas.

Próxima experiência para a queda intermitente: observação read-only durante uma ocorrência real e, num momento seguro explicitamente acordado, comparação com Lectus totalmente terminado. Esse controlo separa falha independente da aplicação; ainda não existe essa prova. Não aplicar workaround global de Bluetooth por inferência.
