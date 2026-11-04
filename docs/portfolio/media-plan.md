# Lectus — plano de media

Estado: **pending**. Preparado em 04/11/2026. Não houve gravação, captura nativa, inferência, troca de app ou upload. Aplicado como documentação numa cópia isolada de master. Sem commit ou publicação.

## Base e identidade

Referência pública: distribuição v0.7.1 de `FCisco95/lectus-releases`; source de referência `FCisco95/lectus@f4627f25ab7a477acceea83fcaca1f79786a7fb3`. Igualdade entre source e binário instalado não foi atestada nesta vaga. Antes de gravar, registar versão no About, caminho/hash do executável realmente aberto, plataforma, data, backend/modelo e permissões. Se a app aberta não corresponder à referência, parar a captura de release e resolver a versão com Cisco. Não fechar, instalar ou substituir a app por inferência.

Reutilizar o papagaio Eclectus multicolorido de `src/assets/logo.png` e os ícones `src-tauri/icons/lectus-master.png`, `icon.png` e `tray-*.png`. O logo foi copiado da tag para `docs/portfolio/media/lectus-logo.png`, sem redesenho. O master de base não continha `src/assets/logo.png`; os assets de produto não foram alterados. Não usar o banner MYCEL/Cisco do perfil.

A tag define a paleta do papagaio: verde `#1db584`, ciano `#17a2a2`, azul `#3b82f6`, magenta `#e91e8c`, coral `#ff6b5a`, dourado `#ffa500`. UI neutra com azul de interação; gradiente reservado à marca. Tipografia de sistema (`Segoe UI`/SF Pro). Fonte: `src/styles/tokens.css` na tag. Não alterar UI ou criar um sistema visual novo.

## Demo principal — 30 segundos

| Tempo | Ação real a captar | Legenda em inglês |
|---|---|---|
| 0–5 s | Editor neutro com campo vazio e pill em repouso; cursor visível | “Keep your cursor where you write.” |
| 5–16 s | Acionar o atalho configurado; dizer a frase sintética; mostrar pill em gravação e transcrição | “Hold. Speak. Release.” |
| 16–25 s | Mostrar o texto realmente inserido no mesmo campo, sem substituir o resultado na edição | “Your words, in the same field.” |
| 25–30 s | Cartela simples com marca existente, v0.7.1/plataforma/backend registados, e acesso | “Requires $20 of ORGANIC or MYCEL. Downloads: github.com/FCisco95/lectus-releases” |

Frase alvo: “The project review is on Thursday.” Usar Whisper local, modelo já disponível e cleanup desligado. Isto evita inferência paga e torna a origem do texto clara. A frase alvo não é uma promessa de saída exata: guardar e apresentar o resultado real; repetir a toma se a legibilidade exigir, sem fabricar correções. Gravar apenas quando Cisco estiver pronto para usar a app comprovada.

## Três capturas planeadas

Os paths abaixo são destinos futuros, **não ficheiros existentes nem referências para o README atual**.

| Path planeado | Conteúdo | Alt text / legenda em inglês |
|---|---|---|
| `docs/portfolio/media/01-input.png` | Campo vazio num editor neutro, pill e cursor; sem outros documentos | “A blank note with the Lectus status pill beside the focused text field.” |
| `docs/portfolio/media/02-dictating.png` | A mesma composição durante gravação nativa real | “Lectus recording a synthetic sentence while the note stays focused.” |
| `docs/portfolio/media/03-result.png` | A mesma composição com o resultado efetivamente inserido | “The dictated sentence inserted into the same note.” |

Captura principal futura: a de resultado, desde que legível a 800–1000 px de largura. As três podem formar sequência estática com legendas; sem slider JavaScript. Settings é opcional e só entra depois de comprovar a versão: Home/histórico exclusivamente sintético, Vocabulary com `Lectus`/`PostgreSQL`, ou seleção de modelos efetivamente disponíveis. Nunca mostrar cards locais de subscrição/tradução/API como release.

## Critérios de captura e publicação posterior

1. Confirmar versão e backend. Não ativar cloud/cleanup remoto, chamar APIs, comprar tokens, ligar wallets ou mudar configurações globais para a gravação. Se o gate impedir uso autorizado, manter media pendente.
2. Preparar editor apenas com dados inventados. Esconder notificações e janelas privadas sem apagar histórico pessoal. Não filmar wallet, Membership, login, chaves, códigos de pairing, username pessoal ou paths privados; não confiar em blur como proteção única.
3. Captar PNGs e vídeo reais com a mesma frase, escala e enquadramento. Cortes/crop podem melhorar enquadramento, sem compor estados de UI ou alterar o resultado. Se usar um diagrama, rotular “Architecture diagram”, separado das capturas.
4. Guardar recibo de captura com versão/hash, OS, backend/modelo, data, frase, saída observada, dimensões e hashes dos ficheiros. Avaliar PNGs em claro/escuro e a largura estreita; confirmar que o texto e a pill continuam legíveis. Não apresentar esse check como aceitação completa do produto.
5. Só depois de autorização para produção/upload escolher alojamento público existente. Thumbnail deve ser um frame real, com destino acessível e rendering verificado. Guardar imagens versionadas com paths relativos e case correto; não ligar vídeo ou imagens antes de existirem. Não adicionar “coming soon” ao README.

## Inventário e lacunas

| Material | Estado | Uso permitido nesta proposta |
|---|---|---|
| Logo e ícones na tag | Existentes | Logo incluído; ícones inventariados, sem redesenho |
| Tokens de design na tag | Existentes | Referência de identidade; sem alteração de produto |
| `docs/mockups/*.html` | Mockups de desenho | Não usar como screenshot de produto |
| Reviews/capturas locais de 21–22/10 | Desenvolvimento, parte com IPC mockado; nota de evidência no handoff | Excluídas da apresentação da release |
| Três PNGs, demo 30 s e thumbnail | **Pending** | Nenhuma referência pública adicionada |

Também pendentes: prova da versão realmente instalada, recolha consentida com app nativa, revisão de dados visíveis, e alojamento/rendering do vídeo. A existência dos instaladores não preenche estas lacunas.
