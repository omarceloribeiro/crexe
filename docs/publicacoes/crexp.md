# CREXP — Creative Executable Package

**Status: prévia mínima na branch `feature/v1`; arquitetura completa no roadmap.** CREXP é um ZIP de especificações: vários documentos Markdown descrevem o software que você quer gerar. A primeira entrega transforma esses documentos em uma intenção textual e reutiliza o fluxo de geração, compilação, testes e correção da engine.

Este é o guia mantido da entrega. O [handoff completo original em português](../formatos/originais/CREXP-HANDOFF-COMPLETO.md) está disponível sem alterações. O handoff inclui propostas futuras, pseudocódigo e licenciamento anterior; a [política vigente](../LICENCIAMENTO_E_ECOSSISTEMA.md) é MIT para a engine e MIT-0 para os materiais de saída designados.

## Três arquivos, três usos

| Formato | O que você compartilha | Ao abrir |
| --- | --- | --- |
| `.crexe` textual | Uma receita simples | Gera ou reutiliza a revisão aceita no cache |
| `.crexp` | Um ZIP com vários documentos SDD em Markdown | Consolida as especificações e segue o fluxo de geração |
| `.crexb` | Receita e snapshot executável aceito | Abre o payload compatível sem IA nem recompilação |

SDD significa desenvolvimento orientado por especificação. Requisitos, plano e testes de aceitação podem ocupar arquivos diferentes. Um ZIP reconhecido como CREXP ou CREXB também pode usar a extensão universal `.crexe`; o conteúdo decide o fluxo. Um ZIP genérico não vira uma receita apenas por ser renomeado.

## Escopo da prévia

A entrega mínima aceita documentos Markdown UTF-8, com nomes internos ASCII portáveis, e um manifesto `crexp.yaml` opcional. O texto pode estar em português, inglês ou outro idioma; o nome externo do pacote pode usar Unicode. Os documentos são consolidados deterministicamente por prioridade, papel e caminho, sem uma chamada extra à LLM para interpretar o pacote. O build e as tentativas de correção continuam seguindo as configurações normais da engine, com um perfil local configurado.

O pacote sem manifesto precisa ter uma estrutura SDD reconhecível, como `spec.md`, `plan.md` e `tasks.md`. Não basta incluir um Markdown qualquer em um ZIP arbitrário. Com manifesto, declare os documentos que formam a intenção. A prévia não interpreta semanticamente requisitos conflitantes, não implementa agentes e não copia assets ou código semente para o projeto.

## Montar um pacote

Para começar com um exemplo pronto, [baixe calculadora.crexp](../../examples/crexp/calculadora.crexp). O pacote contém `requirements.md`, `tests.md` e `crexp.yaml`, todos com textos em português. É uma especificação para geração, não um programa compilado. Os [fontes e instruções do exemplo](../../examples/crexp/README.md) também estão disponíveis.

Coloque os arquivos diretamente na raiz do ZIP, sem uma pasta externa envolvendo o conteúdo:

```text
calculadora.crexp
├── crexp.yaml
├── spec.md
├── plan.md
└── acceptance.md
```

Um manifesto mínimo usa:

```yaml
crexp: 1
package:
  name: Calculadora
  id: org.example.calculadora
  version: 1.0.0
documents:
  - path: spec.md
    role: requirements
    priority: 100
  - path: plan.md
    role: plan
    priority: 20
  - path: acceptance.md
    role: acceptance
    priority: 90
```

`package.id`, `package.version`, `role` e `priority` são opcionais. O manifesto deve listar todos os documentos Markdown exatamente uma vez. Eles precisam existir e ser Markdown aceito. Os campos acima são o subconjunto da prévia: o manifesto extenso do handoff não deve ser tratado como configuração já suportada. `role` e `priority` organizam a entrada; não provam que requisitos contraditórios foram resolvidos.

Em `spec.md`, descreva as quatro operações e a interface. Em `acceptance.md`, registre, por exemplo, que `6 × 7` deve resultar em `42` e que divisão por zero deve apresentar erro. Esses critérios entram na intenção; o texto por si só não vira uma suíte formal executada pela engine.

