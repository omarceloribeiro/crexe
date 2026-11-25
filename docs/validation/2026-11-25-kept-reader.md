# Validação local — leitor mantido como candidato a programa padrão

Data: 25/11/2026. [Plano PT](../PLANO_PROGRAMA_PADRAO_V1.md) · [English plan](../en/PLANO_PROGRAMA_PADRAO_V1.md) · [Guia de uso](../GERAR_COM_CREXE.md).

**Estado: suíte local final aprovada; aceitação gráfica completa ainda pendente.** Este registro distingue evidência observada de comportamento esperado. Não declara o fluxo completo validado para publicação nem suporte gráfico comprovado a todas as versões de Windows 10/11.

## Ambiente e escopo

- Windows 11 Pro, versão do sistema `10.0.26300`, build `26300`.
- Fixtures usam extensões inéditas e documentos sintéticos. O provider HTTP local responde com código controlado; isso testa o protocolo, persistência e abertura, sem medir a qualidade de um modelo real.
- Os testes unitários de registro usam uma chave HKCU isolada. O teste de integração Windows registra um candidato para uma extensão sintética; não escolhe um padrão nem abre Configurações, e executa limpeza do próprio registro ao terminar.
- A fixture gráfica é explicitamente opt-in. Ela registra somente sua extensão inédita, mantém evidências e oferece limpeza própria; não modifica `UserChoice`.

## Testes automatizados observados

Rodada final, após as correções de concorrência do registro, remoção recuperável do índice e medição do timeout:

| Verificação | Resultado registrado |
|---|---|
| Formatação | `cargo fmt --all --check` passou após todas as alterações. |
| Lints | `cargo clippy --locked --all-targets -- -D warnings` passou após todas as alterações. |
| Build | `cargo build --release --locked` passou. |
| Suíte padrão | `cargo test --locked`: **148 testes passaram**, sendo 124 unitários, 17 CLI, 5 `kept_reader`, 1 `open_generating` e 1 `packages`; 16 testes ignorados por exigirem dependências ou execução específica. |
| Integração de geração | Os **9 testes ignorados de `open_generating` passaram**, incluindo `windows_kept_reader_roundtrip`, em **13,72 s**. |
| Site PT/EN | `npm --prefix src/site run check`: 79 testes, 44 páginas HTML, 2.119 links locais e 215 arquivos servidos por HTTP verificados. |
| CI remota | Ainda não registrada para o commit final nesta sessão. |

Na rodada anterior, 145 testes da suíte padrão e os 9 testes de geração haviam passado. A revisão adicionou um lock de registro compartilhado por todas as instalações do mesmo usuário e fez `forget` mover primeiro o índice para uma área recuperável, antes de remover o candidato. Se a remoção do registro falhar, tenta restaurar o índice; se essa restauração também falhar, preserva a cópia e informa seu caminho.

Um teste existente, `explicit_run_timeout`, falhou no limite de tempo da asserção durante a rodada intermediária: seu cronômetro incluía geração e build, embora a mensagem esperada `Command timed out` já estivesse correta. O teste foi ajustado para preparar antes com `--no-run`, medir a execução com cache confirmado e verificar uma única requisição ao provider. O limite de 20 s foi mantido; o teste e a suíte final passaram.

O roundtrip Windows comprovou:

1. Instalação em raiz temporária e preparação de leitor com **uma chamada de planejamento e uma de geração**.
2. Comando do candidato apontando para o executável instalado, com extensão fixa e argumento nativo do documento. A extensão não recebeu valor padrão nem `UserChoice` criado pelo teste.
3. Abertura de um segundo documento pelo launcher instalado, sem `CREXE_HOME`, com configuração indisponível, outro provider/modelo e sem credenciais de geração.
4. Abertura de um terceiro documento após remover somente o cache e a área de geração do teste e invalidar a configuração. O conteúdo correspondeu a cada arquivo, sem novas requisições ao provider nem reconstrução.
5. Preservação dos originais e ausência de seus nomes/conteúdos nos pedidos capturados. Os documentos incluem espaços, Unicode e hífen inicial no nome.
6. Limpeza do candidato sintético e esvaziamento da lista de leitores, preservando outros registros e as escolhas de padrão.

O teste acima chama o launcher diretamente. Ele não substitui a prova de duplo clique do Explorer após escolha real no Windows.

## Observação gráfica

A fixture `tests/windows_kept_reader_acceptance.py` prepara um leitor WinForms com provider controlado e duas entradas sintéticas. A primeira tentativa revelou uma limitação da própria fixture: `Console.OutputEncoding` falhava em um apphost WinExe sem console. O adaptador WinForms passou a escrever por `Console.OpenStandardOutput()`; a fixture de console original permaneceu intacta.

Na fixture B, a árvore de acessibilidade mostrou a janela do leitor com o conteúdo correto do primeiro documento. O comando `reader-defaults choose` abriu Configurações, onde foi observado o candidato com o título esperado.

A operação da tela de Configurações não pôde ser concluída: a árvore de acessibilidade não estava disponível e a captura de tela continha sobreposição de outro aplicativo. Uma tentativa de clique foi recusada porque as coordenadas correspondiam ao Docker, sem comprovação de que atingiriam o controle de Configurações. Não foi feita seleção de padrão nem validado duplo clique real.

Foi também identificado que uma consulta de associação do Shell podia retornar o único candidato de uma extensão inédita sem existir `UserChoice`. Essa consulta foi removida: o registro não é tratado como prova de escolha. A nova ação explícita oferece o convite depois de observar a janela utilizável, inclusive na reutilização. A abertura normal pelo launcher não oferece esse convite.

Na fixture C, com SHA-256 do executável `9cd31f723fe9642346ebeabc93397b48dc0015db7c5d428759ba00c2cef90970` e extensão sintética `crexeqae6c02bcc091c409e`, a acessibilidade mostrou:

- o leitor com o conteúdo correto;
- o diálogo **Leitor mantido — CREXE**, com mensagem e botões **Sim/Não**.

A ação sobre **Não** pela acessibilidade falhou por indisponibilidade de geometria, e a tentativa por teclado não confirmou fechamento. Portanto, o resultado comprova a exibição do leitor e do convite, mas não comprova cancelamento, aceitação nem escolha em Configurações. Esse binário também antecede as correções finais de registro/remoção.

## O que falta para concluir a aceitação

1. Conferir a CI do commit candidato. A suíte local final, os 9 testes de geração, formatação, clippy e build de release já passaram.
2. Em usuário de teste ou VM, operar **Sim** e **Não**, confirmar a escolha do candidato no Windows e registrar o resultado.
3. Dar duplo clique real em outro documento da mesma extensão e comprovar seu conteúdo, com zero chamadas ao provider e sem recompilação.
4. Verificar cancelamento da escolha, seleção posterior de outro aplicativo e remoção do candidato, preservando padrões e registros alheios.
5. Registrar os builds do Windows efetivamente aceitos. Windows 10 e o fallback de Configurações ainda não foram exercitados nesta sessão.

A prova automatizada já cobre a execução persistente e a privacidade do protocolo. A parte pendente é a experiência completa de escolha e abertura pela interface do sistema, além da conferência da CI do commit candidato.
