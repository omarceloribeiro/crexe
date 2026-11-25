# Licenciamento e organização do ecossistema CREXE

Política atualizada em **25/11/2026**. A engine CREXE usa **MIT**. Os materiais que o CREXE incorpora à saída e os exemplos oficiais têm a concessão adicional **MIT-0**. **O usuário escolhe a licença do aplicativo gerado, inclusive proprietária.** Os textos de [LICENSE](../LICENSE) e [LICENSE-GENERATED](../LICENSE-GENERATED) definem as condições e o escopo dessas permissões.

## Licenças atuais

| Material | Licença e alcance |
| --- | --- |
| Engine/CLI e suas implementações | MIT em [LICENSE](../LICENSE), identificada em [Cargo.toml](../Cargo.toml). Permite uso, modificação, distribuição, sublicenciamento e venda, preservando os avisos exigidos pela MIT. |
| Materiais de saída e exemplos oficiais | MIT-0 no escopo de [LICENSE-GENERATED](../LICENSE-GENERATED). Não exige atribuição nem retenção do aviso para esses materiais. |
| Aplicativo gerado | Licença escolhida pelo usuário, respeitados os direitos e termos de eventuais componentes de terceiros. O CREXE não atribui uma licença automaticamente ao aplicativo inteiro. |
| Dependências, assets, receitas e outros materiais de terceiros | Permanecem sob suas próprias licenças e condições; não são relicenciados por MIT ou MIT-0 do CREXE. |
| Documentos históricos e originais fornecidos | Preservam seus avisos e contexto. Propostas antigas de licenciamento não definem a política atual da engine. |

A MIT permite que a comunidade porte a engine, desenvolva forks, integre seu código a outros produtos e distribua versões abertas ou proprietárias. Ela não exige publicar modificações, mas exige preservar o aviso de copyright e o aviso de permissão nas cópias ou partes substanciais do software. A licença não transfere automaticamente a titularidade ao mantenedor.

A documentação mantida descreve essa política. Não há uma licença CC BY ou Apache separada instituída por este documento; consulte os avisos aplicáveis a cada material, especialmente no acervo histórico.

## Aplicativos gerados e materiais MIT-0

