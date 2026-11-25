# Licenciamento e organização do ecossistema CREXE

Revisão de 24/11/2026 do plano de licenciamento de 14/10/2026 e da implementação atual. A decisão mantida é **GNU GPL v3 para a engine**. Este registro distingue o que já existe das formalizações propostas; não constitui uma nova licença, política de marcas ou parecer jurídico.

## Decisão e situação atual

A separação entre projeto aberto e produtos comerciais é coerente com a direção do CREXE. Não é necessário trocar a licença do Core para organizar esses dois espaços.

| Item | Situação no repositório |
| --- | --- |
| Engine/CLI | Texto GNU GPL v3 em [LICENSE](../LICENSE), referenciado por `license-file` em [Cargo.toml](../Cargo.toml). |
| Versões futuras da GPL | O plano propõe `GPL-3.0-or-later`; o README atual não acrescenta essa permissão. A variante precisa ser consolidada explicitamente antes de alterar metadados e avisos. |
| Templates e exemplos | Ainda não há concessão MIT/Apache específica que implemente a matriz proposta. |
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

Usar uma ferramenta GPL não torna automaticamente sua saída GPL. Código incorporado da própria ferramenta, templates, dependências e assets podem impor condições específicas. Portanto, exportar fontes não equivale a conceder direitos irrestritos sobre todo o conteúdo. Referência: [FAQ GNU sobre saída de programas](https://www.gnu.org/licenses/gpl-faq.html#GPLOutput).

Comunicação por CLI, pipes ou IPC costuma indicar programas separados, mas não é garantia jurídica: o acoplamento e os dados trocados também importam. Uma integração proprietária deve ser analisada concretamente. Referência: [FAQ GNU sobre agregação](https://www.gnu.org/licenses/gpl-faq.html#MereAggregation).

A GPL permite atividade comercial. Seu mecanismo de fornecimento de fontes está ligado às condições de distribuição; ela não contém a obrigação de interação por rede da AGPL. Isso não dispensa analisar a distribuição dos componentes de um produto comercial. Referências: [GNU GPL v3, seções 2 e 6](https://www.gnu.org/licenses/gpl-3.0.html), [FAQ GNU sobre versões modificadas](https://www.gnu.org/licenses/gpl-faq.html#UnreleasedMods).

### Pendência concreta: templates exportados

O plano prevê templates permissivos e inventário de licenças, mas essa separação ainda precisa ser implementada:

| Conteúdo | Origem atual |
| --- | --- |
| Projeto `.csproj` inserido no app | [src/profiles.rs](../src/profiles.rs), funções de criação/preparação do projeto .NET. |
| Makefile do perfil GTK | [src/profiles/native.rs](../src/profiles/native.rs). |
| Scripts de build/run/test/publish e README exportados | [src/package.rs](../src/package.rs). |
| Manifestos de projeto e geração | [src/project.rs](../src/project.rs) e [src/history.rs](../src/history.rs); ainda sem inventário de licenças de templates e avisos de terceiros. |

Antes de prometer uso permissivo dos templates, inventariar esses trechos, confirmar titularidade, escolher a concessão aplicável, separar seus arquivos/avisos e incluir as licenças necessárias na exportação. O pacote atual não cria automaticamente uma licença validada para o aplicativo. Isso não autoriza concluir que toda saída é GPL; impede anunciar que a liberação permissiva do plano já foi aplicada.

As receitas oficiais também precisam de uma concessão explícita para serem anunciadas como MIT. O [empacotador da engine](../scripts/package-engine.py) atualmente reúne exemplos e a licença da engine, sem licença MIT separada para as receitas.

## Formato, identidade e marca

É adequado distinguir interoperabilidade com `.crexe` de apresentação como produto oficial. A GPL v3 admite ressalvas de direitos de marca e exigências razoáveis de identificação de modificações, sem permitir restrições adicionais incompatíveis ao código. Referência: [GNU GPL v3, seção 7](https://www.gnu.org/licenses/gpl-3.0.html#section7).

Uma futura política deve preservar referências descritivas verdadeiras, como “suporta arquivos .crexe”, e nomes técnicos do protocolo. `CREXE-PROJECT.json`, `CrexeApp.csproj` ou uma indicação da ferramenta usada não devem ser confundidos automaticamente com uma marca principal de produto.

Não se deve afirmar uma proibição universal de nomes, cores, domínios ou usos comerciais apenas por publicar uma política. A extensão e os limites da proteção dependem dos direitos aplicáveis. No Brasil, registro, território e produtos/serviços protegidos são relevantes. O status da marca CREXE não foi verificado nesta revisão. Referência: [INPI — perguntas frequentes sobre marcas](https://www.gov.br/inpi/pt-br/acesso-a-informacao/perguntas-frequentes/marcas).

## Licenças adicionais e contribuições

Apache 2.0 para futuros SDKs/protocolos e CC BY 4.0 para documentação são opções compatíveis com a organização proposta, desde que aplicadas a materiais definidos e por quem tenha os direitos necessários. CC BY não deve substituir uma licença de software. Referências: [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0.html), [Creative Commons — software e documentação](https://creativecommons.org/faq/#can-i-apply-a-creative-commons-license-to-software).

Um CLA não é requisito geral para manter um projeto GPL. Para licenciamento comercial alternativo, é preciso controlar direitos suficientes sobre todas as partes envolvidas; um CLA ou uma cessão pode ser um instrumento para isso. Não há CLA vigente instituído por este registro. Referência: [Open Source Initiative — acordos de contribuição](https://opensource.org/faq).

Antes de transformar o plano em políticas vinculantes, consolidar a variante GPL, a titularidade, as concessões de templates/documentação, os direitos de contribuição e as regras de marca. A formalização contratual e marcária deve receber revisão jurídica. Enquanto isso, a apresentação pública descreve a licença existente e as capacidades efetivamente implementadas.
