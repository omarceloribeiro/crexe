# Contrato de formatos — implementação em desenvolvimento

Todos usam `.crexe`, UTF-8 (BOM opcional) e LF ou CRLF. Arquivo vazio, documento acima de 1 MiB e estruturas identificadas mas inválidas falham antes da chamada ao modelo.

## Texto puro

`gerar uma calculadora de regra de 3` é uma entrada completa. Nenhuma seção ou palavra-chave é obrigatória. O nome vem do arquivo sem extensão. O texto inteiro constitui a intenção. Uma frase com dois-pontos continua sendo texto puro.

O Windows prefere `dotnet-winforms` quando encontra `dotnet`; se só encontra `g++`, escolhe `cpp-win32`. Sem ambos, informa o requisito .NET antes da geração. Linux seleciona `c-gtk` e macOS `objc-cocoa`. Os três perfis C/C++/Objective-C++ estão em validação experimental. `--profile` permite a escolha explícita, inclusive `dotnet-console` para console. O host não é emulado e dependências globais não são instaladas. Veja a [matriz e os SDKs](PERFIS_V1.md).

## Markdown

```markdown
---
crexe: 1
name: Regra de três
---
# Minha intenção
Uma calculadora simples para A/B = C/X.
```

O marcador reconhece Markdown estruturado. `name` e `profile` são opcionais. O corpo inteiro é a intenção; títulos não possuem significado obrigatório. Front matter sem fechamento, versão diferente ou campos desconhecidos causam erro. Markdown sem marcador é aceito como prompt livre.

## YAML legado

Chave reservada na raiz (`version`, `targets`, `selectors` ou `prompt_core`), ou um objeto JSON, identifica receita estruturada. Exige `version: 1.0`; erros não provocam fallback para plaintext. O adaptador preserva inputs, seletores, `prompt_core`, templates e targets do exemplo original.

`build.steps`, `run.cmd` e `test.steps` opcional usam listas de argumentos. O build exige ao menos um comando; o artefato esperado precisa existir no workspace. Não se escolhe outro executável automaticamente. Templates não resolvidos e inputs inválidos falham antes da geração. `generator` legado é ignorado com aviso: migre provider/modelo para configuração local. `workspace.cache.root` não controla o cache. Marcadores/layout personalizados e restrição de rede não implementada são rejeitados.

Os campos principais exigem mappings. Operações usam de 1 a 32 steps; test/publish malformados não são ignorados. `policies` aceita somente `allowNetwork`, `timeoutSeconds` e `commandAllowlist`, com tipos validados. Isso ainda não é uma validação normativa de todo campo possível dos RFCs históricos; não se devem inferir capacidades desses documentos.

## Projeto e correções

Ambos os providers retornam `files: [{path, content}]`. A engine valida toda a lista antes da escrita, incluindo caminhos relativos, duplicatas, colisões arquivo/diretório, links e limites. Cada tentativa compila em uma subpasta nova; fontes/artefatos anteriores não contaminam a seguinte.

O perfil .NET fornece `.csproj` e comandos; o modelo fornece fontes. O build é seguido pelo `--crexe-self-test` solicitado ao gerador. Falhas devolvem fontes atuais e diagnósticos ao modelo, que deve retornar um snapshot completo. Até duas correções são permitidas. Nenhum SDK de agentes ou sandbox do provider é necessário.

O ZIP contém a lista de fontes e arquivos fornecidos pela engine, excluindo caches de restore, binários e credenciais. Build/run são BAT no Windows e shell nos outros sistemas; no perfil .NET há também test/publish. Argumentos incompatíveis com exportação segura de BAT produzem erro explícito.

Este contrato descreve o código atual. RFCs históricas são visão, não garantia de capacidades implementadas. A reconciliação normativa completa faz parte dos critérios de release.
