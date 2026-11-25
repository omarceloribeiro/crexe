# Plano: abertura, configuração e cliques repetidos na v1

Data: 25/09/2026. Base analisada: `826c46e`, branch `feature/v1`. Status: **implementado, em validação integrada**. O registro de implementação e os relatórios distinguem testes automatizados de aceitação manual ainda pendente. Complementa o plano principal; não declara a v1 pronta para publicação.

## 1. Resultado esperado e decisões

O usuário instala a engine, escolhe provider/modelo e informa a chave em uma janela simples. Ao abrir um `.crexe`, acompanha uma única preparação e recebe o aplicativo visível. Salvar a configuração vale para a próxima execução, sem reinstalação.

Decisões confirmadas pelo autor:

- A configuração terá uma janela nativa em Windows, macOS e Linux, usando a mesma base Rust.
- Cliques repetidos durante geração, build, testes ou abertura inicial reaproveitam a preparação existente.
- Depois que o aplicativo abriu, outro duplo clique pode abrir **outra instância**. Não implementar instância única permanente para os aplicativos.

Continuam valendo: instalação por usuário em caminho estável, Ollama e OpenAI intercambiáveis, ausência de fallback pago automático, workspace temporário no host e proibição de Windows Sandbox. Esta mudança não inclui atualização de modelos, instalação de SDKs, migração para outra API de geração ou interface de configuração em servidor web.

### Diagnóstico confirmado

- O TOML editado pelo autor era `config.example.toml`, dentro do pacote Windows extraído. A leitura limitada ao campo `default_provider` confirmou `openai`. A engine não lê esse arquivo na instalação; `install` copia somente o executável. Sem configuração ativa, usa os defaults embutidos que selecionam Ollama.
- A engine chama `presentation::finish()` antes de iniciar o aplicativo, tanto após geração quanto no cache hit. Não existe transferência explícita de foco para a janela do filho. Isso explica uma hipótese plausível de abertura atrás das demais janelas; ainda é necessário medir minimização e foco separadamente.
- Existe trava por fingerprint de cache. Uma segunda execução recebe erro de concorrência, em vez de localizar a preparação existente. Mudanças na intenção/configuração podem produzir outra chave de cache.
- Credenciais são resolvidas por variável de ambiente, `.env` explícito e ambiente de usuário Windows. Não existe editor visual nem armazenamento no cofre do sistema.

## 2. Abertura visível e coordenação de execuções

### Transferência para a janela do aplicativo

- Separar o início do processo de seu acompanhamento pelo executor. Acrescentar um evento de aplicativo iniciado/pronto, mantendo timeout, cancelamento, captura e supervisão dos processos existentes.
- No Windows com `--ui`, manter a espera na fase “Abrindo seu programa…” enquanto o processo inicia. Localizar a janela principal visível pelo PID criado ou seus descendentes supervisionados, nunca por título ou por qualquer janela do sistema.
- Restaurar a janela quando estiver minimizada e solicitar ativação/foreground usando APIs Win32. Transferir a permissão de foreground ao processo filho quando possível; fechar a espera após a transferência, em vez de fechá-la antes do spawn.
- Limitar a procura por janela a 15 segundos, interrompendo antes se o processo sair ou houver cancelamento. Perfis de console não aguardam uma janela gráfica. A ausência de janela nesse prazo não mata um aplicativo ainda iniciando; termina a espera e registra o resultado de ativação.
- Se o OS recusar a ativação, manter o aplicativo restaurado e indicar atenção na barra de tarefas. Não usar simulação de teclado, mudança global de regras de foco ou `TopMost` permanente. A [documentação de SetForegroundWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setforegroundwindow) impõe condições para ativação; o teste não deve confundir recusa do OS com minimização.
- Aplicar o mesmo fluxo à geração nova e ao cache hit. Manter a especialização Win32 em módulo separado com `cfg`; comandos de terminal não ganham manipulação compulsória de foco. A configuração gráfica será multiplataforma, mas esta correção de ativação trata o defeito Windows observado.

### Cliques repetidos

