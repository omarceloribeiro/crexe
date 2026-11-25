# Segurança e confiança no CREXE

> Guia técnico · 24/11/2026 · Descreve o CREXE v1 em desenvolvimento e separa os controles existentes das propostas de segurança. A referência operacional é a [política de segurança do projeto](../../SECURITY.md).

Um arquivo `.crexe` descreve um programa; a engine transforma essa descrição em fontes, compila e executa testes antes de abrir o resultado. Cada passagem introduz uma decisão de confiança. Uma descrição legível pode produzir código incorreto. Um compilador autorizado pode executar etapas de um projeto. Um teste aprovado pode deixar comportamentos importantes sem cobertura.

Na implementação atual, ferramentas e programas gerados executam **com as permissões do usuário, no host**. A pasta temporária organiza os arquivos, mas não constitui isolamento. Não há contenção geral de arquivos, rede ou subprocessos. Essa condição deve orientar a decisão de executar qualquer receita, inclusive quando o modelo é local.

## O que existe hoje

Os controles da engine tratam falhas específicas. Entender seus limites evita transformar uma proteção parcial em promessa de segurança.

| Controle atual | O que oferece | O que não oferece |
|---|---|---|
| Validação de caminhos e limites de arquivos | Restringe os arquivos materializados pela engine | Controle sobre arquivos acessados posteriormente pelo código nativo |
| Lista local de ferramentas autorizadas | Confere os executáveis iniciados pela engine, pelo caminho resolvido | Contenção dos subprocessos que essas ferramentas iniciarem |
| Credenciais no cofre do usuário | Evita guardar a chave diretamente no TOML | Impedimento geral de acesso a dados do usuário por um aplicativo |
| Remoção de variáveis de credenciais conhecidas | Reduz a herança acidental de segredos por processos filhos | Garantia de que não existam outros segredos acessíveis no host |
| Limites de respostas, logs e tempo | Contém certas falhas de recursos e permite cancelamento | Cota geral de memória, disco ou rede de todo código executado |
| Revisões e hashes do cache | Identifica o material preservado e permite reabrir uma revisão aprovada | Prova de correção funcional, autoria ou ausência de comportamento malicioso |

Provider, endpoint e credenciais são escolhas locais. Uma receita não pode ampliar a lista local de ferramentas ou o orçamento de chamadas. Ainda assim, autorizar um compilador não torna qualquer projeto seguro. As regras completas ficam na [configuração local](../CONFIGURACAO.md).

O YAML legado pode expressar `allowNetwork`, mas `false` é recusado porque o isolamento correspondente não está implementado. Não existe hoje uma política efetiva de “rede desligada por padrão” para o programa gerado. A recusa explicita uma capacidade ausente.

## Preparar e inspecionar

No Windows, a partir da raiz do repositório e com a engine já compilada, estes comandos ajudam a conhecer o ambiente:

```powershell
.\target\release\crexe.exe doctor
.\target\release\crexe.exe doctor --check
.\target\release\crexe.exe inspect examples\plaintext\calculadora.crexe
```

`doctor` mostra requisitos e caminhos sem executar os SDKs. A variante `--check` executa sondagens locais das ferramentas, sujeitas à política e a timeout; não chama o provider, instala dependências ou compila um aplicativo. `inspect` resolve a intenção e os requisitos sem executar os SDKs. Encontrar uma ferramenta não comprova sua procedência nem a segurança de uma receita.

O argumento `--no-run` tem outro propósito: durante uma nova geração, mantém a compilação e os testes definidos, inclusive código produzido pelo modelo, mas suprime a execução normal do aplicativo. Nos perfis OpenGL, a verificação gráfica também fica de fora. Reutilizar uma revisão do cache não recompila nem repete a bateria de testes. Portanto, `--no-run` não serve como modo seguro para inspecionar código desconhecido. Antes de gerar, conheça a origem da receita, revise a intenção e confirme que o ambiente é apropriado para executar o resultado com suas permissões.

