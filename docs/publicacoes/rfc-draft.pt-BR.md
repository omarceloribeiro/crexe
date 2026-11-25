# CREXE — Especificação Creative Executable

> Tradução para português do RFC Draft 0.1 histórico, publicada em 24/11/2026. Preserva as propostas e a terminologia normativa do original; não descreve integralmente a implementação atual. O [original em inglês](../crexe_handoff_docs/01_RFC_CREXE_SPEC_DRAFT.md) permanece disponível. Consulte o [contrato implementado dos formatos](../FORMATOS_V1.md) para o comportamento atual.

**Status:** Draft 0.1\
**Categoria:** Informativo / Padrão proposto\
**Extensão de arquivo:** `.crexe`\
**Tipo de mídia proposto:** `application/crexe+yaml`\
**Tipo de mídia experimental alternativo:** `application/x-crexe`

---

## 1. Resumo

Este documento propõe o **CREXE**, um formato de arquivo Creative Executable e um modelo de execução para distribuir software como especificações generativas em vez de binários pré-compilados.

Um arquivo `.crexe` define:

- intenção do aplicativo
- regras de comportamento
- condições da plataforma de destino
- configuração do gerador
- requisitos da cadeia de ferramentas
- instruções de compilação
- instruções de execução
- políticas de cache e segurança

Em vez de distribuir binários, o CREXE permite gerar e compilar software localmente para o sistema operacional e a arquitetura do host.

---

## 2. Motivação

A distribuição tradicional de software depende de artefatos pré-compilados:

```text
app.exe
app.dmg
app.deb
app.apk
```

Isso cria fragmentação entre:

- sistemas operacionais
- arquiteturas de CPU
- versões de runtime
- versões de dependências
- ecossistemas de empacotamento

O CREXE propõe um modelo diferente:

> Distribua intenção. Compile localmente.

Em vez de distribuir vários binários, o publicador distribui:

```text
application.crexe
```

O runtime do host gera e compila o aplicativo nativo apropriado para o ambiente atual.

---

## 3. Terminologia

### CREXE

Um arquivo de especificação Creative Executable.

### Runtime

O programa que lê, gera, compila, armazena em cache e executa arquivos `.crexe`.

### Prompt Core

A definição do aplicativo e de suas regras de comportamento, independente do sistema operacional.

### Target

Um caminho de implementação específico de um sistema operacional e de uma cadeia de ferramentas.

### Generator

A engine que transforma um prompt CREXE em arquivos de projeto. Na v1, normalmente é uma LLM online.

### Workspace

Um diretório local onde são armazenados arquivos gerados, artefatos de compilação, logs e arquivos de manifesto.

### Fingerprint

Um hash determinístico usado para decidir se é necessário regenerar ou recompilar.

---

## 4. Princípios centrais

### 4.1 Intenção antes do binário

O arquivo `.crexe` descreve o que o aplicativo é, não uma representação binária fixa dele.

### 4.2 Compilação local

Os binários são produzidos na máquina host.

### 4.3 Geração ciente do sistema operacional

O runtime injeta variáveis de ambiente como:

- `OS`
- `ARCH`
- `USER_HOME`
- `RUNTIME_VERSION`

### 4.4 Um único prompt, várias saídas nativas

Um arquivo CREXE pode definir um prompt core e diferentes envoltórios específicos para cada target.

Exemplo:

- Windows → C + API Win32
- macOS → Objective-C + Cocoa
- Linux → C + GTK3

### 4.5 Cache determinístico

Se a especificação e as entradas efetivas não mudarem, o runtime deve reutilizar a compilação armazenada em cache.

### 4.6 Formato legível por humanos

O formato recomendado para a v1 é YAML.

---

## 5. Estrutura do arquivo

Um arquivo CREXE DEVERIA incluir:

```yaml
version: 1.0

meta:
  id: "example.app"
  name: "Example App"
  description: "Description"

inputs:
  AppName:
    type: string
    default: "ExampleApp"

workspace:
  mode: cache

policies:
  timeoutSeconds:
    generate: 180
    build: 300
    run: 300

selectors:
  target:
    rules:
      - when: "{{OS}} == 'windows'"
        use: "windows"

prompt_core:
  app_definition: |
    Define the application here.

prompt:
  system: |
    You are a project generator.
  user_by_target:
    windows: |
      {{prompt_core.app_definition}}
      Generate Windows implementation.

generator:
  mode: online
  transport: responses
  model: gpt-5.5-pro

targets:
  windows:
    build:
      steps:
        - cmd: ["cmd", "/c", "..."]
    run:
      cmd: ["build\\ExampleApp.exe"]
```

---

## 6. Modelo de execução

Ao executar, o runtime DEVERIA:

1. Carregar o arquivo `.crexe`.
2. Validar o schema.
3. Resolver entradas e substituições.
4. Detectar sistema operacional e arquitetura.
5. Selecionar o target.
6. Calcular o fingerprint.
7. Se houver correspondência no cache, executar o binário armazenado.
8. Se não houver correspondência no cache:
   - montar o prompt final
   - chamar o gerador
   - validar a saída do gerador
   - gravar arquivos no workspace
   - compilar
   - armazenar o manifesto
   - executar

---

## 7. Contrato de saída do gerador

O gerador DEVE retornar um objeto JSON:

```json
{
  "files": [
    {
      "path": "relative/path",
      "content": "file content"
    }
  ],
  "hints": {
    "entrypoint": "optional",
    "notes": "optional"
  }
}
```

O runtime DEVE rejeitar:

- Markdown fora do JSON no modo estrito
- caminhos de saída absolutos
- travessia de diretórios com `../`
- `files` vazio
- `content` que não seja string

---

## 8. Cache

Um runtime DEVERIA calcular o fingerprint a partir de:

- hash do arquivo CREXE
- valores efetivos de entrada
- versão do runtime
- provider do gerador
- modelo do gerador
- transporte do gerador
- parâmetros relevantes de geração

---

## 9. Considerações de segurança

Implementações CREXE DEVEM tratar todo arquivo `.crexe` como não confiável.

Base obrigatória:

- lista de comandos permitidos
- isolamento do workspace
- nenhuma escrita fora do workspace
- validação da saída do gerador
- prevenção de travessia de diretórios
- executável gerado permitido somente dentro do workspace/cache
- rede desabilitada durante compilação/execução, salvo aprovação explícita

---

## 10. Modelo de compatibilidade

O CREXE não garante compatibilidade universal por si só.

Ele transfere a compatibilidade do momento da distribuição para o momento da geração/compilação.

Um arquivo `.crexe` é compatível se:

- existir um runtime
- existir a cadeia de ferramentas do target
- o prompt do target for válido
- o código gerado compilar com sucesso

---

## 11. Extensões futuras

- `.crexe` assinado
- registro de schemas
- metadados de marketplace
- suporte a LLM local
- modo visual de design/edição
- políticas empresariais
- contêineres de compilação isolados
- verificação de builds reproduzíveis
- lockfiles de dependências
- assinatura de pacotes

---

## 12. Conclusão

O CREXE propõe uma nova categoria de arquivo de software:

> uma especificação executável criativa.

Não é código-fonte no sentido tradicional, não é um binário e não é apenas um instalador.

É uma receita para gerar software executável localmente.
