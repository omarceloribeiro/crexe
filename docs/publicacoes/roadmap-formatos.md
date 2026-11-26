# Roadmap — CREXP e CREXB

**Edição de 26/11/2026.** As prévias mínimas estão integradas à `main` e disponíveis na pré-release pública `v1.0.0-preview.1`; não significam que toda a arquitetura dos handoffs esteja entregue ou que já exista uma release estável. Este roadmap não fixa datas para os próximos estágios.

CREXE permite descrever. CREXP permite organizar especificações. CREXB permite distribuir e abrir um resultado aceito. Os três papéis se complementam: a receita preserva intenção, o pacote organiza essa intenção e o bundle preserva também uma implementação.

## O que cabe na primeira entrega

| Área | Prévia mínima | Limite explícito |
| --- | --- | --- |
| CREXP | ZIP com vários Markdown, manifesto mínimo opcional, inspeção e consolidação determinística | Sem assets, agentes ou resolução semântica de conflitos |
| Geração de CREXP | Reutiliza o pipeline, SDKs e tentativas normais | O SDD é enviado ao provider configurado; não é o contrato de privacidade do gerador de leitores |
| CREXB | Cria, inspeciona, verifica, prepara cache e abre snapshot aceito | Um target por bundle, subconjunto de perfis e runtime previamente disponível |
| Receita no CREXB | Original exato registrado na geração, extração explícita para regenerar | Cache antigo sem snapshot requer regeneração explícita antes de exportar |
| Evidência | Build e autotestes reais do pipeline | Sem afirmar aceitação humana ou validação visual do aplicativo |
| Integridade | ZIP validado, caminhos controlados e hashes antes do launch | Hash não autentica publisher; assinatura exigida é recusada |
| Armazenamento | Payload em cache e dados mutáveis separados | Não oferece sandbox do aplicativo |
| Publicação | Guias PT/EN e originais completos para download | Handoffs originais são em português; os guias ingleses não são tradução integral |

Os guias mantidos de [CREXP](crexp.md) e [CREXB](crexb.md) descrevem comandos, limites e condições. O snapshot aceito é um estado técnico do pipeline, não um selo de confiança.

## CREXP completo

| Etapa | Trabalho previsto | Critério para considerar entregue |
| --- | --- | --- |
| Documentos e manifesto | Papéis ricos, inventário, assets, código semente, contratos, políticas e capacidades versionadas | Arquivos suportados explícitos; capacidades desconhecidas não são silenciosamente ignoradas |
| Recipe Compiler | Grafo de documentos, precedência, conflitos e `recipe.resolved.yaml` | Decisões e conflitos rastreáveis; ambiguidades relevantes bloqueiam ou exigem resolução explícita |
| Reprodutibilidade | Hashes normalizados, plano de geração, lock file, fingerprints de resolver e toolchain | Distinguir interpretação, geração, build e aceitação; não presumir identidade bit a bit |
| Geração agêntica | Agentes locais/remotos, ferramentas, ciclos de build/test/correção e orçamento | Limites de tempo, custo, rede e permissões verificados; nenhuma instrução do pacote eleva privilégios |
| Resolvers e targets | Adaptação para ambientes e plataformas adicionais | Matriz de suporte testada com exemplos reais, sem anunciar sistemas apenas imaginados |
| Publicação | Publish para pasta e, futuramente, destinos cloud com configuração externa | Operações explícitas, credenciais fora do pacote, artefatos e relatórios verificáveis |
| Observabilidade e confiança | Relatórios, traces locais, políticas, assinatura e interoperabilidade | Clareza sobre o que sai da máquina, origem verificável e testes de compatibilidade |

## CREXB completo

