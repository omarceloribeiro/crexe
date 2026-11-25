# Computação regenerável: uma agenda de pesquisa a partir do CREXE

> Whitepaper de visão e proposta · 24/11/2026. Explora possibilidades de longo prazo que não estão entregues pela v1. Não é uma especificação normativa, um padrão aprovado ou uma promessa de produto.

## Resumo

Um software pode ser preservado como binário, fontes ou descrição do comportamento desejado. A computação regenerável investiga a relação entre essas representações: manter a intenção compreensível, produzir implementações para ambientes definidos e conservar evidências para avaliar mudanças.

O CREXE oferece um ponto de partida limitado: receitas legíveis, geração de fontes, compilação local, revisões e exportação. A hipótese de longo prazo é ampliar esse percurso para ferramentas pessoais e alguns utilitários do ambiente de trabalho. Para isso, seria necessário resolver problemas de confiança, manutenção, equivalência funcional e custo que a existência de um gerador, isoladamente, não resolve.

## Uma definição operacional

Neste documento, regenerar significa produzir uma nova implementação a partir de uma intenção e de contexto controlado, submetendo o resultado a critérios de aceitação. Recompilar significa construir novamente fontes existentes. Reutilizar cache significa abrir um artefato já preservado. São operações diferentes, com custos e riscos diferentes.

Um ambiente regenerável precisaria escolher explicitamente entre elas. Reabrir uma calculadora não justificaria inferência contínua. Uma nova necessidade poderia motivar a edição da receita; uma atualização de dependência poderia exigir apenas recompilação; uma incompatibilidade comprovada poderia justificar regeneração. Essa escolha deveria ser observável e reversível, sem substituir uma versão funcional por uma hipótese não validada.

## Por que conservar a intenção

Os fontes descrevem uma implementação com grande precisão, mas nem sempre tornam evidentes as razões de suas decisões. Uma receita bem escrita pode registrar objetivos, exemplos, restrições e critérios de sucesso em uma forma que usuários e desenvolvedores consigam discutir juntos.

Imagine uma ferramenta didática de proporções. A professora quer que valores inválidos sejam explicados, que números decimais sigam a convenção local e que a fórmula apareça antes do resultado. Essas regras podem acompanhar o projeto como intenção revisável. Posteriormente, uma turma solicita um modo de porcentagem. O desafio passa a ser demonstrar que a nova versão acrescenta esse modo sem alterar a regra anterior.

A qualidade da receita importa. Linguagem natural admite ambiguidades e pressupostos omitidos. Exemplos concretos, contraexemplos e testes ajudam a tornar o pedido verificável. Isso amplia a participação de pessoas sem formação em programação, mas não elimina a necessidade de julgamento, aprendizado ou apoio técnico.

## Arquitetura proposta para um ambiente regenerável

Uma arquitetura futura poderia separar o registro da intenção, a produção de candidatos e a autorização de uso. O diagrama descreve uma proposta de pesquisa, não módulos já disponíveis no CREXE.

```text
Intenção versionada + exemplos + critérios independentes
                          |
               contexto e requisitos do destino
                          |
             escolha explícita da operação
               /          |           \
          reutilizar   recompilar   regenerar
               \          |           /
                 candidato identificado
                          |
           verificações funcionais e de segurança
                          |
             decisão de aceitar ou rejeitar
                          |
       implantação controlada + observação + reversão
                          |
             histórico e evidências preservados
```

Cada transição precisaria registrar quem a iniciou, quais entradas foram usadas, que verificações ocorreram e o que mudou. Um modelo poderia ajudar a formular ou corrigir a implementação, mas não deveria ser a única autoridade que declara o próprio trabalho aceitável.

## Camadas e limites de aplicação

O campo inicial reúne tarefas delimitadas, com falhas observáveis e recuperáveis. Quanto maior o alcance da ferramenta, maior o esforço para demonstrar que uma regeneração preserva suas responsabilidades.

| Camada | Possível investigação | Limite necessário |
|---|---|---|
| Ferramentas pessoais | Calculadoras, visualizadores e pequenos organizadores | Casos de aceitação claros e dados de teste |
| Utilitários de sessão | Interfaces para tarefas do usuário | Permissões mínimas, recuperação e migração de estado |
| Componentes compartilhados | Bibliotecas e serviços usados por vários programas | Contratos de compatibilidade e avaliação dos consumidores |
| Kernel, drivers e cadeia de inicialização | Fora do escopo desta proposta | Exigiriam evidências e processos de garantia muito mais fortes |

A exclusão de componentes críticos é uma delimitação de pesquisa, não uma afirmação de impossibilidade técnica. Sua falha pode comprometer disponibilidade, integridade ou segurança antes que mecanismos comuns de recuperação estejam disponíveis. A proposta não recomenda experimentação automática nessas camadas nem mudanças em sistemas de segurança crítica.

Mesmo um utilitário aparentemente simples pode ser arriscado. Renomear arquivos em massa afeta dados reais; uma ferramenta de cálculo usada em decisões financeiras pode ter consequências relevantes. O escopo deve considerar o uso e as permissões, não apenas o tamanho do código.

