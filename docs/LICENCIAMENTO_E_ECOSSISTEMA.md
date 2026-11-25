# Licenciamento e organização do ecossistema CREXE

Revisão de 25/11/2026. A engine permanece sob **GNU GPL v3**. **O usuário escolhe a licença do aplicativo gerado, inclusive proprietária.** Os materiais de saída fornecidos pelo CREXE e as receitas oficiais recebem a concessão **MIT-0** em [LICENSE-GENERATED](../LICENSE-GENERATED). Este documento explica essa separação e as propostas futuras; não substitui os textos das licenças nem constitui parecer jurídico.

## Decisão e situação atual

A separação entre projeto aberto e produtos comerciais é coerente com a direção do CREXE. Não é necessário trocar a licença do Core para organizar esses dois espaços.

| Item | Situação no repositório |
| --- | --- |
| Engine/CLI | Texto GNU GPL v3 em [LICENSE](../LICENSE), referenciado por `license-file` em [Cargo.toml](../Cargo.toml). |
| Versões futuras da GPL | O plano propõe `GPL-3.0-or-later`; o README atual não acrescenta essa permissão. A variante precisa ser consolidada explicitamente antes de alterar metadados e avisos. |
| Materiais de saída e exemplos oficiais | Concessão MIT-0 explícita em [LICENSE-GENERATED](../LICENSE-GENERATED), com escopo delimitado. O aplicativo não recebe uma licença automaticamente. |
| Documentação e código do site | Esta revisão não aplica CC BY, MIT ou Apache a arquivos existentes. |
| Protocolos, SDKs externos, `.crexp` e certificação | Direções futuras, não funcionalidades ou políticas entregues nesta versão. |
| Marca e titularidade | O plano apresenta Bestows Solutions Ltda. como titular proposta. Esta revisão não comprova cessão de direitos nem registro de marca. |

