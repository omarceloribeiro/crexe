# Site público — crexe.org

Código da landing pública e da biblioteca do CREXE, separado da engine Rust. Páginas e downloads estáticos em português e inglês, responsivos e sem chamadas a modelos. O contador da engine usa uma Cloudflare Pages Function que consulta os agregados públicos do GitHub Releases; não usa cookies nem identificação de visitantes. O build usa Node.js 22+, Git e `markdown-it`, instalado pelo lockfile; o parser não é enviado ao navegador.

## Executar localmente

Na raiz do repositório:

```sh
npm --prefix src/site ci
npm --prefix src/site run dev
```

Abra `http://127.0.0.1:4173`. O servidor faz o build ao iniciar e serve somente `src/site/dist`, no loopback. Encerre com `Ctrl+C`. Após editar páginas, execute `npm --prefix src/site run build` em outro terminal e atualize o navegador. Após editar o Worker, reinicie o preview para carregar o novo módulo.

## Gerar e verificar

```sh
npm --prefix src/site run check
npm --prefix src/site run build
```

`check` inclui o build, testes do renderer Markdown, sintaxe JavaScript, links locais e âncoras de todas as páginas, sitemap, paridade com os documentos/exemplos originais, regras do catálogo de releases, inventário do ZIP e SHA-256. Também serve o resultado por HTTP no loopback: verifica redirecionamentos HTML, bytes dos downloads, tipos de conteúdo, cabeçalhos e respostas 404. Não compila a engine nem chama providers. `dist/` e `node_modules/` são gerados e ignorados pelo Git.

A CI executa o mesmo `npm ci` e `check` em um job Ubuntu com Node.js 22. Esse job verifica o site independentemente da matriz Rust e dos testes de leitores gerados; não publica no Cloudflare Pages nem cria releases.

Para hospedar em `crexe.org` com o contador, publique **apenas o conteúdo de `src/site/dist`** no Cloudflare Pages com Functions. Não publique a raiz do repositório. Os assets usam URLs relativas; o canonical e o sitemap identificam `https://crexe.org/`. Configure os tipos `.crexe` como `application/octet-stream`, `.md`/`.sha256` como `text/plain; charset=utf-8` e `.xml` como `application/xml`. O preview local já define os tipos. Nenhum deploy, alteração de DNS ou release é executado por estes scripts.

