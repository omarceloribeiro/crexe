# Pendências de validação e próxima atualização

Registro: 26/11/2026. Referência: `v1.0.0-preview.1`, engine `0078660`, site `25f67b1`.

Este documento registra os resultados e os ajustes adiados pelo mantenedor. Não implementa correções, não altera as permissões locais e não constitui o FAQ público. O FAQ futuro deverá ter versões em português e inglês.

## Exemplos testados pelo mantenedor

| Arquivo | Resultado observado | Interpretação |
|---|---|---|
| `regra-de-tres.crexe` | Funcionou, conforme confirmação do mantenedor. | Receita Markdown com front matter e pedidos de fundo rosa, exemplos e tooltips. Não foi feita auditoria independente de cada operação ou controle da interface. |
| `calculadora2.crexe` | `Provider connection failed; check the configured service and endpoint`. | Receita de texto simples pedindo quatro operações, limpar, divisão por zero e fundo verde. Falha no transporte da requisição ao provedor, antes de aceitar uma resposta de geração; não é evidência de erro de sintaxe ou de compilação da receita. |
| `calculator_native.crexe` | `Shell commands require execution.allow_shells = true in trusted local configuration; the recipe cannot authorize them`. | O arquivo recebido corresponde ao exemplo YAML do repositório. Seus comandos de build dependem de shells bloqueados pela configuração padrão. |

Os dois diretórios temporários indicados pelos erros foram encontrados vazios na inspeção. Não há neles logs que permitam identificar a causa específica da falha de comunicação. Nenhuma nova chamada paga ao provedor foi feita nesta análise.

O sucesso da regra de três confirma aquele uso observado, mas não demonstra que todas as chamadas ao provedor funcionarão. Também é necessário distinguir uma nova geração de uma abertura com cache; não foi coletada evidência dessa distinção no teste relatado.

## Ajustes adiados

### 1. Alinhar o exemplo YAML à instalação padrão

O exemplo [calculator_native.crexe](../examples/yaml/calculator_native.crexe) chama `cmd /c` no Windows e `sh -lc` no macOS/Linux. A política local começa com `allow_shells = false`, e a lista de comandos da receita não pode ampliar essa permissão. A checagem ocorre antes da chamada à LLM.

- [ ] Atualizar o exemplo público para funcionar com a política padrão, preferindo comandos diretos e preparação de diretórios pela engine; avaliar o tratamento de flags do `pkg-config` no Linux.
- [ ] Revisar os campos legados de provedor/modelo presentes no YAML: a engine usa a configuração local, não esses valores da receita.
- [ ] Validar a cópia exata oferecida pelo site em uma instalação limpa, com o SDK documentado, sem habilitar shells globalmente.
- [ ] Conferir geração, compilação, abertura, operações básicas e divisão por zero; registrar separadamente as plataformas efetivamente testadas.

Habilitar shells e adicionar ferramentas à configuração local é uma decisão explícita de confiança, não deve ser a solução automática do tutorial inicial. Referência: [configuração local](CONFIGURACAO.md).

### 2. Diagnosticar melhor falhas de comunicação

Em [generator.rs](../src/generator.rs), o erro relatado corresponde a falhas de envio HTTP que não foram classificadas como timeout. DNS, TLS, proxy, conexão interrompida ou serviço inacessível são hipóteses, não causas comprovadas neste caso. Timeout e respostas HTTP de autenticação, saldo ou limite têm tratamento separado.

- [ ] Na próxima reprodução, confirmar perfil e endpoint ativos e testar a conexão pela configuração; registrar horário e categoria segura da falha.
- [ ] Diferenciar erros de transporte na mensagem e no diagnóstico, sem incluir chaves, cabeçalhos de autenticação, corpos arbitrários de erro ou conteúdo privado.
- [ ] Melhorar o diagnóstico de falhas anteriores à geração, quando o diretório temporário permanece vazio.
- [ ] Validar a calculadora de texto simples com uma geração nova e registrar provider/modelo, resultado e eventual reutilização de cache.
- [ ] Preservar a política atual de não repetir automaticamente erros de conexão nem trocar de provedor silenciosamente. Reparos após build/teste são outro fluxo.

### 3. Compatibilidade dos próximos pacotes