- Criar uma sessão de preparação por usuário/raiz CREXE e caminho canônico do `.crexe`, adquirida antes de abrir a janela de espera. Registrar também a assinatura da requisição efetiva, considerando conteúdo, configuração e opções de execução, sem segredos.
- Usar lock do OS para a posse da sessão e IPC local com `interprocess` — named pipes no Windows, Unix-domain sockets nos demais sistemas. Restringir o canal ao usuário e limitar mensagens; não criar serviço permanente nem porta TCP. [Referência de IPC](https://docs.rs/interprocess/latest/interprocess/local_socket/index.html).
- A segunda abertura idêntica pede à sessão existente para apresentar sua janela e termina sem chamar o provider, compilar, criar outro progresso ou enfileirar outro lançamento. No terminal, informa que o arquivo já está em preparação.
- Se o mesmo arquivo ou as opções mudaram durante a preparação, apresentar o trabalho existente e informar que a alteração exige terminar ou cancelar essa preparação antes de abrir novamente. Não substituir o prompt de uma chamada ativa nem iniciar outra geração silenciosamente.
- Liberar a sessão após o tratamento da abertura inicial, ou na conclusão de `--no-run`, falha ou cancelamento. Não manter essa trava durante toda a vida do aplicativo. O próximo clique poderá abrir outra instância, conforme a escolha do autor.
- Preservar a trava transacional de cache como proteção complementar. Intenções diferentes podem ser preparadas em paralelo quando não disputam a mesma entrada de cache.
- Recuperar sessão abandonada pela disponibilidade do lock, não somente por PID gravado. Metadados antigos não podem impedir nova execução após crash. A comunicação na curta corrida de inicialização deve ter retry limitado a três segundos, sem disparar uma geração duplicada se o proprietário continua ativo.

## 3. Configuração visual e credenciais

### Interface e acesso

- Adicionar `crexe configure` ao mesmo executável. Abrir a mesma tela ao executar a engine sem argumentos em ambiente gráfico; `--help` mantém a ajuda. Os demais comandos continuam sem inicializar backend gráfico ou cofre.
- Usar `egui/eframe`, com renderer Glow, fontes embutidas, acessibilidade e backends X11/Wayland no Linux. Fixar dependências no lockfile. Separar estado/validação/persistência da interface; a janela não contém a lógica do provider. [Referência do framework](https://docs.rs/eframe/latest/eframe/).
- Exibir provider Ollama/OpenAI, modelo, campo de chave mascarado quando aplicável, estado “chave configurada”, e botões Salvar, Cancelar, Testar conexão e Atualizar modelos. Endpoint e limites existentes ficam em uma área avançada. Manter a tela pequena, em português, com navegação por teclado e escala de DPI.
- Listar modelos Ollama via `GET /api/tags`; para OpenAI, consultar `GET /v1/models` apenas ao solicitar atualização/teste. Manter modelo editável, valor atual e indicação de que disponibilidade não comprova compatibilidade com o adaptador de geração. Não recomendar automaticamente modelos novos nem baixar modelos. Referências: [Ollama](https://docs.ollama.com/api/tags) e [OpenAI](https://developers.openai.com/api/reference/resources/models).
- Testar conexão verifica serviço/autenticação/listagem, sem geração de texto. Operações de rede e cofre ficam fora da thread da UI; rede tem timeout de 15 segundos e erro acionável, sem exibir respostas que possam conter segredos. Salvar não depende de teste de conexão bem-sucedido.
- Oferecer “Importar TOML” por campo de caminho e comando equivalente `crexe configure --import caminho.toml`. A importação preenche os campos de provider/conexão/modelo/limites para revisão; somente Salvar aplica. Preservar políticas locais de execução e perfis que não foram editados. Não ler TOML ao lado de uma receita automaticamente.

### Configuração ativa e instalação

- Centralizar leitura e escrita no caminho já definido por OS ou em `--config` explícito. A tela mostra o destino real. `doctor` diferencia arquivo carregado de defaults embutidos e informa provider/modelo efetivos, sem credenciais.
- Salvar com validação, lock de configuração e substituição atômica, preservando campos avançados/comentários e detectando alterações externas entre carregar e salvar. Em conflito, pedir recarga na própria interface; não sobrescrever outra edição.
- Uma execução captura suas configurações ao começar. Salvar na tela afeta a próxima execução, sem trocar o provider no meio de uma geração. CLI `--provider`/`--model` mantém seus overrides explícitos.
- Na primeira instalação, criar a configuração ativa a partir dos defaults somente se ela não existir. Atualizações/reinstalações preservam a configuração e as chaves existentes. `config.example.toml` continua sendo exemplo; não vira uma segunda configuração ativa nem sobrescreve preferências ao reinstalar.
- Os instaladores interativos abrem a tela da cópia instalada ao concluir. O comando CLI `install` permanece adequado para uso não interativo. Incluir lançadores de configuração no pacote: CMD no Windows, `.command` no macOS e `.sh` no Linux, todos usando o caminho instalado ou a cópia extraída quando ainda não instalada.
- No caso relatado pelo autor, importar o TOML editado do pacote e salvar aplica a escolha OpenAI à configuração ativa. O caminho específico da máquina não deve ser embutido em scripts ou testes do produto.

### Chaves sem variável de ambiente obrigatória

- Salvar a chave fornecida na UI no cofre do usuário: Windows Credential Manager com persistência local, macOS Keychain e Secret Service no Linux. Usar `keyring-core` e os backends específicos por plataforma; não implementar criptografia própria. [Referência da abstração](https://docs.rs/keyring-core/latest/keyring_core/).
- O TOML guarda apenas a referência da credencial. Vincular a referência à configuração, ao perfil e ao endpoint, para não reaproveitar uma chave salva após troca do destino. Mostrar apenas o estado de presença; a UI não recarrega a chave salva em um campo visível.
- Regra de resolução: `--env-file` explicitamente fornecido pode substituir a credencial com a variável configurada naquele arquivo; sem esse override, uma referência de cofre usa o cofre, sem ser sobrescrita por uma variável antiga. Configurações legadas sem referência mantêm a resolução existente. Se uma referência de cofre estiver inválida/bloqueada, informar o problema, sem fallback silencioso.
- Trocar/remover chave é uma operação explícita. Ao salvar uma chave nova, gravar uma nova entrada, publicar sua referência com o TOML validado e só então remover a antiga entrada CREXE substituída. Falhas preservam a configuração/chave anterior; não tocar em entradas de outros aplicativos.
- Excluir segredo e referência do cofre da identidade efetiva de geração: trocar apenas a chave não deve regenerar programas. Preservar a identidade dos provedores legados quando seus parâmetros de geração não mudam; cobrir isso com fixtures do formato atual. Trocar provider/modelo continua selecionando a revisão correspondente conforme a política atual de cache.
- Não colocar chaves em logs, argumentos, variáveis dos processos gerados, cache, ZIP ou exemplo de configuração. Um cofre indisponível gera orientação visível; não salvar em texto puro automaticamente. O uso avançado por ambiente/`.env` continua disponível, inclusive em hosts sem desktop/cofre.

## 4. Sequência de implementação e aceitação

Executar em incrementos revisáveis na `feature/v1`: (1) reproduzir e corrigir ativação; (2) coordenar preparações; (3) persistência de configuração/cofre; (4) tela, instaladores e pacotes; (5) validação integrada e documentação. Preservar `baseline/pre-v1`; não integrar na `main` ou publicar release por causa deste plano.

| Área | Cenários obrigatórios |
|---|---|
| Foco | Fixture GUI com janela normal/minimizada, geração nova e cache hit; verificar `IsIconic` e janela de foreground separadamente. Processo com inicialização lenta, saída antecipada, aplicativo console e OS recusando ativação. Inspecionar somente janelas dos processos do ensaio. |
| Concorrência | Disparar dez aberturas próximas: uma geração, um build e um lançamento. Clique durante build/teste/abertura; cancelamento; crash; metadado antigo; alteração da intenção/configuração durante geração. Após a primeira janela pronta, outro clique abre outra instância sem gerar novamente. |
| Configuração | Instalar e reinstalar a partir de outra pasta; primeira configuração, preservação de preferências e importação do exemplo alterado. Salvar local → OpenAI e observar que a próxima chamada vai somente ao provider selecionado. Mudança inversa, configuração explícita, conflito de edição e erro de escrita. |
| Credenciais | Chave sintética salva/atualizada/removida; referência ausente, cofre bloqueado e falha parcial de persistência. Variável antiga não substitui chave da UI; override `.env` explícito funciona. Troca só de chave mantém cache. Nenhum segredo nos logs, manifestos, ZIPs ou ambiente dos filhos. |
| Interface | Windows, macOS e Linux: abrir, editar, importar, cancelar, salvar e reabrir; teclado/DPI; modelo manual, listagem indisponível e erro de autenticação. Fechar tela sem salvar não altera preferências. CLI/doctor/inspect continuam funcionando sem display. |
| Distribuição | Formatação, lint, suíte existente e novos testes nos três OS. Verificar imports/dependências dos pacotes após adicionar GUI/cofre; executar smoke de binário extraído. Exercitar backend de credenciais com entradas sintéticas exclusivas e removê-las ao fim. |

Os testes de provider usam servidores locais controlados e contagem de chamadas, sem gastos de API. Testes do cofre não usam a chave real do autor. Revalidar no Windows, fora do Codex, instalação, configuração, geração/cache, foco e cliques repetidos; repetir os smokes gráficos nos outros desktops antes de declarar a tela validada nessas plataformas. Compilar em CI não substitui conferir comportamento visual e foco.

O critério deste incremento é concluir os quatro problemas relatados com evidências registradas. Os demais critérios de release do plano principal continuam abertos.