O build inclui `_headers` com as regras de download do Cloudflare Pages e `404.html` bilíngue para endereços inexistentes. A página 404 usa links à raiz do domínio, pois pode ser servida em qualquer profundidade. O servidor de preview aceita também URLs HTML sem extensão, como no Pages. Referências: [cabeçalhos](https://developers.cloudflare.com/pages/configuration/headers/) e [resolução de páginas e 404](https://developers.cloudflare.com/pages/configuration/serving-pages/).

## Preparar a pasta de publicação

Use uma revisão commitada e um checkout limpo. A saída deve ser nova ou vazia, fora do checkout; relatórios operacionais ficam separados do conteúdo público:

```sh
npm --prefix src/site run package -- C:/entrega/cloudflare-pages C:/entrega/operacao
```

O comando executa `check`, copia somente o site publicável e valida novamente a cópia por HTTP. Produz `SITE-INVENTORY.json` com revisão, tamanhos e hashes e `SITE-HTTP.json` com os resultados. Verifica o limite de 25 MiB por arquivo e de 20.000 arquivos do plano Free do Pages. Consulte os [limites atuais do serviço](https://developers.cloudflare.com/pages/platform/limits/) ao publicar.

Para visualizar a pasta entregue sem reconstruí-la:

```sh
npm --prefix src/site run dev -- --directory C:/entrega/cloudflare-pages
```

No Cloudflare Pages com integração Git, selecione a raiz do repositório, Node.js 22+ e o comando de build `npm --prefix src/site ci && npm --prefix src/site run check`; a saída é `src/site/dist`. Para upload direto, envie o conteúdo da pasta pública com `index.html`, `_headers`, `_worker.js` e `_routes.json` na raiz. O contador exige execução de Pages Functions (advanced mode), sem banco ou secrets. Não envie a pasta de operação, invólucros da CI ou pacotes nativos ao Pages.

## Contador de downloads

A landing PT/EN mostra o total e os números por plataforma dos pacotes exatos configurados em `downloads.json`, somente da versão anunciada. Inclui downloads iniciados no site e diretamente no GitHub. Não soma checksums, fontes, documentos, versões anteriores ou pessoas únicas. O GitHub fornece `download_count`; CREXE não incrementa cliques nem acrescenta telemetria à engine.

`GET /api/download-counts` usa o Worker entregue em `_worker.js` e cache de aproximadamente 15 minutos no Cloudflare. `_routes.json` restringe a Function a esse endpoint: páginas e downloads continuam estáticos. O navegador consulta apenas o próprio domínio. O Worker consulta o repositório fixo `omarceloribeiro/crexe` sem token e sem encaminhar cookies, IP ou cabeçalhos do visitante. O código do contador não registra visitantes; isso não altera a política de logs dos provedores de hospedagem/download.

Sem release configurada, a contagem fica indisponível para publicação; falhas, limites da API e assets ausentes mostram indisponibilidade, nunca zero inventado. Links de download não dependem do contador. Zero só é mostrado quando retornado pelos assets públicos verificados. Hosts puramente estáticos ainda servem o site, mas não executam esse endpoint.

O preview local executa o Worker empacotado para a rota de contagem. `check` cobre o backend com API/cache simulados, a interface nos dois idiomas e a rota HTTP sem depender do GitHub. Após cada publicação, confira a contagem ao vivo com a release pública e o endpoint da hospedagem. Operação e escopo: [português](../../docs/DOWNLOADS_E_PUBLICACAO.md) / [English](../../docs/en/DOWNLOADS_AND_PUBLICATION.md).

Os pacotes da engine pertencem ao GitHub Releases. Enquanto a release não estiver acessível sem autenticação, mantenha os URLs de binários como `null`. Depois da publicação, baixe cada pacote e checksum anonimamente, valide o hash, ative os links reais no catálogo e gere outra entrega do site. O GitHub do projeto também precisa estar público para os links externos de código-fonte funcionarem para visitantes.

## Conteúdo e downloads

- `index.html`: conteúdo, navegação, exemplos, início rápido, documentação e limites.
- `en/index.html`: edição completa da landing em inglês; assets, exemplos e downloads são compartilhados.
- `styles.css`: identidade visual e adaptação a desktop/celular, com preferência de movimento reduzido.
- `main.js`: menu móvel, calculadora ilustrativa, seleção de sistema, copiar comandos e catálogo de downloads. A calculadora é somente uma demonstração no navegador.
- `publications.json` e `publications.en.json`: catálogos editoriais PT/EN com fonte, título, categoria, data, descrição e escopo de cada publicação.
- `publications.css` e `library.js`: apresentação das páginas de leitura e impressão pelo navegador.
- `downloads.json`: links revisados de pacotes publicados e seus checksums.
- `assets/`: marca e recursos locais, sem fontes remotas.
- `scripts/`: build, verificação e servidor local.

A seção do projeto aberto apresenta a implementação, a biblioteca e os caminhos de contribuição e segurança. Os textos seguem a [revisão de licenciamento](../../docs/LICENCIAMENTO_E_ECOSSISTEMA.md): a engine usa MIT, com preservação dos avisos de copyright e licença, e a concessão MIT-0 dos materiais de saída está vigente no escopo de [LICENSE-GENERATED](../../LICENSE-GENERATED). Outras políticas, registro de marca e produtos futuros permanecem propostas, sem anúncio de disponibilidade.

As duas landings apresentam **Gerar com CREXE**: selecionar um documento comum para gerar ou reutilizar um leitor que o abra. O texto distingue o menu Windows do seletor da janela em macOS/Linux, descreve o contrato inicial de leitura, a chamada única de planejamento por preparação do leitor e o uso do cache. O site não recebe documentos nem gera programas no navegador. A referência de implementação vem dos guias do projeto em português e inglês; ensaios de CI, modelo real e desktop não devem ser apresentados como equivalentes.

Os exemplos `.crexe` e a documentação são copiados de suas fontes mantidas no repositório. Edite os originais e gere novamente o site. Não mantenha uma segunda cópia editorial desses documentos nesta pasta.

CREXP e CREXB têm guias e roadmap nas duas edições da landing e da biblioteca. Os quatro originais em `docs/formatos/originais/` são preservados byte a byte: handoffs completos em português, schema e manifesto ilustrativo. O build publica o schema também em `schemas/crexb-manifest-v1.schema.json`, conforme seu `$id`. O check fixa os hashes dos originais e verifica que os botões nas duas edições identificam corretamente o idioma português. Os guias ingleses traduzem integralmente os guias mantidos, sem se apresentar como traduções integrais dos handoffs. O manifesto de exemplo contém placeholders e não é um aplicativo executável.

O exemplo real `examples/crexp/calculadora.crexp` é copiado para download nas duas edições. É um ZIP de SDD em português, não um programa compilado. Para alterar seu conteúdo, edite os fontes em `examples/crexp/calculadora/`, execute `python scripts/crexp_example.py` e valide com `--check`; a CI verifica a correspondência do ZIP determinístico. Os formatos `.crexp` e `.crexb` usam `application/octet-stream` e cabeçalhos de download no preview e em `_headers`.

## Biblioteca e publicação de textos

A coleção editorial está em [docs/publicacoes](../../docs/publicacoes/README.md). Os white papers, artigos, guias e o manifesto distinguem fatos da implementação, análise e visão de futuro. O catálogo também apresenta referências técnicas existentes como páginas de leitura. Os rascunhos históricos permanecem disponíveis, identificados como históricos.

O build cria `dist/biblioteca/index.html` (português), `dist/en/library/index.html` (inglês) e uma página por entrada de cada catálogo, com sumário, tempo estimado de leitura, data, escopo, links relacionados e download do Markdown. A leitura, navegação, troca de idioma e os sumários funcionam sem JavaScript; o botão de impressão é um aprimoramento opcional e usa os recursos do navegador, inclusive salvar como PDF quando disponível.

Para adicionar um texto, mantenha o Markdown e sua tradução no repositório e inclua uma entrada em cada catálogo. Use o mesmo `slug` único e categoria nas duas edições, data da revisão editorial em `YYYY-MM-DD` e um aviso de escopo preciso. O build rejeita uma edição incompleta. Coloque os novos documentos sob controle do Git antes do build: a coleta de documentação aceita apenas arquivos rastreados. Execute `check` após alterar conteúdo ou links.

As traduções inglesas ficam em `docs/en/`. O RFC português está em `docs/publicacoes/rfc-draft.pt-BR.md`; o original inglês histórico permanece intacto e tem download próprio na tradução. A licença MIT da engine, a licença MIT-0 dos materiais de saída e os exemplos executáveis são publicados a partir dos arquivos mantidos no repositório. O acervo histórico não pertencente à coleção publicada mantém seus idiomas originais.

Os links PT/EN levam à página equivalente. `lang`, `hreflang`, canonical e sitemap identificam as duas edições; não há redirecionamento automático por idioma do navegador. As mensagens dinâmicas usam o idioma da página. O site traduz a documentação, não a interface nativa da engine.

O renderer não permite HTML bruto no Markdown nem URLs de scripts. Links relativos para textos do catálogo levam à versão HTML; outros apontam para as fontes incluídas no site. Os testes verificam essa transformação e a navegação entre páginas. As datas não são inferidas do relógio da máquina. O sitemap inclui automaticamente as páginas da biblioteca.

O download de código-fonte é criado por `git archive` do **commit HEAD**, com o hash no nome e um arquivo SHA-256 correspondente. Não contém mudanças ainda não commitadas, arquivos ignorados, credenciais locais, caches nem o código do próprio site. Os arquivos históricos de `docs/archive` também ficam fora do ZIP. O `source.json` registra a revisão exata. Os links no HTML são preenchidos no build e funcionam sem JavaScript.

O catálogo anuncia a primeira pré-release pública, `v1.0.0-preview.1`, com snapshots de desenvolvimento para Windows/x86_64, macOS/Apple Silicon e Linux/x86_64. Não é uma versão estável. Os botões nativos só aparecem após o carregamento de um catálogo válido; exemplos e fontes também são downloads na publicação gerada. A publicação dos pacotes no GitHub não configura a hospedagem nem o DNS de `crexe.org`.

Para ativar um pacote após publicar e verificar a release, preencha `version`, `releaseUrl` e os dois URLs da plataforma em `downloads.json`:

```json
{
  "version": "v1.0.0",
  "releaseUrl": "https://github.com/omarceloribeiro/crexe/releases/tag/v1.0.0",
  "platforms": {
    "windows": {
      "url": "https://github.com/omarceloribeiro/crexe/releases/download/v1.0.0/crexe-windows-x86_64.zip",
      "checksumUrl": "https://github.com/omarceloribeiro/crexe/releases/download/v1.0.0/crexe-windows-x86_64.zip.sha256"
    },
    "linux": { "url": null, "checksumUrl": null },
    "macos": { "url": null, "checksumUrl": null }
  }
}
```

Esse JSON é um exemplo de configuração, **não uma release existente**. Use os nomes reais dos assets. O build rejeita pares incompletos, HTTP, credenciais em URLs e links GitHub sem a tag exata da versão. Não consulta a API do GitHub nem prova que os URLs remotos existem: a disponibilidade e os hashes devem ser conferidos antes de ativá-los. Plataformas com ambos os campos `null` continuam em preparação. Sem JavaScript, o catálogo permanece acessível pelo link no aviso de downloads.

A cada nova release, revise também o estágio do projeto e o início rápido no HTML. A engine não inclui SDKs ou modelos. Não anuncie isolamento de segurança, builds universais, estabilidade ou capacidades além do que foi validado e documentado.
