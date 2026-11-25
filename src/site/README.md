# Site público — crexe.org

Código da landing pública do CREXE, separado da engine Rust. Site estático em português, responsivo e sem dependências de runtime, serviços de terceiros, analytics ou chamadas a modelos. O build usa Node.js 22+ e Git; não requer `npm install`.

## Executar localmente

Na raiz do repositório:

```sh
npm --prefix src/site run dev
```

Abra `http://127.0.0.1:4173`. O servidor faz o build ao iniciar e serve somente `src/site/dist`, no loopback. Encerre com `Ctrl+C`. Após editar arquivos, execute `npm --prefix src/site run build` em outro terminal e atualize o navegador.

## Gerar e verificar

```sh
npm --prefix src/site run check
npm --prefix src/site run build
```

`check` inclui o build, valida sintaxe JavaScript, links locais e âncoras, paridade com os documentos/exemplos originais, regras do catálogo de releases, inventário do ZIP e SHA-256. Não compila a engine nem chama providers. `dist/` é gerado e ignorado pelo Git.

Para hospedar em `crexe.org`, use **apenas o conteúdo de `src/site/dist`** como raiz de um servidor estático HTTPS. Não publique a raiz do repositório. Os assets usam URLs relativas; o canonical e o sitemap identificam `https://crexe.org/`. Configure os tipos `.crexe` como `application/octet-stream`, `.md`/`.sha256` como `text/plain; charset=utf-8` e `.xml` como `application/xml`. O preview local já define os tipos. Nenhum deploy, alteração de DNS ou release é executado por estes scripts.

## Conteúdo e downloads

- `index.html`: conteúdo, navegação, exemplos, início rápido, documentação e limites.
- `styles.css`: identidade visual e adaptação a desktop/celular, com preferência de movimento reduzido.
- `main.js`: menu móvel, calculadora ilustrativa, seleção de sistema, copiar comandos e catálogo de downloads. A calculadora é somente uma demonstração no navegador.
- `downloads.json`: links revisados de pacotes publicados e seus checksums.
- `assets/`: marca e recursos locais, sem fontes remotas.
- `scripts/`: build, verificação e servidor local.

A seção de ecossistema distingue `crexe.org`, dedicado ao projeto aberto, do portal comercial planejado `crexe.bestows.solutions`. Os textos de licenciamento seguem a [revisão mantida no repositório](../../docs/LICENCIAMENTO_E_ECOSSISTEMA.md): preservam GNU GPL v3, não anunciam novas concessões MIT/Apache/CC BY, registro de marca ou produtos futuros como já disponíveis.

Os exemplos `.crexe` e a documentação são copiados de suas fontes mantidas no repositório. Edite os originais e gere novamente o site. Não mantenha uma segunda cópia editorial desses documentos nesta pasta.

O download de código-fonte é criado por `git archive` do **commit HEAD**, com o hash no nome e um arquivo SHA-256 correspondente. Não contém mudanças ainda não commitadas, arquivos ignorados, credenciais locais, caches nem o código do próprio site. Os arquivos históricos de `docs/archive` também ficam fora do ZIP. O `source.json` registra a revisão exata. Os links no HTML são preenchidos no build e funcionam sem JavaScript.

O estado inicial reflete a documentação atual: release pública em preparação. Windows/x86_64, macOS/Apple Silicon e Linux/x86_64 têm espaços próprios; nenhum botão anuncia um executável indisponível. Exemplos e fontes já são downloads reais na publicação gerada.

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

Ao entregar a primeira release, revise também o estágio do projeto e o início rápido no HTML. A engine não inclui SDKs ou modelos. Não anuncie isolamento de segurança, builds universais, estabilidade ou capacidades além do que foi validado e documentado.