“GPL v3” e “GPL v3 ou posterior” não são identificadores equivalentes. A opção de usar versões posteriores depende do aviso aplicável ao programa; o exemplo de aplicação incluído ao final do texto da licença não substitui esse aviso. Por isso, mantêm-se `LICENSE`, `Cargo.toml` e a formulação atual do README. Referência: [GNU GPL v3, seção 14](https://www.gnu.org/licenses/gpl-3.0.html#section14).

## Estrutura dos sites

| Espaço | Conteúdo e papel |
| --- | --- |
| `crexe.org` | Projeto aberto: apresentação, engine/CLI, downloads, formatos implementados, RFCs, documentação, exemplos, contribuição e segurança. Governança e políticas formais entram quando estiverem definidas. |
| `crexe.bestows.solutions` | Destino planejado para os produtos e serviços comerciais da Bestows. Studio, Education, Cloud, Gallery, Enterprise, suporte e certificação permanecem propostas até anúncio e entrega próprios. |

A landing pública conserva o foco em compreender, experimentar e baixar o projeto. Ela não deve anunciar produtos futuros como disponíveis, sugerir uma fundação independente ou tratar políticas em preparação como contratos vigentes. A divisão de domínios é editorial e institucional: não determina, sozinha, a licença de cada implementação.

Não é necessário criar todos os repositórios previstos antes de existirem seus componentes. Quando houver múltiplas licenças, os limites entre arquivos, dependências e conteúdo exportado precisam ser claros, seja em pastas do mesmo repositório ou em repositórios separados.

## Engine, programas gerados e produtos separados

Quem gera um aplicativo com CREXE pode usar, modificar, vender e distribuir seu programa sob a licença que escolher, inclusive fechada/proprietária. O CREXE não exige publicar os fontes, pagar royalties nem incluir crédito ao CREXE. Usar uma ferramenta GPL não torna automaticamente sua saída GPL. Referência: [GNU GPL v3, seção 2](https://www.gnu.org/licenses/gpl-3.0.html#section2).

Para que os templates da própria engine também permitam essa liberdade, seus materiais incorporados à saída são oferecidos sob [MIT No Attribution (MIT-0)](https://spdx.org/licenses/MIT-0.html), no escopo de [LICENSE-GENERATED](../LICENSE-GENERATED). Essa licença permite sublicenciar e vender, sem exigir atribuição ou retenção do aviso. A concessão também alcança esses materiais em aplicativos já gerados; ela não troca a licença da engine.

Código, bibliotecas, assets e receitas de terceiros mantêm suas próprias condições, assim como os termos do modelo/provider utilizado. O usuário escolhe a licença do aplicativo respeitando esses termos. O CREXE não reivindica titularidade sobre a saída nem determina sua proteção autoral, e não pode liberar direitos de terceiros. O exportador não audita automaticamente essas licenças.

Comunicação por CLI, pipes ou IPC costuma indicar programas separados, mas não é garantia jurídica: o acoplamento e os dados trocados também importam. Uma integração proprietária deve ser analisada concretamente. Referência: [FAQ GNU sobre agregação](https://www.gnu.org/licenses/gpl-faq.html#MereAggregation).

A GPL permite atividade comercial. Seu mecanismo de fornecimento de fontes está ligado às condições de distribuição; ela não contém a obrigação de interação por rede da AGPL. Isso não dispensa analisar a distribuição dos componentes de um produto comercial. Referências: [GNU GPL v3, seções 2 e 6](https://www.gnu.org/licenses/gpl-3.0.html), [FAQ GNU sobre versões modificadas](https://www.gnu.org/licenses/gpl-faq.html#UnreleasedMods).

### Materiais de saída licenciados sob MIT-0

O escopo da concessão inclui as partes de autoria do CREXE dos materiais abaixo, suas cópias e modificações. As implementações Rust que os produzem continuam GPLv3:

| Conteúdo | Origem atual |
| --- | --- |
| Projeto `.csproj` inserido no app | [src/profiles.rs](../src/profiles.rs), funções de criação/preparação do projeto .NET. |
| Makefile do perfil GTK | [src/profiles/native.rs](../src/profiles/native.rs). |
| Scripts de build/run/test/publish e README exportados | [src/package.rs](../src/package.rs). |
| Estruturas e textos dos manifestos de projeto e geração | [src/project.rs](../src/project.rs) e [src/history.rs](../src/history.rs). O manifesto de geração fica na revisão local; não integra o ZIP de fontes. |
| Instruções e contratos incorporados à saída e às receitas geradas | [src/reader.rs](../src/reader.rs), perfis, [src/orchestration.rs](../src/orchestration.rs) e [src/generator.rs](../src/generator.rs), somente seus materiais emitidos. |
| Receitas oficiais e documentos de exemplo dos pacotes | Diretório `examples/` e textos de exemplo em [scripts/package-engine.py](../scripts/package-engine.py). |

O projeto e seu ZIP recebem `CREXE-OUTPUT-LICENSE.txt`, uma cópia de `LICENSE-GENERATED`. O aviso identifica a concessão dos materiais do CREXE e pode ser removido conforme MIT-0. O exportador preserva um `LICENSE` fornecido pelo usuário; não cria uma licença para o aplicativo inteiro. Exportar uma revisão antiga também inclui o aviso atual sem gerar novamente o programa ou alterar sua revisão em cache.

Os pacotes da engine incluem tanto `LICENSE` (GPLv3 da engine) quanto `LICENSE-GENERATED` (MIT-0 dos materiais designados e exemplos oficiais). Esta concessão não abrange automaticamente toda a documentação, os testes de conformidade ou implementações independentes do formato.

As dependências continuam fora dessa concessão. Por exemplo, OpenTK e suas dependências no perfil OpenGL .NET, GTK/libepoxy, frameworks Apple e runtimes do compilador seguem suas próprias licenças e condições de distribuição. Compilar ou exportar não transforma esses componentes em código MIT-0 do CREXE.

## Formato, identidade e marca

É adequado distinguir interoperabilidade com `.crexe` de apresentação como produto oficial. A GPL v3 admite ressalvas de direitos de marca e exigências razoáveis de identificação de modificações, sem permitir restrições adicionais incompatíveis ao código. Referência: [GNU GPL v3, seção 7](https://www.gnu.org/licenses/gpl-3.0.html#section7).

Uma futura política deve preservar referências descritivas verdadeiras, como “suporta arquivos .crexe”, e nomes técnicos do protocolo. `CREXE-PROJECT.json`, `CrexeApp.csproj` ou uma indicação da ferramenta usada não devem ser confundidos automaticamente com uma marca principal de produto.

Não se deve afirmar uma proibição universal de nomes, cores, domínios ou usos comerciais apenas por publicar uma política. A extensão e os limites da proteção dependem dos direitos aplicáveis. No Brasil, registro, território e produtos/serviços protegidos são relevantes. O status da marca CREXE não foi verificado nesta revisão. Referência: [INPI — perguntas frequentes sobre marcas](https://www.gov.br/inpi/pt-br/acesso-a-informacao/perguntas-frequentes/marcas).

## Licenças adicionais e contribuições

Apache 2.0 para futuros SDKs/protocolos e CC BY 4.0 para documentação são opções compatíveis com a organização proposta, desde que aplicadas a materiais definidos e por quem tenha os direitos necessários. CC BY não deve substituir uma licença de software. Referências: [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0.html), [Creative Commons — software e documentação](https://creativecommons.org/faq/#can-i-apply-a-creative-commons-license-to-software).

Um CLA não é requisito geral para manter um projeto GPL. Para licenciamento comercial alternativo, é preciso controlar direitos suficientes sobre todas as partes envolvidas; um CLA ou uma cessão pode ser um instrumento para isso. Não há CLA vigente instituído por este registro. Referência: [Open Source Initiative — acordos de contribuição](https://opensource.org/faq).

O escopo MIT-0 dos materiais de saída e exemplos está explícito em `LICENSE-GENERATED`; novas contribuições a esses materiais devem respeitar essa concessão. Permanecem separadas as decisões futuras sobre variante GPL, titularidade societária, documentação, direitos de contribuição e regras de marca. A formalização contratual e marcária deve receber revisão jurídica. A apresentação pública descreve as licenças existentes e as capacidades efetivamente implementadas.
