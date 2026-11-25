# Biblioteca editorial do CREXE

Coleção em português preparada em 24/11/2026 a partir do handoff de 23/09/2026, do rascunho de white paper, dos artigos de distribuição/criação e das reflexões sobre computação regenerável fornecidos pelo autor. Os textos foram reconciliados com o estado atual do repositório; não são transcrições das conversas.

Os documentos originais do projeto permanecem no [acervo histórico](../crexe_handoff_docs/README.md). O nome vigente é **CREXE — Creative Executable**. Uma publicação informativa ou uma visão editorial não transforma o projeto em padrão aprovado por uma organização de normalização.

A coleção também tem [edição completa em inglês](../en/publicacoes/README.md). O site oferece navegação PT/EN entre as páginas equivalentes.

## White papers

- [Fundamentos da distribuição por intenção](whitepaper-fundamentos.md): modelo, contratos, custos e limites da implementação.
- [Computação regenerável](whitepaper-computacao-regeneravel.md): investigação de longo prazo, incluindo utilitários de sistema e critérios de validação.

## RFCs

- [RFC Draft 0.1 em português](rfc-draft.pt-BR.md): tradução integral, com sumário e download no site.
- [RFC Draft 0.1 — original em inglês](../crexe_handoff_docs/01_RFC_CREXE_SPEC_DRAFT.md): preservado integralmente como proposta histórica e disponível para download nas duas edições do site.

## Guias técnicos

- [Segurança e confiança](guia-seguranca-e-confianca.md): fronteiras atuais e propostas de isolamento.
- [Modelos locais, remotos e APIs do sistema](guia-modelos-locais-e-remotos.md): opções atuais da engine e possibilidades de integração.

## Artigos e manifesto

- [E se também pudéssemos distribuir intenção?](artigo-distribuir-intencao.md)
- [Além de executar: software como material de criação](artigo-alem-de-executar.md)
- [Software como RNA: uma metáfora e seus limites](ensaio-software-como-rna.md)
- [Manifesto Creative Executable](manifesto-creative-executable.md)

## Referências da implementação

O [README principal](../../README.md), o [contrato dos formatos](../FORMATOS_V1.md), a [configuração](../CONFIGURACAO.md), os [perfis](../PERFIS_V1.md), os [projetos exportados](../PROJETOS_V1.md) e os [limites de segurança](../../SECURITY.md) descrevem o comportamento entregue. O [licenciamento](../LICENCIAMENTO_E_ECOSSISTEMA.md) distingue a GNU GPL v3 da engine, a MIT-0 vigente para os materiais de saída e exemplos oficiais e as demais políticas futuras propostas.

## Critérios editoriais

- Distinguir implementação, proposta de pesquisa, metáfora e opinião.
- Não converter cache e hashes em garantia de geração ou build reproduzível.
- Não converter execução local, JSON válido ou uma lista de comandos permitidos em garantia de isolamento.
- Não anunciar integrações com APIs nativas, suporte a novos sistemas, assinaturas ou certificações antes de existirem e serem verificados.
- Não prometer redução universal de tamanho, custo ou tempo sem medições comparáveis.
- Manter exemplos executáveis alinhados aos contratos atuais; marcar pseudocódigo e propostas como tais.
- Preservar as licenças existentes. Os textos desta coleção não recebem uma nova licença CC BY, MIT ou Apache nem concedem permissões de marca; descrevem separadamente a concessão MIT-0 dos materiais de saída e exemplos em `LICENSE-GENERATED`.

## Edição e publicação no site

Estes arquivos são a fonte editorial. Os catálogos em `src/site/publications.json` e `src/site/publications.en.json` definem títulos, resumos, categorias, status e fontes por idioma. O build do site gera a biblioteca e páginas HTML com sumário, links entre textos, opção de impressão e download do Markdown. As referências técnicas são renderizadas a partir de seus arquivos originais ou de suas traduções mantidas em `docs/en/`.

Para alterar um texto, edite seu Markdown e sincronize a tradução; para acrescentar uma publicação, inclua os arquivos e suas entradas nos dois catálogos, com o mesmo slug. Rode `npm --prefix src/site run check` depois de instalar as dependências do site. A data do catálogo identifica a edição desta coleção; atualize-a quando publicar uma revisão editorial.
