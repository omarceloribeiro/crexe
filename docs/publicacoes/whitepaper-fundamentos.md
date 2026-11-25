# CREXE: fundamentos de um executável criativo

> Whitepaper informativo · 24/11/2026. Descreve os fundamentos e o estado da v1 em desenvolvimento. Não constitui uma especificação normativa nem um padrão aprovado pela IETF ou por outra entidade de padronização.

## Resumo

CREXE significa **Creative Executable**. Um arquivo `.crexe` expressa a intenção de um aplicativo em texto legível. Uma engine interpreta o documento, combina o pedido com a configuração local, solicita fontes a um modelo e coordena sua compilação, verificação e execução. A receita pode circular independentemente do aplicativo produzido; o projeto gerado também pode ser exportado.

A proposta amplia as opções de distribuição e personalização de pequenas ferramentas. Sua utilidade depende de condições concretas: um modelo compatível, ferramentas de desenvolvimento disponíveis, testes adequados e disposição para executar o resultado. A implementação atual demonstra esse percurso, mas permanece em desenvolvimento e não oferece isolamento de segurança. Este documento distingue o mecanismo presente das hipóteses que ainda exigem pesquisa.

## A intenção como artefato compartilhável

Uma pessoa pode precisar de uma calculadora com regras específicas, uma interface para uma tarefa recorrente ou um exercício didático. Distribuir a descrição permite que outra pessoa compreenda o objetivo e o adapte antes da geração. Um pedido simples pode ser o conteúdo completo de `calculadora.crexe`:

```text
Gerar uma calculadora simples com as quatro operações,
botão de limpar, tratamento de divisão por zero e fundo verde.
```

Essa descrição não determina todos os detalhes da implementação. Duas gerações podem resolver o mesmo pedido de maneiras diferentes, inclusive com comportamentos não solicitados. A receita é uma intenção transportável; a equivalência entre resultados precisa ser demonstrada. Quando o objetivo é compartilhar exatamente os fontes de uma versão aprovada, o projeto exportado é o artefato apropriado.

## Os documentos aceitos pela v1

A extensão reúne três formas documentadas de expressar o pedido. Elas atendem a diferentes necessidades de estrutura sem exigir que todo usuário escreva uma especificação extensa.

| Forma | Conteúdo principal | Papel na experiência |
|---|---|---|
| Texto puro | O documento inteiro expressa a intenção | Começar com uma descrição; o nome vem do arquivo |
| Markdown | Corpo livre e cabeçalho reconhecido com `crexe: 1` | Organizar o pedido e, opcionalmente, indicar nome e perfil |
| YAML legado | Inputs, seletores, intenção e targets estruturados | Preservar receitas com comandos e condições explícitos |

O [contrato de formatos](../FORMATOS_V1.md) é a referência para os campos efetivamente reconhecidos. Estruturas identificadas e inválidas não são reinterpretadas silenciosamente como texto livre. As RFCs históricas registram a evolução da proposta; seus exemplos não comprovam funcionalidades disponíveis. Este whitepaper não acrescenta formatos de entrada ou campos ao contrato.

## Arquitetura e percurso de execução

A engine é implementada em Rust. Seu papel é coordenar etapas verificáveis e manter o estado necessário para retomadas e alterações. O modelo propõe fontes; a engine valida a resposta e executa as operações definidas pelo perfil ou pela receita legada, sujeitas à política local.

```text
Arquivo .crexe + configuração local
                 |
        leitura e normalização
                 |
    ambiente, perfil e revisão anterior
                 |
       cache válido? ---- sim ----> abrir artefato aprovado
                 |
                não
                 |
    modelo local ou serviço remoto
                 |
      validação dos fontes recebidos
                 |
       compilação + testes definidos
                 |
      falha? ---- sim ----> reparo dentro dos limites
                 |
          revisão aprovada
                 |
       cache + projeto exportável
                 |
          execução opcional
```

O perfil informa ferramentas, arquivos de projeto e operações. Há perfis para Windows Forms, console e interfaces nativas, além de opções OpenGL. Os perfis novos permanecem experimentais conforme a [matriz de pré-requisitos](../PERFIS_V1.md). Detectar o sistema e encontrar um compilador não garante que qualquer pedido possa ser atendido. A engine não emula outro sistema nem instala SDKs globais automaticamente.

## Escolhas locais e geração remota

O usuário seleciona provider, modelo, endpoint e credenciais na configuração local. Ollama oferece o percurso com modelo local; OpenAI e DeepSeek são opções remotas. Uma falha local não autoriza migração automática para um serviço pago. Na geração remota, o pedido e o contexto de fontes necessário ao trabalho são enviados ao provider escolhido.

Os adaptadores presentes e seus limites estão descritos na [configuração](../CONFIGURACAO.md). O registro histórico de experiências com outros transportes não significa suporte atual a eles. A compatibilidade de um modelo também não decorre apenas de aparecer em uma listagem: ele precisa responder ao contrato aceito pela engine.