## Inspecionar e gerar

Com a engine compilada, na raiz do checkout:

```powershell
.\target\release\crexe.exe inspect calculadora.crexp
.\target\release\crexe.exe exec calculadora.crexp
```

Em macOS/Linux, use `./target/release/crexe`. Em um pacote da engine, use o executável da pasta extraída. `inspect` permite verificar a entrada sem chamar o modelo. Executar o pacote pode chamar o provider configurado e requer os SDKs do perfil. As [instruções de configuração](../CONFIGURACAO.md) e os [perfis](../PERFIS_V1.md) continuam aplicáveis.

A consolidação para geração e seu inventário ficam em `packages/recipes/<hash>/` sob a raiz de instalação da engine, respeitando `CREXE_HOME`. Inspecionar não escreve esses arquivos. A materialização recusa conteúdo adulterado em vez de sobrescrevê-lo. Ordenação determinística dos documentos não significa que o modelo produzirá sempre o mesmo código.

O hash identifica a materialização do conteúdo; o histórico de revisões usa o caminho original do `.crexp`. Editar o pacote nesse caminho pode evoluir a última revisão aceita. Uma tentativa nova que falha não substitui a baseline aceita usada para exportar um CREXB.

No Windows, `install` e `associate` incluem `.crexp` e `.crexb` na abertura por duplo clique. Não há associação global de `.zip`, e o editor textual da engine não edita pacotes ZIP. Nos demais sistemas, use a CLI desta prévia.

## Conteúdo e privacidade

**Os documentos do CREXP são a receita.** Ao gerar com um provider remoto, a intenção consolidada desses documentos é enviada ao modelo configurado. Não inclua segredos ou dados que você não queira enviar. Escolher um modelo local mantém essa etapa no ambiente local, sujeito à configuração e às dependências utilizadas.

Esse contrato é diferente de [Gerar com CREXE](../GERAR_COM_CREXE.md): naquele fluxo, apenas a extensão do documento comum é usada para planejar o leitor, e nome, caminho, conteúdo e metadados do documento não vão para o modelo. CREXP e CREXB são entradas da engine, não documentos comuns para o gerador de leitores.

## Validação do ZIP

A engine valida a estrutura e aplica limites antes de interpretar os documentos. Caminhos que escapam da raiz, nomes ambíguos, links simbólicos, entradas duplicadas e arquivos fora do subconjunto suportado são motivos para rejeição. Arquivos inválidos não devem ser reinterpretados como texto para tentar continuar.

Na prévia, os limites do CREXP são 8 MiB para o ZIP, 1 MiB expandido, 128 entradas, 256 KiB por arquivo, profundidade de 12 níveis e razão de compressão máxima de 200. ZIP64, arquivos criptografados e arquivos divididos em volumes não são aceitos. Esses limites são de implementação, não os limites mais amplos propostos no handoff.

Essas verificações protegem a leitura do pacote. Elas não isolam o compilador ou o aplicativo gerado: a execução continua no host, com as permissões do usuário. Leia os [limites de segurança](../../SECURITY.md).

## Da prévia à proposta completa

O [roadmap dos formatos](roadmap-formatos.md) mantém explícito o trabalho restante: classificação rica de documentos, precedência e conflitos, Recipe Compiler, inventário, receita resolvida, lock files, recursos auxiliares, agentes, resolvers, publicação e políticas de confiança.

O objetivo completo é preservar a intenção de um software independentemente de linguagem, framework ou geração tecnológica. A prévia demonstra o primeiro passo: vários documentos SDD em um arquivo que a engine consegue ler e usar para gerar.

## Downloads e referências

- [Handoff CREXP completo — Markdown original em português](../formatos/originais/CREXP-HANDOFF-COMPLETO.md).
- [Hashes SHA-256 dos originais](../formatos/originais/checksums.sha256).
- [CREXB: distribuir o resultado aceito](crexb.md).
- [Roadmap completo dos dois formatos](roadmap-formatos.md).

O site oferece este guia integralmente em português e inglês pelo seletor de idioma. O handoff completo permanece no original em português.