- [ ] Linux: reduzir e fixar a versão mínima de glibc usada no build. O pacote atual requer símbolos `GLIBC_2.43`; o Debian 13 testado tem glibc 2.41 e não o executa. Instalar bibliotecas de interface ou trocar XFCE por GNOME não resolve essa incompatibilidade.
- [ ] Testar o próximo pacote em Ubuntu 22.04 e Debian 13, além do ambiente de build. O mantenedor confirmou funcionamento na VM Ubuntu mais recente utilizada nesta sessão; isso não comprova compatibilidade com distribuições anteriores.
- [ ] macOS: disponibilizar pacote Intel `x86_64` ou universal e definir/testar a versão mínima do macOS. O pacote atual é somente `aarch64`; o MacBook Intel de 2017 não está coberto.

Esses ajustes foram adiados para uma próxima release. Preservar os arquivos e a tag da pré-release atual.

## Escopo do FAQ futuro — português e inglês

- [ ] Shell bloqueado: explicar política local, exemplo legado e caminho recomendado, sem orientar a liberar execução indiscriminadamente.
- [ ] Falha de conexão: separar transporte, timeout, autenticação, saldo e limites; indicar o teste de conexão e os dados seguros úteis para suporte.
- [ ] Um exemplo funciona e outro falha: distinguir comandos/SDKs específicos, geração nova, cache e falhas de comunicação.
- [ ] `previous cache preserved`: explicar que uma revisão aprovada existente é preservada; a mensagem não garante que uma primeira geração já tenha um programa anterior.
- [ ] `Diagnostic workspace` vazio: explicar a limitação atual e o que registrar para reproduzir o problema.
- [ ] `GLIBC_x.y not found`: explicar versão mínima do pacote e bibliotecas do sistema, sem recomendar substituir manualmente a glibc.
- [ ] macOS Intel versus Apple Silicon: identificar a arquitetura do download e os requisitos mínimos.
- [ ] Configuração e SDK: distinguir credencial/provedor, instalação da engine e compiladores necessários.
- [ ] Programa padrão no Windows: explicar a confirmação no próprio sistema e como validar a abertura posterior.
- [ ] Contador de downloads: explicar os assets contados, o cache de 15 minutos e a diferença entre downloads e usuários únicos.

Publicar o FAQ somente após conferir as orientações contra a versão disponível e ligar as páginas PT/EN entre si.

## Continuação da validação da publicação

Verificações anteriores desta sessão cobriram páginas PT/EN, documentos originais, links internos, pacotes públicos e checksums. Esses resultados validam a entrega dos arquivos, não o funcionamento de todas as receitas em todos os ambientes.

Consulta inicial em 26/11/2026, aproximadamente 04:30 UTC (01:30 em São Paulo):

| Item | Resultado |
|---|---|
| `https://crexe.org/` | HTTP 200. |
| `https://crexe.org/api/download-counts` | HTTP 200, `status: available`, versão `v1.0.0-preview.1`; total 14 (Windows 5, Linux 5, macOS 4), com atualização em cache às 04:18:17 UTC. Os números são um retrato, não valores fixos esperados. |
| `https://www.crexe.org/` | HTTP 522; redirecionamento ainda não validado. |

Após o mantenedor configurar a regra, nova verificação em 26/11/2026 confirmou os quatro casos abaixo. Cada requisição retornou um único redirecionamento HTTP 301, seguido de HTTP 200 no destino:

| Origem | Destino confirmado |
|---|---|
| `http://www.crexe.org/` | `https://crexe.org/` |
| `https://www.crexe.org/` | `https://crexe.org/` |
| `http://www.crexe.org/en/?publication_probe=1` | `https://crexe.org/en/?publication_probe=1` |
| `https://www.crexe.org/en/?publication_probe=1` | `https://crexe.org/en/?publication_probe=1` |

O erro 522 não ocorreu nesses testes. O contador continuou disponível, com total 14 e atualização em cache às 04:40:03 UTC (01:40:03 em São Paulo).

- [x] Configurar no Cloudflare o redirecionamento permanente de `www.crexe.org` para `crexe.org`, preservando caminho e query string.
- [x] Validar HTTP e HTTPS para `www`, incluindo `/en/` e uma query string, até chegar ao domínio principal sem 522.
- [x] Manter os erros dos exemplos acima como pendências explícitas; não declarar a validação funcional de todos os exemplos concluída.

A validação de entrega do site, downloads, contador e redirecionamento está concluída para esta publicação. As correções funcionais dos exemplos e a ampliação da compatibilidade continuam abertas nas seções anteriores.

Referência operacional: [downloads e publicação](DOWNLOADS_E_PUBLICACAO.md).
