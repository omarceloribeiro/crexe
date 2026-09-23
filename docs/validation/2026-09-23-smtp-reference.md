# Referência de entrega: SMTP Email Tester

Data: 2026-09-23. Origem: ZIP fornecido pelo autor, gerado em uma conversa no ChatGPT. Objetivo: verificar a forma de entrega que deve orientar o CREXE. Este projeto não substitui o baseline em `src/` e não foi gerado pela engine.

## Intenção e resultado esperado

O autor pediu um programa .NET 8 para testar configurações SMTP, destinatário e body, com envio autenticado ou default credentials e remetente configurável, inicialmente igual ao usuário. Terminou com: “me entregue o programa completo pronto em um zip. com instrução de como compilar, executar.”

Segundo o autor, a primeira entrega compilava e o programa era satisfatório. Não temos a execução interna da conversa para inferir número de chamadas, testes ou reparos feitos pelo chat. A reprodução local abaixo confirma a compilação dos fontes recebidos, sem modificações.

## Conteúdo inspecionado

ZIP recebido: `1-SmtpEmailTesterDotNet8.zip`, 10.361 bytes.

SHA-256: `ff12e03761607b48bd618271a5321fe740c0bd42f0bbf49cff7c67bd9b88cd3e`.

```text
SmtpEmailTesterDotNet8/
  README.md
  .gitignore
  build.cmd
  run.cmd
  publish-win-x64-self-contained.cmd
  clean.cmd
  src/SmtpEmailTester/
    SmtpEmailTester.csproj
    App.xaml
    App.xaml.cs
    MainWindow.xaml
    MainWindow.xaml.cs
```

São 11 arquivos em um projeto WPF `net8.0-windows`, sem referências explícitas a pacotes externos no `.csproj`. O código separa recursos/estilos, interface e comportamento. A implementação lida com usuário/senha, default credentials e relay sem credenciais; sincroniza o remetente com o usuário enquanto a opção correspondente está habilitada; valida campos; apresenta logs e exceções; executa o envio fora da thread da interface. A senha não é persistida pelo código inspecionado. O próprio app distingue aceitação pelo servidor SMTP de entrega na caixa de entrada.

O README explica pré-requisitos, comandos, localização das saídas, publicação self-contained, uso e limitações. Os scripts de build e publish preservam falhas e usam `cd /d "%~dp0"`, permitindo execução a partir de outro diretório. `run.cmd` usa `dotnet run` em Release. `clean.cmd` percorre diretórios `bin`/`obj` recursivamente; foi apenas lido e não executado. O CREXE deve ter limpeza limitada às saídas declaradas do projeto.

## Reprodução local

O ZIP foi extraído em uma pasta temporária com espaços e acentos, após validação dos caminhos. Um `global.json` no diretório pai, fora do conteúdo extraído, selecionou o SDK já instalado `8.0.425`. Os scripts originais foram chamados por caminho absoluto a partir do checkout CREXE, fora da pasta do projeto SMTP.

A primeira invocação do SDK iniciou uma verificação de integridade de workloads e download de um pacote Android alheio ao projeto. O processo de restore foi interrompido; a continuação do script falhou por falta do arquivo de assets, sem evidência de erro nos fontes. A execução foi retomada com `DOTNET_SKIP_WORKLOAD_INTEGRITY_CHECK=1` e `DOTNET_CLI_WORKLOAD_UPDATE_NOTIFY_DISABLE=true`, somente no ambiente do processo, além de telemetria desativada. A [documentação oficial das variáveis .NET](https://learn.microsoft.com/en-us/dotnet/core/tools/dotnet-environment-variables#dotnet_skip_workload_integrity_check) descreve o controle dessa verificação. Isso reforça a necessidade de registrar e controlar a preparação da toolchain separadamente da correção de código.

| Verificação | Resultado |
|---|---|
| `build.cmd`, SDK 8.0.425 | PASS: saída 0, zero erros e zero avisos. |
| `publish-win-x64-self-contained.cmd` | PASS: saída 0 e executável de distribuição produzido. |
| Caminho com espaços/acentos e diretório de chamada externo | PASS para os dois scripts executados. |
| Arquitetura do executável publicado | PASS: cabeçalho PE AMD64 (`0x8664`). |
| Integridade dos fontes/scripts recebidos | PASS: todos os 11 arquivos idênticos aos bytes do ZIP após build/publish. |
| Interface, `run.cmd` e uso interativo | Não executados nesta validação. |
| Envio SMTP, autenticação e entrega de email | Não testados; nenhum email enviado. |
| Máquina limpa sem SDK/runtime .NET | Não testada; esta máquina tem SDKs e runtimes instalados. |

O comando de publish usa `--self-contained true`, `PublishSingleFile=true`, inclusão de bibliotecas nativas para extração e compressão. O executável resultante tem 71.609.978 bytes. Isso comprova geração do artefato solicitado, sem equivaler a teste em outra máquina sem runtime.

Evidências: [resultado estruturado](2026-09-23-smtp-reference-results.json), [log do build](2026-09-23-smtp-reference-build.log) e [log do publish](2026-09-23-smtp-reference-publish.log). Caminhos privados foram substituídos nos logs. ZIP e binários não foram copiados para o repositório; o pacote Rust original não foi alterado.

## Decisões incorporadas ao plano

1. Entregar um projeto completo com ZIP e instruções é responsabilidade padrão da engine, mesmo que o plaintext contenha apenas a intenção do programa.
2. Gerar arquivos estruturados pelo provider, validar/compilar/testar na engine e empacotar localmente. O provider não precisa fabricar ou hospedar um ZIP.
3. Manter operações de build, run, test, publish e export no manifesto de projeto. Scripts de cada sistema derivam do mesmo plano usado pela engine e funcionam fora dela com os pré-requisitos documentados.
4. Publicação produz uma distribuição local identificada por target/modo. Um futuro item “Publish” no menu de contexto reutiliza essa operação, sem confundir publicação de binários com deploy em serviços externos.
5. Se a geração inicial passar no build e nos testes exigidos, concluir sem chamadas extras de reparo. As evidências de verificação continuam necessárias.
6. Respeitar tecnologias explícitas na intenção, como .NET 8 neste exemplo; usar a recomendação por OS somente para escolhas não especificadas. Não impor WPF ou a estrutura deste ZIP aos demais perfis.
7. Usar um servidor SMTP local de teste e credenciais fictícias em uma futura suíte automatizada deste caso; não depender de envio externo ou credenciais padrão reais.

As mudanças estão no [plano da v1](../PLANO_V1.md). São decisões e critérios de implementação; a engine ainda não oferece essas novas operações.
