# Downloads da engine e publicação do site

Estado: contador implementado; ativação pública depende da release e do deploy. [English](en/DOWNLOADS_AND_PUBLICATION.md).

## O que é contado

A landing mostra downloads dos arquivos de Windows, Linux e macOS da versão configurada em `src/site/downloads.json`. Cada número vem de `download_count` do respectivo asset no GitHub Releases. O total soma somente esses pacotes. Os botões do site usam os mesmos URLs dos assets; os downloads feitos diretamente no GitHub entram no mesmo número.

Não é contagem de pessoas únicas ou instalações. Não inclui checksums, ZIP de fontes, exemplos, documentos, outras versões ou assets não anunciados. Não há contagem própria de cliques, cookie, identificador de visitante ou telemetria nova na engine. Não somar esse total com outra métrica de cliques do site, pois os eventos podem se sobrepor.

## Preparar a primeira publicação

1. Validar e integrar a revisão escolhida na `main`, com o repositório ainda privado.
2. Preparar tag e release em rascunho correspondentes ao código testado; anexar os três pacotes nativos e seus `.sha256`. Os pacotes de desenvolvimento atuais devem ser identificados como pré-release, sem prometer estabilidade.
3. Tornar o repositório público conforme a decisão do mantenedor e publicar a release.
4. Conferir os seis URLs sem autenticação e verificar os hashes. O upload para Releases é independente dos artefatos temporários do Actions.
5. Preencher `downloads.json` com tag, URL da release e os nomes reais dos assets. Plataformas sem pacote permanecem com ambos os campos `null`.
6. Executar `npm --prefix src/site ci` e `npm --prefix src/site run check`, commitar a configuração e publicar o site.

O build valida o catálogo, mas não publica releases nem verifica a existência dos links remotos. A CI atual não faz o upload de assets para Releases automaticamente. O contador também não cria releases nem habilita downloads por conta própria.

## Cloudflare Pages

Use integração Git, branch de produção `main`, raiz do repositório, Node.js 22 ou superior, build `npm --prefix src/site ci && npm --prefix src/site run check` e saída `src/site/dist`. Associe `crexe.org` em Custom domains do projeto.

O build entrega `_worker.js` (Pages Functions advanced mode), `_routes.json` e `_headers` na raiz da saída. Preserve os três também no upload direto. A Function é invocada apenas em `/api/download-counts`; os arquivos estáticos seguem no serviço de assets. Não exige D1, KV, token GitHub ou variáveis secretas. Um host sem suporte à Function mantém o site e os links, mas não a contagem.

Após o deploy, abrir `/api/download-counts` deve retornar JSON com `schemaVersion: 1`, `status: "available"`, a versão configurada, total, números por plataforma e `updatedAt`. Compare os valores com os assets exatos no GitHub. Confirme as duas landings, o download de um pacote por plataforma, os checksums e os downloads de RFC/white papers. Sem catálogo publicado, `status: "unpublished"` e `total: null` são esperados.

## Cache, disponibilidade e privacidade

O navegador consulta somente o endpoint do próprio site. A Function consulta a API pública do GitHub sem credenciais e usa cache de 15 minutos no edge; respostas de indisponibilidade têm cache curto de 60 segundos. O cache é por datacenter e depende da configuração completa do catálogo, portanto uma troca de versão não reutiliza os números da anterior. `updatedAt` informa quando os dados foram obtidos, não quando ocorreu o último download. Não há promessa de atualização em tempo real.

O repositório consultado é fixo. Parâmetros da URL do visitante não selecionam repositório, release ou servidor externo. O Worker não encaminha IP, cookies ou headers do visitante à API, e não registra dados pessoais. Os provedores de hospedagem e download continuam sujeitos às suas próprias políticas de logs.

Limites de API, falha de rede, release privada/em rascunho, asset ausente ou resposta inconsistente produzem indisponibilidade, sem inventar zero ou impedir o download. Um zero real é mostrado. A contagem usa apenas assets públicos da versão anunciada, incluindo pré-releases explícitas; não depende de `releases/latest`.

## Manutenção

Para cada versão, criar nova tag/release com novos arquivos e checksums; verificar os downloads públicos; atualizar o catálogo; validar e publicar o site. Preservar releases antigas. O total mostrado muda para a nova versão, sem acumular a anterior. A automação futura de release deve fazer esses mesmos passos; ela é separada da implementação do contador.

Testes: `npm --prefix src/site run check`. Cobrem respostas do GitHub e cache simulados, integridade do agregado, estados de falha, PT/EN, build e HTTP local. Isso não substitui a conferência pública após o deploy.

## Referências

- [GitHub: arquivos de releases e download_count](https://docs.github.com/en/rest/releases/assets)
- [GitHub: gerenciamento de releases](https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository)
- [Cloudflare: Pages Functions advanced mode](https://developers.cloudflare.com/pages/functions/advanced-mode/)
- [Cloudflare: controle de invocações com _routes.json](https://developers.cloudflare.com/pages/functions/routing/)
- [Cloudflare: Cache API](https://developers.cloudflare.com/workers/runtime-apis/cache/)
