# Contrato de formatos — implementação em desenvolvimento

Todos usam `.crexe`, UTF-8 (BOM opcional) e LF ou CRLF. Documento acima de 1 MiB e estruturas identificadas mas inválidas falham antes da chamada ao modelo. Arquivo vazio (inclusive apenas espaços ou BOM) oferece edição no fluxo gráfico Windows e orientação para `crexe edit` na CLI. O editor permite salvar conteúdo vazio/incompleto; a validação ocorre ao executar.

## Texto puro

`gerar uma calculadora de regra de 3` é uma entrada completa. Nenhuma seção ou palavra-chave é obrigatória. O nome vem do arquivo sem extensão. O texto inteiro constitui a intenção. Uma frase com dois-pontos continua sendo texto puro.

A seleção automática oferece ao modelo os perfis suportados com SDKs disponíveis no host, incluindo OpenGL. Prefere o perfil do baseline, ou o padrão do sistema quando não existe revisão anterior. Uma mudança exige justificativa. `--profile` permite a escolha fixa, inclusive `dotnet-console` para console. O host não é emulado e dependências globais não são instaladas. Veja a [matriz e os SDKs](PERFIS_V1.md).

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

Os providers retornam `files: [{path, content}]`, com `profile`, `change_kind` (`incremental` ou `recreate`) e `reason` opcionais para compatibilidade com respostas antigas. Seleção automática pede esses metadados; mudança de perfil exige justificativa e só aceita perfis oferecidos pela engine. A engine valida toda a lista antes da escrita, incluindo caminhos relativos, duplicatas, colisões arquivo/diretório, links e limites. Cada tentativa compila em uma subpasta nova; fontes/artefatos anteriores não contaminam a seguinte.

O perfil .NET fornece `.csproj`, dependências cadastradas e comandos; o modelo fornece fontes. O build é seguido pelo `--crexe-self-test` solicitado ao gerador. Falhas devolvem fontes atuais e diagnósticos ao modelo, que deve retornar um snapshot completo. São 1–10 tentativas totais, padrão 3, respeitando o orçamento local. Nenhum SDK de agentes ou sandbox do provider é necessário.

Cada documento possui histórico independente por caminho canônico. A revisão aprovada guarda pedido resolvido, perfil, parâmetros, ancestral e inventário de fontes com hashes em `CREXE-GENERATION.json`. O índice em `cache/.documents` aponta apenas para revisões aprovadas. Novo pedido recebe o prompt e fontes textuais declarados no manifesto anterior, inclusive projetos e dependências; compilados e temporários ficam fora. O índice só muda após aprovação, e a identidade da geração inclui seu ancestral. `--fresh` desconsidera esse histórico explicitamente; `--rebuild` o utiliza.

O ZIP contém a lista de fontes e arquivos fornecidos pela engine, excluindo caches de restore, binários e credenciais. Build/run são BAT no Windows e shell nos outros sistemas; no perfil .NET há também test/publish. Argumentos incompatíveis com exportação segura de BAT produzem erro explícito.

Este contrato descreve o código atual. RFCs históricas são visão, não garantia de capacidades implementadas. A reconciliação normativa completa faz parte dos critérios de release.
