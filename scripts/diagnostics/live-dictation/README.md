# Aceitação humana de ditado

Abre um campo WinForms próprio para a pessoa ditar uma frase através do Lectus **já instalado e em execução**. Não lança nem substitui a app, não muda configuração, não simula teclas/voz e não captura áudio no helper.

1. Coordenar previamente com a pessoa: aparecerá a janela do teste.
2. Executar com Windows PowerShell 5.1 em STA:

   ```powershell
   powershell.exe -NoProfile -STA -File .\scripts\diagnostics\live-dictation\run.ps1
   ```

3. A pessoa clica no campo, segura **Ctrl direito**, diz **“A reunião é na quinta-feira às dez horas”**, solta a tecla e aguarda o campo fechar.
4. Consultar apenas o recibo novo em `receipts/`. `outcome=passed` exige frase reconhecível no campo, foco/PID/controlo próprios e timestamp novo de histórico, com configuração/processo/hash preservados.

Prazo padrão e máximo: 180 s. Fechar a janela cancela. Se perder o foco, o helper espera; se chegar texto sem foco confirmado, aborta. Configuração diferente do perfil local revisto ou alterações durante o teste abortam sem configurar a app.

O recibo guarda texto bruto apenas se corresponder exatamente à frase canónica. Fala alternativa fica como comprimento, SHA256 e presença das palavras esperadas. Conteúdo anterior do histórico não é registado; só contagem/timestamp máximo são usados.

## Resultado real preservado

**PASS em 21-10-2026, 20:24:30.4037640–20:24:46.7651583 UTC.** A pessoa ditou através da instalação 0.7.1 para o campo próprio. Texto recebido: `A reunião é na quinta-feira às 10 horas.` seguido de CRLF. Sem input sintético ou alteração da app/configuração.

Prova portátil e limites: [`docs/reviews/2026-10-21-live-dictation-acceptance.md`](../../../docs/reviews/2026-10-21-live-dictation-acceptance.md) e [recibo redigido](../../../docs/reviews/2026-10-21-live-dictation-receipt.json).

Esta prova aceita somente o fluxo da **instalação anterior**; não testa o candidato de desenvolvimento nem a correção de volume nele existente. O intervalo de 16,36 s inclui reação humana e não representa latência do modelo. Não cobre desligação/troca do microfone, suspensão, cloud ou Windows→Mac.