## Binários continuam fazendo parte do sistema

Um artefato previamente compilado pode oferecer inicialização previsível, distribuição eficiente e uma versão que muitas pessoas conseguem testar em comum. Esses benefícios permanecem relevantes. Uma arquitetura regenerável pode combinar receitas, fontes aprovados e binários identificados, permitindo escolher o artefato adequado à situação.

Preservar um binário conhecido também pode apoiar recuperação quando o modelo, o compilador ou um serviço remoto ficam indisponíveis. A receita, sozinha, não reconstrói todo o ambiente que a tornou executável. Compatibilidade futura depende de ferramentas, dependências, conhecimento das APIs e critérios que permitam julgar uma nova implementação.

## Continuidade exige mais que um histórico de prompts

Uma ferramenta pode acumular arquivos, preferências e hábitos de uso. Regenerá-la preservando apenas a descrição da interface deixa de fora esses compromissos. Um sistema maduro precisaria tratar formatos de dados, migrações, acessibilidade, comportamento de erros e integração com outros programas como partes explícitas do contrato.

O histórico deveria conservar candidatos rejeitados e causas relevantes sem expor dados privados desnecessariamente. Comparações entre versões precisariam mostrar mudanças de comportamento e de permissões. Retornar ao executável anterior não basta se a nova versão já transformou os dados de maneira incompatível; reversão do aplicativo e recuperação dos dados são problemas relacionados, porém distintos.

## Evidência, confiança e variabilidade

Cache não equivale a reprodutibilidade. Ele conserva um resultado; não garante que outra geração produza os mesmos fontes ou que outro ambiente produza o mesmo binário. Uma pesquisa séria deve separar equivalência comportamental, reprodução dos fontes e reprodução dos bytes compilados.

Assinaturas e hashes poderiam identificar origem e integridade de artefatos, mas não demonstrariam sua correção. Também seria necessário avaliar dependências, comportamento em execução e limites de autoridade. Testes escritos pelo mesmo gerador merecem revisão independente para reduzir o risco de que o teste confirme o erro da implementação.

Na v1 atual, o [modelo de segurança](../../SECURITY.md) é execução no host com as permissões do usuário. O isolamento necessário para ampliar o uso com receitas desconhecidas permanece um problema aberto. Qualquer proposta futura de restrição de rede, arquivos ou processos precisaria ser imposta por mecanismos reais e testada contra tentativas de contorno.

## Economia do ciclo completo

Uma receita curta não torna automaticamente a distribuição mais barata. A avaliação deve incluir obtenção de modelos, processamento de inferência, SDKs, dependências, armazenamento de revisões, tempo humano e operação posterior. Esses custos podem ser compartilhados entre aplicativos ou repetidos em muitas máquinas.

O resultado depende do cenário. Uma ferramenta muito personalizada pode justificar uma primeira geração demorada. Distribuir o mesmo utilitário estável a milhares de máquinas pode favorecer um binário comum. Não há ganho universal de desempenho, banda ou energia decorrente de compilar localmente. É necessário comparar alternativas sob as mesmas tarefas e condições de aceitação.

## Programa de investigação

A evolução pode começar com experimentos pequenos e ampliar o alcance apenas quando a evidência justificar o próximo passo.

| Pergunta | Experimento proposto | Critério de decisão |
|---|---|---|
| A intenção sobrevive à mudança? | Acrescentar requisitos a ferramentas conhecidas | Testes independentes detectam regressões anteriores |
| O resultado é utilizável? | Observar tarefas reais em diferentes hosts | Usuários concluem a tarefa com requisitos documentados |
| A recuperação funciona? | Interromper etapas e simular candidatos defeituosos | Versão válida e dados permanecem recuperáveis |
| O custo é justificável? | Comparar geração, compilação e distribuição convencional | Medição completa, incluindo preparo e trabalho humano |
| O mecanismo respeita limites? | Avaliar permissões e tentativas de acesso indevido | Restrições comprovadas no ambiente declarado |

Resultados negativos também precisam ser publicados: pedidos ambíguos, migrações malsucedidas e casos em que a intervenção humana supera o benefício esperado. A comparação deve separar testes com respostas controladas de ensaios com modelos reais, e primeira geração de reabertura pelo cache.

## Uma trajetória possível

No curto prazo, o trabalho útil é tornar ferramentas delimitadas mais compreensíveis, editáveis e verificáveis. Em um horizonte mais longo, comunidades poderiam manter intenções, exemplos e testes junto de implementações aprovadas. Parte de um ambiente de trabalho poderia então evoluir por esses contratos, preservando alternativas convencionais quando fossem mais adequadas.

Essa trajetória depende de engenharia, avaliação e governança. O [whitepaper de fundamentos](whitepaper-fundamentos.md) descreve o ponto de partida implementado. Esta visão formula perguntas verificáveis sobre onde a regeneração oferece autonomia útil e onde seus custos e riscos recomendam outro caminho.