Quem gera um aplicativo com CREXE pode usar, modificar, vender e distribuir seu programa sob a licença que escolher, inclusive fechada/proprietária. Pela licença dos materiais fornecidos pelo CREXE, não há obrigação de publicar os fontes, pagar royalties nem creditar o CREXE. A concessão [MIT No Attribution (MIT-0)](https://spdx.org/licenses/MIT-0.html) permite sublicenciar e vender os materiais designados sem exigir atribuição ou retenção de seu aviso.

Essa concessão cobre as partes de autoria do CREXE dos materiais abaixo, suas cópias e modificações, conforme `LICENSE-GENERATED`. Ela não concede direitos sobre toda a saída do usuário ou do modelo nem determina sua proteção autoral. As implementações Rust que produzem esses materiais usam MIT; a concessão MIT-0 é específica para os materiais emitidos e exemplos designados.

| Conteúdo | Origem atual |
| --- | --- |
| Projeto `.csproj` inserido no app | [src/profiles.rs](../src/profiles.rs), funções de criação/preparação do projeto .NET. |
| Makefile do perfil GTK | [src/profiles/native.rs](../src/profiles/native.rs). |
| Scripts de build/run/test/publish e README exportados | [src/package.rs](../src/package.rs). |
| Estruturas e textos dos manifestos de projeto e geração | [src/project.rs](../src/project.rs) e [src/history.rs](../src/history.rs). O manifesto de geração fica na revisão local; não integra o ZIP de fontes. |
| Instruções e contratos incorporados à saída e às receitas geradas | [src/reader.rs](../src/reader.rs), perfis, [src/orchestration.rs](../src/orchestration.rs) e [src/generator.rs](../src/generator.rs), somente seus materiais emitidos. |
| Receitas oficiais e documentos de exemplo dos pacotes | Diretório `examples/` e textos de exemplo em [scripts/package-engine.py](../scripts/package-engine.py). |

O projeto e seu ZIP recebem `CREXE-OUTPUT-LICENSE.txt`, uma cópia de `LICENSE-GENERATED`. O aviso explica a concessão dos materiais do CREXE e pode ser removido conforme MIT-0. O exportador preserva um `LICENSE` fornecido pelo usuário; não cria uma licença para o aplicativo inteiro. Exportar uma revisão antiga também inclui o aviso atual sem gerar novamente o programa ou alterar sua revisão em cache.

Distribuir a própria engine ou incorporar partes de sua implementação exige preservar os avisos MIT aplicáveis. A permissão de remover o aviso dos materiais MIT-0 não se estende à engine nem dispensa avisos exigidos por terceiros. Os pacotes da engine incluem `LICENSE` e `LICENSE-GENERATED` para distinguir esses escopos.

## Direitos de terceiros

Código, bibliotecas, assets e receitas de terceiros mantêm suas próprias condições, assim como os termos do modelo/provider utilizado. O usuário escolhe a licença do aplicativo respeitando esses termos; o CREXE não pode liberar direitos de terceiros. O exportador não audita automaticamente essas licenças.

Por exemplo, OpenTK e suas dependências no perfil OpenGL .NET, GTK/libepoxy, frameworks Apple e runtimes do compilador seguem suas próprias licenças e condições de distribuição. Compilar ou exportar não transforma esses componentes em código MIT-0 do CREXE.

Os pacotes nativos da engine incluem `THIRD-PARTY-NOTICES.txt` e `third-party-licenses/`, com textos, origens, versões e hashes. A coleta usa os metadados Cargo filtrados para o host e inclui conservadoramente dependências de build e desenvolvimento. Textos suplementares fixados por versão em `scripts/third_party_licenses/` e seu mapa associado preservam avisos ausentes dos pacotes de origem. Esse inventário documental não afirma que todos os componentes listados estejam incorporados ao binário nem certifica conformidade jurídica completa.

## Contribuições

Contribuições à engine são feitas sob MIT. Contribuições aos materiais de saída e exemplos designados em `LICENSE-GENERATED` também são feitas sob MIT-0, além da licença do repositório, salvo acordo explícito em contrário. Essa autorização adicional é necessária para preservar a liberdade de licenciamento dos aplicativos gerados; a MIT da engine, sozinha, não remove sua exigência de avisos.

A licença não transfere automaticamente a titularidade das contribuições ao mantenedor. O projeto não exige cessão de direitos nem CLA como condição geral de contribuição. Quem contribui deve ter os direitos necessários para oferecer o material nesses termos, identificar conteúdo de terceiros e preservar os avisos aplicáveis. Consulte o [guia de contribuição](../CONTRIBUTING.md).

## Histórico e versões anteriores

A engine utilizava GNU GPL v3 antes desta decisão. A política atual privilegia a adoção, os ports e a integração da implementação sob MIT, mantendo MIT-0 para os materiais de saída. A mudança não reescreve commits, tags, pacotes ou licenças de versões anteriores e não revoga permissões já concedidas. Consulte a licença que acompanha cada versão; este documento não promete relicenciar contribuições de terceiros retroativamente.

Os RFCs, handoffs originais e arquivos de `docs/archive` permanecem como registros históricos. Propostas antigas sobre produtos ou licenças não substituem `LICENSE`, `LICENSE-GENERATED` e esta política atual. As referências à GPL no diagnóstico do [plano da v1](PLANO_V1.md) registram o estado anterior. A [biblioteca editorial](publicacoes/README.md) distingue os originais das publicações mantidas.

## Formato, identidade e produtos futuros

A interoperabilidade com `.crexe` e as implementações independentes do formato são bem-vindas. Usar o formato, portar a engine ou escolher uma licença para um aplicativo não autoriza apresentar um produto como oficial do CREXE. A licença de software não transfere direitos sobre marcas; referências descritivas verdadeiras, como “suporta arquivos .crexe”, devem ser distinguidas da identidade oficial. Este documento não afirma que a marca esteja registrada nem institui uma política marcária separada.

`crexe.org` reúne o projeto aberto, downloads, documentação, RFCs e exemplos. Produtos e serviços comerciais futuros podem ser organizados em outros repositórios e sites, sem serem apresentados como funcionalidades disponíveis nesta versão. Essa organização não muda as licenças dos componentes utilizados.