Nos [perfis atuais](../PERFIS_V1.md), o autoteste é gerado junto com o programa. Ele ajuda a encontrar erros, mas pode repetir a mesma interpretação equivocada. Para decisões importantes, inclua testes independentes com resultados esperados definidos por quem conhece o problema.

## Separar as fronteiras de confiança

O fluxo pode ser lido assim:

```text
Intenção recebida
    → interpretação pelo modelo
    → arquivos produzidos
    → ferramentas de build
    → testes e aplicativo
    → arquivos, rede e outros recursos do usuário
```

Uma falha em qualquer passagem pode escapar das verificações anteriores. Responder no JSON esperado comprova a forma da resposta, não a segurança dos fontes. Da mesma maneira, instruir o modelo a ignorar pedidos perigosos não cria uma barreira de permissões. A OWASP trata a injeção de instruções como risco que exige validação, privilégio mínimo e testes de fronteiras, além das orientações dadas ao modelo. [OWASP: Prompt Injection](https://genai.owasp.org/llmrisk/llm01-prompt-injection/).

Essa separação também vale para privacidade: inferência local pode evitar enviar a intenção a um serviço remoto, mas o aplicativo resultante continua capaz de acessar a rede se o sistema permitir. Localidade do modelo e contenção do programa são propriedades distintas.

## Arquitetura proposta para evolução

As medidas abaixo são **propostas**, não opções que já possam ser ativadas na v1.

| Camada proposta | Responsabilidade verificável |
|---|---|
| Contrato estrutural | Versionar campos, tipos e limites; recusar recursos não suportados antes da execução |
| Fronteira do gerador | Tratar a resposta como dados; separar qualquer acesso a ferramentas por interfaces autorizadas |
| Isolamento do build | Conter compiladores, projetos e descendentes; controlar leituras, escritas e dependências |
| Isolamento do aplicativo | Aplicar permissões de arquivos, rede, processos e interface durante a execução |
| Capacidades declaradas | Mostrar o acesso solicitado e verificar se o backend consegue impô-lo |
| Proveniência e assinatura | Vincular identidade, intenção, fontes, dependências e artefatos efetivamente verificados |
| Revisão e recuperação | Permitir inspeção antes de executar, preservar evidências e retornar à revisão anterior |

Uma política de rede precisa definir a fase: download do modelo, geração remota, restauração de dependências e execução são operações diferentes. Um backend que não consegue impor a política escolhida deve falhar explicitamente. Escrever uma capacidade no documento não a implementa.

Assinaturas exigem escopo preciso: assinar a intenção não equivale a assinar todos os binários que modelos diferentes poderão produzir. A assinatura identifica a origem do material assinado; não demonstra que ele é inofensivo. Renovação de chaves, revogação e vínculo entre revisões também precisam de projeto.

## Evidências necessárias

A evolução deve começar com um modelo de ameaças: quais recursos precisam ser protegidos, quem fornece cada entrada e quais componentes podem estar comprometidos. Depois, cada garantia precisa de ensaios independentes. Um plano mínimo inclui:

1. Verificar que acessos fora das permissões falham, inclusive em processos descendentes e ferramentas de build.
2. Observar arquivos e conexões externamente ao aplicativo testado; sua própria declaração de sucesso não basta.
3. Exercitar cancelamento, indisponibilidade do backend, limites de recursos e recuperação sem promover revisões incompletas.
4. Separar testes de integridade, comportamento e isolamento; publicar o ambiente e as limitações de cada resultado.

A política atual não inclui Windows Sandbox, Docker ou WSL como backends de isolamento. Pesquisas futuras precisam de implementação, revisão e evidências próprias antes de mudar essa condição.

Para relatar um problema, use dados sintéticos, versão da engine e um exemplo mínimo. Não publique chaves, caches privados ou dumps do ambiente. A [política de segurança](../../SECURITY.md) permanece o ponto de referência para os limites atuais e para comunicar falhas.
