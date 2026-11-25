# Distribuir intenção: outra maneira de compartilhar software

> Artigo • visão editorial • 24/11/2026

Um aplicativo chega até nós como uma resposta pronta. Alguém escolheu suas funções, organizou a interface, compilou o código e preparou uma distribuição. Esse trabalho permite instalar e usar uma ferramenta sem reconstruir todas as decisões que a produziram. Para muitas situações, essa continua sendo uma excelente forma de entregar software.

Há, porém, necessidades pequenas e particulares que não encontram uma resposta pronta satisfatória. Uma calculadora precisa de outra apresentação. Uma atividade pede regras diferentes. Uma ferramenta resolve quase tudo, mas falta justamente a operação importante para quem a utiliza. Nesses casos, compartilhar também a descrição do que queremos construir pode abrir uma possibilidade adicional.

O CREXE explora essa possibilidade. Seu nome significa **Creative Executable**. Um arquivo `.crexe` expressa a intenção de um aplicativo e participa de um processo que gera fontes, compila e executa o resultado no computador do usuário. A proposta acrescenta uma forma de compartilhar software, ao lado de binários, código-fonte e outros formatos que continuam úteis.

## Uma descrição que pode ser transformada

Considere um arquivo chamado `calculadora.crexe` com este conteúdo:

```text
Gerar uma calculadora simples com as quatro operações,
botão de limpar, tratamento de divisão por zero e fundo verde.
```

Esse texto apresenta uma intenção compreensível antes de qualquer compilação. Quem o recebe pode avaliar o pedido, alterar as cores ou explicar melhor um comportamento. A descrição também torna visível parte das decisões que, em um executável isolado, precisariam ser descobertas pela interface ou pela documentação.

Isso não significa que o texto determine todos os detalhes do programa. Ainda será necessário escolher uma implementação, resolver ambiguidades e verificar o resultado. Uma descrição curta deixa muitas decisões em aberto: organização das telas, tratamento de entradas, acessibilidade e estrutura interna. Escrever uma receita melhor faz parte do trabalho de criação.

Na implementação atual, a engine aceita texto puro, Markdown e um formato YAML legado. Ela usa a configuração local de geração e os perfis disponíveis para preparar o projeto. Compilação e testes disponíveis no perfil antecedem a aprovação de uma revisão. Os requisitos concretos estão no [guia do projeto](../../README.md) e na [documentação de perfis](../PERFIS_V1.md).

## Editar o pedido também é projetar

Depois de experimentar a calculadora, alguém pode acrescentar: “mantenha o comportamento atual e aumente os botões para facilitar o uso em uma tela pequena”. A intenção passa a registrar uma necessidade percebida durante o uso. O arquivo deixa de ser apenas um ponto de partida e torna-se parte da conversa com a ferramenta.

O CREXE já mantém um histórico local por documento. Ao alterar a intenção, a geração recebe o pedido anterior, os fontes aprovados e o pedido atual. A orientação é preservar os comportamentos não afetados, mas essa orientação não garante que toda mudança será correta. É preciso conferir o resultado, inclusive funções que deveriam ter permanecido iguais.

Uma falha de preparação não substitui a última revisão aprovada. Reabrir sem mudanças pode reutilizar a revisão validada no cache. Esse mecanismo evita trabalho repetido nas condições previstas; ele não torna a geração determinística nem demonstra que duas máquinas produzirão os mesmos fontes. Cache e reprodutibilidade são assuntos relacionados, com exigências diferentes.

## Compartilhar a receita ou compartilhar uma versão

Uma receita e um projeto exportado atendem a objetivos distintos. A receita permite retomar o pedido e produzir uma implementação no ambiente de destino, desde que existam suporte e ferramentas adequados. Os fontes exportados preservam uma realização concreta daquele pedido e podem ser estudados, modificados e recompilados com seus requisitos.

Por isso, distribuir intenção não elimina a distribuição de fontes. No CREXE, a exportação inclui arquivos do projeto, manifesto e scripts. A pessoa que recebe esse material pode trabalhar nele sem depender da engine para recompilá-lo, conforme o [contrato das operações do projeto](../PROJETOS_V1.md). O resultado continua sujeito às dependências e às condições do ambiente.

O usuário escolhe a licença do aplicativo, inclusive proprietária, e pode vendê-lo sem obrigação de abrir os fontes ou creditar o CREXE. A engine permanece GNU GPL v3; seus materiais incorporados à saída e as receitas oficiais têm concessão MIT-0. Dependências, assets e receitas de terceiros, assim como os termos do modelo/provider, conservam suas condições. O [licenciamento](../LICENCIAMENTO_E_ECOSSISTEMA.md) explica o escopo dessa liberdade e o aviso incluído na exportação.

## A liberdade precisa de critérios

Gerar localmente não torna o código confiável por origem. Um programa compilado na própria máquina ainda pode conter erros ou realizar ações indesejadas. Na versão atual, ferramentas e aplicativos executam com as permissões do usuário; a pasta de trabalho não oferece isolamento de segurança. Ler a receita ajuda a entender o pedido, mas não substitui examinar a implementação e decidir se ela deve ser executada.

Também há custos concretos: preparar SDKs, obter dependências, aguardar geração e compilação, revisar resultados. Algumas tarefas continuarão melhor atendidas por um aplicativo já distribuído e testado. Outras podem justificar uma ferramenta personalizada. Essa escolha deve considerar o trabalho completo, sem pressupor que gerar será sempre mais rápido, barato ou simples.

O interesse do CREXE está na possibilidade de aproximar quem usa uma ferramenta das decisões que a moldam. Uma receita legível pode circular, receber ajustes e servir de base para novas experiências. O projeto ainda está em desenvolvimento. Seu convite é experimentar esse percurso com critérios claros: descrever, gerar, verificar, usar e compartilhar aquilo que foi compreendido e aprovado.