Credenciais podem ser armazenadas no cofre do sistema. A configuração define limites de tentativas, saída e tempo. Esses controles restringem a operação, mas não equivalem a um teto monetário preciso: o orçamento de saída não contabiliza integralmente todos os custos possíveis de uma API.

## Continuidade ao editar a receita

Editar o texto e salvar não inicia geração. Ao abrir novamente o documento, a engine pode usar a última revisão aprovada como contexto: pedido anterior, fontes textuais e novo pedido. Isso permite solicitar um ajuste sem começar deliberadamente do zero.

A continuidade é uma orientação e uma estrutura de histórico, não uma garantia de preservação semântica. O modelo pode justificar uma recriação ou mudança de perfil quando a seleção não estiver fixada. Uma alteração visual também pode introduzir regressões funcionais. Por isso, testes devem acompanhar as regras que precisam permanecer.

Falhas e cancelamentos não avançam o índice para uma revisão incompleta. `--rebuild` força nova geração utilizando o histórico; `--fresh` pede um início sem esse contexto. O histórico pertence ao caminho canônico do documento: mover ou copiar a receita estabelece outro histórico local.

## Cache, proveniência e reprodutibilidade

O cache preserva revisões aprovadas, inventário de fontes e hashes. Ele permite reutilizar um resultado existente e identificar alterações ou inconsistências nos arquivos registrados. Essa função é diferente de reproduzir uma geração.

Repetir uma chamada ao modelo pode produzir fontes distintos. Recompilar os mesmos fontes pode produzir bytes diferentes conforme compilador, dependências e ambiente. Uma pesquisa de reprodutibilidade precisaria controlar essas variáveis e especificar qual propriedade pretende reproduzir: comportamento, fontes ou binário. Um hash identifica bytes; não prova correção, segurança ou equivalência funcional.

## O projeto continua útil fora da engine

Cada revisão aprovada inclui um pacote de fontes. O manifesto `CREXE-PROJECT.json` registra os arquivos e operações; scripts permitem recompilação com as ferramentas exigidas, sem nova geração por IA. A CLI também oferece build, teste, execução, publicação local e exportação de projetos existentes.

Essas operações têm limites distintos. Exportar fontes editados não valida automaticamente as mudanças. Publicar produz arquivos locais, não realiza deploy. No perfil .NET, a publicação depende do runtime. SDKs, bibliotecas do sistema e condições de execução continuam relevantes, conforme o [contrato de projetos](../PROJETOS_V1.md).

## Confiança e limites de segurança

Na v1, compilação e execução ocorrem no host com as permissões do usuário. A pasta temporária organiza o trabalho, mas não restringe acesso a arquivos, rede ou outros recursos. Validação de caminhos, limites de resposta, política de ferramentas e cancelamento tratam falhas específicas; não tornam código desconhecido seguro.

Também é necessário distinguir teste de confirmação independente. Um autoteste produzido pelo mesmo modelo pode repetir uma interpretação incorreta do pedido. Compilar e passar nesse teste são evidências úteis, porém insuficientes para afirmar que o aplicativo cumpre todos os requisitos. Os [limites de execução](../../SECURITY.md) devem orientar qualquer avaliação do projeto.

## Como avaliar a proposta

Uma avaliação útil compara o percurso completo com alternativas apropriadas, usando tarefas e resultados de aceitação previamente definidos.

| Dimensão | Evidência a registrar |
|---|---|
| Correção | Casos de teste independentes, falhas e regressões após edição |
| Experiência | Tempo até o primeiro uso, intervenções necessárias e clareza dos diagnósticos |
| Custo | Inferência, instalação de ferramentas, dependências, armazenamento e manutenção |
| Portabilidade | Hosts e arquiteturas realmente testados, com seus pré-requisitos |
| Continuidade | Preservação das regras anteriores e recuperação após interrupções |

Não há redução garantida de banda ou processamento. Uma receita pequena pode exigir modelos, SDKs e dependências grandes. A primeira geração e a reabertura pelo cache precisam ser medidas separadamente. A [distribuição atual](../DISTRIBUICAO_V1.md) distingue snapshots de desenvolvimento de uma release pública validada.

## Abertura e próximos passos

A engine mantém a GNU GPL v3 do repositório. O usuário escolhe a licença do aplicativo gerado, inclusive proprietária, sem obrigação de abrir os fontes ou creditar o CREXE. A concessão MIT-0 dos materiais que o CREXE incorpora à saída e das receitas oficiais está formalizada em [LICENSE-GENERATED](../../LICENSE-GENERATED); este whitepaper apenas a descreve. Dependências, assets e receitas de terceiros, assim como os termos do modelo/provider, mantêm suas condições.

O avanço depende de evidências sobre qualidade, compatibilidade, segurança e operação em máquinas reais. A [visão de computação regenerável](whitepaper-computacao-regeneravel.md) amplia a hipótese para um horizonte mais longo, preservando essa separação entre capacidade demonstrada e pesquisa proposta.