| Etapa | Trabalho previsto | Critério para considerar entregue |
| --- | --- | --- |
| Experiência de exportação | Interface de exportação, opções de fontes/testes/logs e políticas de aceite | Exportar a geração selecionada com evidência real; tentativas falhas não se passam por builds aceitos |
| Receita e regeneração | CREXP incorporado, inventário, plano, inspeção guiada e geração para outro host | Receita preservada; regeneração explícita com custos e requisitos visíveis |
| Runtimes e compatibilidade | Payload autossuficiente quando viável, dependências verificadas e reparo guiado | Respeitar redistribuição de dependências; nunca executar target incompatível silenciosamente |
| Confiança na distribuição | Assinatura, publisher, SBOM, notices, scanning e proveniência | Integridade e autenticidade distinguidas; políticas de assinatura realmente verificadas |
| Privacidade na exportação | Scanner de segredos, seleção de evidências e redação controlada | Não divulgar silenciosamente dados privados nem perder a ligação entre receita e build |
| Múltiplos targets | Vários payloads e seleção do mais adequado | Compatibilidade demonstrada em cada host; extração e validação limitadas ao conteúdo autorizado |
| Versões e atualizações | Índices, canais, rollback, migrações, coleta de cache e possíveis deltas | Limpar cache não apaga dados do usuário; atualizações não promovem conteúdo incompleto |
| Concorrência e escala | Locks, transações, políticas de retenção e gerenciamento de bundles em execução | Aberturas simultâneas e falhas intermediárias não corrompem cache ou estado |

## O que continua sendo pesquisa

O whitepaper [Compatibilidade temporal generativa](whitepaper-compatibilidade-temporal-generativa.md) acrescenta uma visão de evolução, sem implementação ou validação experimental de drivers e pontes temporais no CREXE. Seu [original completo em inglês](originais/CREXE_Generative_Temporal_Compatibility_White_Paper.md) está disponível para download. `.crdll` e `.crdrv` são nomes propostos, não extensões reconhecidas como novos formatos pela engine atual.

| Direção de pesquisa | Questão a investigar | Evidência necessária antes de anunciar suporte |
| --- | --- | --- |
| `.crdll` — bibliotecas generativas | Preservar comportamento, APIs e ABIs ao gerar componentes para outros ambientes | Contratos versionados e testes independentes de comportamento e integração em cada target |
| `.crdrv` — compatibilidade de hardware | Gerar componentes em espaço de usuário ou, quando justificado, privilegiados, a partir de protocolos documentados | Simulação, testes de protocolo, limites de privilégio, recuperação e validação controlada no hardware |
| Compatibilidade temporal bidirecional | Hardware antigo com SO futuro; hardware moderno com SO histórico | Demonstração reproduzível de cada combinação, incluindo requisitos físicos e limitações |
| Pontes entre épocas | Gerar as partes de software de uma solução distribuída, como um componente legado e um proxy moderno | Adaptador físico disponível, interfaces documentadas e testes ponta a ponta; código não cria hardware ausente |

Os casos Lisa/impressora moderna, Windows 3.1/IPP, controle Bluetooth/retrocomputador e DOS/NVMe são cenários de pesquisa. O relato do autor sobre geração de software para Lisa não valida esses cenários. Não há prazo de entrega; essas propostas complementam, sem substituir, os papéis de CREXE, CREXP e CREXB.

Integração profunda com sistemas operacionais, plataformas históricas, montagem virtual, execução parcial/streaming e reprodução bit a bit em toolchains diferentes continuam hipóteses de pesquisa. Uma Gallery, serviços comerciais e publicação cloud podem ser planejados sem serem produtos disponíveis nesta versão. Este site não inclui uma landing comercial.

Não há promessa de que um programa gerado seja seguro apenas por estar em um pacote, de que hashes provem autoria ou de que toda máquina seja compatível. O programa é executado no host com as permissões do usuário. A escolha de modelos, SDKs e dependências continua relevante.

## Documentos completos, sem cortes

- [CREXP — handoff completo original, em português](../formatos/originais/CREXP-HANDOFF-COMPLETO.md).
- [CREXB — handoff completo original, em português](../formatos/originais/CREXB-HANDOFF-COMPLETO.md).
- [CREXB — JSON Schema original do manifesto v1](../formatos/originais/crexb-manifest-v1.schema.json).
- [CREXB — manifesto ilustrativo original](../formatos/originais/crexb-manifest.example.json).
- [SHA-256 dos originais](../formatos/originais/checksums.sha256).

Os quatro arquivos foram preservados byte a byte. Schema e manifesto ilustram a proposta; o exemplo usa placeholders e **não é um aplicativo executável**. O manifesto v1 proposto aceita possibilidades além do subconjunto da prévia. Os handoffs mantêm discussões anteriores de licenciamento; a [política vigente](../LICENCIAMENTO_E_ECOSSISTEMA.md) é MIT para a engine e MIT-0 para os materiais de saída designados. Os guias em inglês têm o mesmo escopo mantido, com os originais portugueses identificados como tais; use o seletor de idioma no site.
