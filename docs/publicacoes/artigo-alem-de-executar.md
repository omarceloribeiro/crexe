# Além de executar: o software como material de criação

> Artigo · visão editorial · 24/11/2026. Uma reflexão sobre ferramentas pessoais, baseada na experiência atual do CREXE e em possibilidades futuras explicitamente identificadas.

Uma professora recebe uma calculadora de regra de três. Ela funciona, mas a atividade da próxima aula pede algo diferente: mostrar os passos do cálculo, aceitar números decimais e explicar por que determinado valor não pode ser zero. A necessidade é pequena. A distância entre usar a ferramenta e alterá-la pode ser grande.

Planilhas, linguagens de programação e plataformas visuais já oferecem caminhos para essa adaptação. Cada alternativa tem suas vantagens. A proposta do CREXE é acrescentar outra entrada: uma descrição legível do programa que a pessoa quer criar.

## A intenção também pode ser um arquivo

Um arquivo `.crexe` pode começar com uma frase. No [exemplo de regra de três](../../examples/markdown/regra-de-tres.crexe), o pedido descreve os campos A, B e C, a relação A/B = C/X e a validação de números e divisão por zero.

Esse documento é pequeno o bastante para ser lido antes de qualquer geração. Quem recebe pode entender o objetivo e propor uma alteração. A receita não esconde sua finalidade atrás de uma interface já pronta; ela coloca o pedido em primeiro plano.

Uma edição poderia acrescentar:

```text
Mostre a fórmula utilizada e os passos do cálculo.
Explique entradas inválidas em português.
Mantenha os campos A, B e C e o resultado X.
```

Isso ainda é uma especificação a ser interpretada. Não é prova de que o resultado será correto. A pergunta passa a ser: como transformar esse pedido em uma implementação que possa ser verificada?

## O percurso criativo já tem partes concretas

Na versão em desenvolvimento, o CREXE oferece um editor para a intenção. No Windows, a associação atual inclui a opção de edição no menu de contexto; o comando `crexe edit` abre o editor em uma sessão gráfica compatível. Salvar modifica o documento. A geração acontece quando o arquivo é aberto novamente para execução.

O modelo configurado recebe o pedido e, quando existe histórico utilizável, o contexto da revisão aprovada. A engine prepara o projeto, compila e executa os testes previstos no perfil. A nova revisão só passa a ser a base aprovada após essas etapas. A alteração pode falhar; preservar a revisão anterior permite investigar sem transformar toda tentativa em atualização definitiva.

Esse percurso aproxima edição e experimentação. Não elimina SDKs, dependências, erros de compilação ou decisões de tecnologia. Organiza essas tarefas ao redor da intenção do usuário.

## Criar também é decidir como testar

Uma interface que abre não demonstra que a matemática está correta. Na calculadora da professora, a avaliação poderia incluir entradas simples com resultado conhecido, números decimais, campos vazios e denominadores inválidos. Depois viriam critérios de apresentação: a explicação é compreensível para a turma? O texto cabe na janela? A navegação por teclado funciona?

Parte desses critérios pode ser automatizada. Parte exige uso e julgamento humano. Quanto maior a consequência de um erro, maior precisa ser a exigência de revisão. Uma atividade demonstrativa e um sistema usado para decisões financeiras não devem receber o mesmo tratamento só porque ambos começaram como texto.

Especificar melhor ajuda, mas não substitui validar. Uma cultura de software por intenção precisa ensinar as duas práticas juntas.

## Compartilhar a receita e compartilhar a versão

Há duas entregas diferentes. Uma receita permite que outra pessoa interprete, adapte e gere uma implementação em seu ambiente. Um projeto exportado preserva os fontes da revisão escolhida, com as instruções e ferramentas necessárias para recompilá-los.

Essa distinção evita uma promessa enganosa: duas pessoas com o mesmo `.crexe` não recebem necessariamente programas idênticos. Modelos, configuração e ambiente podem variar. Se o objetivo é ensinar a intenção, a receita é uma boa unidade de conversa. Se o objetivo é revisar exatamente uma implementação, os fontes aprovados são a referência mais concreta.

O [contrato dos projetos exportados](../PROJETOS_V1.md) descreve as operações disponíveis. Dependências, assets e templates continuam sujeitos às suas condições de uso; a exportação não transfere automaticamente direitos irrestritos sobre todo o conteúdo.

## Um ambiente de criação é uma visão maior

O handoff imagina modos de design, personalização visual, explicação pedagógica e comparação entre pedido e resultado. São direções para um ambiente criativo futuro. Não são todas funções da engine atual.

Uma evolução útil teria que tornar as escolhas compreensíveis: o que mudou, o que foi preservado, qual modelo foi usado, o que os testes demonstraram e como recuperar uma versão anterior. O usuário precisaria conseguir interromper o processo, examinar os fontes e reconhecer o que ainda não foi validado.

Essa experiência teria valor mesmo sem produzir qualquer aplicativo para qualquer sistema. Começar por ferramentas pequenas e bem delimitadas permite aprender onde a geração ajuda e onde um componente pronto, uma planilha ou um projeto convencional continuam sendo opções melhores.

## Software como material de trabalho

O interesse do Creative Executable está nessa possibilidade de participação. Receber uma ferramenta pode ser também receber uma maneira legível de discutir seu comportamento. Uma melhoria pode começar como edição da intenção, passar por geração e testes e terminar em um projeto que outras pessoas conseguem examinar.

A criatividade não precisa esconder a engenharia. Ela ganha força quando o pedido, a implementação e as evidências permanecem próximos. É esse encontro que o CREXE procura explorar: reduzir a distância entre precisar de uma ferramenta e conseguir transformá-la com responsabilidade.

## Continue a leitura

- [Fundamentos da distribuição por intenção](whitepaper-fundamentos.md).
- [E se também pudéssemos distribuir intenção?](artigo-distribuir-intencao.md).
- [Segurança e confiança](guia-seguranca-e-confianca.md).
- [Licenciamento e ecossistema](../LICENCIAMENTO_E_ECOSSISTEMA.md).
