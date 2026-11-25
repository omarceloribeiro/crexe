# Modelos locais e remotos no CREXE

> Guia técnico · 24/11/2026 · Estado da v1 em desenvolvimento. Integrações com modelos do sistema operacional são pesquisa; os adaptadores atuais são Ollama, OpenAI e DeepSeek. Requisitos externos podem mudar.

Executar a geração no próprio computador pode reduzir o envio de dados e permitir trabalhar sem conexão depois da preparação. Usar um serviço remoto pode oferecer modelos e capacidade computacional que o computador não comporta. A escolha depende da tarefa, dos dados e dos recursos disponíveis.

Nenhuma dessas escolhas torna o código gerado automaticamente seguro ou correto. Um modelo local também erra; uma resposta remota bem estruturada também precisa ser compilada e verificada. O [guia de segurança](guia-seguranca-e-confianca.md) descreve os limites da execução atual.

## Opções implementadas

| Adaptador | Uso atual | Condições importantes |
|---|---|---|
| Ollama | Padrão local, sem chave de API | Serviço e modelo precisam estar disponíveis; confira o endpoint configurado |
| OpenAI | Geração por API remota | Requer configuração local, credencial válida e disponibilidade do serviço |
| DeepSeek | Geração por API remota | Requer configuração local, credencial válida e disponibilidade do serviço |

Não existe fallback automático para um provider pago. O arquivo `.crexe` não escolhe endpoint ou credenciais. Essas decisões pertencem ao usuário, na [configuração da engine](../CONFIGURACAO.md). Um endpoint Ollama pode apontar para outra máquina; o nome do adaptador, sozinho, não comprova que os dados permanecem no dispositivo.

A tela de configuração permite escolher o modelo, testar a conexão e limitar tentativas. Consultar modelos não gera um aplicativo nem garante que todos os modelos listados consigam cumprir o contrato de saída do CREXE. As chaves salvas pela interface ficam no cofre nativo do usuário; `.env` só é carregado quando indicado explicitamente.

## Preparar uma primeira geração

Com a engine compilada, execute na raiz do repositório, no Windows:

```powershell
.\target\release\crexe.exe configure
.\target\release\crexe.exe doctor --check
.\target\release\crexe.exe inspect examples\plaintext\calculadora.crexe
```

Configure o provider e confira o modelo antes de gerar. `doctor --check` verifica os SDKs locais sem chamar o provider; `inspect` mostra a interpretação da receita e seus requisitos. São verificações diferentes do teste de conexão da tela.

Depois de revisar a receita e aceitar a execução com as permissões do usuário, a geração e abertura usam:

```powershell
.\target\release\crexe.exe examples\plaintext\calculadora.crexe
```

Uma geração malsucedida não significa necessariamente falta de capacidade do modelo. Pode haver SDK ausente, contrato de resposta incompatível, limite de contexto ou erro de compilação. A engine permite reparos de build/teste dentro do orçamento local; falhas de autenticação, saldo, conexão ou resposta inválida encerram a tentativa. Aumentar esse orçamento indiscriminadamente não resolve a causa.

## O que precisa estar offline

“Funcionar offline” deve ser testado por fase, com o ambiente preparado e a conexão realmente indisponível.

| Fase | O que verificar |
|---|---|
| Instalação | Engine, serviço de inferência, pesos, SDKs e bibliotecas já estão disponíveis? |
| Geração | O endpoint é local? O modelo consegue carregar e responder sem serviços externos? |
| Compilação | Todas as dependências necessárias estão presentes, incluindo pacotes do projeto? |
| Abertura pelo cache | A revisão aprovada e os recursos de execução continuam disponíveis? |
| Uso do aplicativo | O comportamento solicitado depende de APIs, arquivos remotos ou autenticação? |

O CREXE não instala automaticamente os SDKs. Um build pode precisar restaurar dependências, conforme o [perfil escolhido](../PERFIS_V1.md). Reabrir uma revisão válida pelo cache evita uma nova geração, mas não elimina necessidades próprias do aplicativo. O ZIP de fontes permite recompilar sem CREXE, desde que as ferramentas e dependências necessárias estejam presentes.

Também é preciso separar “sem internet durante o uso” de “sem acesso à rede imposto pelo sistema”. A v1 não oferece contenção de rede para o código gerado.

## Modelos oferecidos pelo sistema operacional

Já existem APIs públicas para modelos locais em plataformas relevantes. A afirmação de que os modelos internos são sempre inacessíveis a aplicativos ficou desatualizada. A disponibilidade depende do dispositivo, da versão do sistema, das configurações e das condições da API.

| Plataforma | Acesso documentado | Limites a considerar |
|---|---|---|
| Apple | Foundation Models oferece acesso ao modelo no dispositivo usado pelo Apple Intelligence | Verificar elegibilidade, ativação e disponibilidade do modelo antes de criar a sessão |
| Android | ML Kit GenAI acessa Gemini Nano por meio do serviço AICore | Suporte varia por dispositivo e API; há limites de uso e execução em primeiro plano |
| Windows | Windows AI APIs expõem inferência local, documentada para Phi Silica e sua evolução | Hardware, SDK, canal de distribuição e requisitos de acesso variam; consultar o estado do modelo |

A Apple documenta geração estruturada e verificações de disponibilidade. A integração precisa consultar essa disponibilidade em tempo de execução e informar quando o modelo não estiver pronto. A própria documentação inclui geração de código entre as tarefas não recomendadas para esse modelo. [Apple: geração com Foundation Models](https://developer.apple.com/documentation/foundationmodels/generating-content-and-performing-tasks-with-foundation-models).

O Google oferece APIs especializadas e geração por prompt, mas cada combinação de dispositivo e modelo tem suas condições. A documentação informa quotas e restrições de execução em segundo plano. [Google: ML Kit GenAI](https://developers.google.com/ml-kit/genai/).

A documentação da Microsoft já anuncia a transição de Phi Silica para Aion Instruct. Phi Silica exige autorização de acesso por Limited Access Feature; as condições mudam na transição anunciada. Nomes de modelos e requisitos precisam ser verificados durante uma integração; não são uma base estável para prometer suporte universal. [Microsoft: Phi Silica e Windows AI APIs](https://learn.microsoft.com/windows/ai/apis/phi-silica).

Essas APIs **não estão integradas ao CREXE atual**. Disponibilidade para resumir ou gerar texto não comprova qualidade para projetos completos. Em dispositivos móveis, ter inferência local tampouco implica permissão para compilar e executar livremente programas nativos. Isso exige estudo específico da plataforma.

## Repetição, cache e reprodução

Fixar modelo, parâmetros e seed ajuda a reduzir variações quando o backend permite, mas não garante saídas idênticas entre versões, hardware ou implementações. A documentação do PyTorch, por exemplo, distingue o controle de aleatoriedade da reprodução entre plataformas e entre CPU e GPU. Esse é um exemplo de limite técnico, não uma declaração de que o CREXE utiliza PyTorch. [PyTorch: Reproducibility](https://docs.pytorch.org/docs/2.15/notes/randomness.html).

Há três objetivos diferentes: obter comportamento equivalente ao regenerar; reconstruir os mesmos bytes a partir de fontes e ferramentas controlados; reutilizar um artefato já aprovado. O cache do CREXE atende ao terceiro. Não demonstra os dois primeiros.

## Como avaliar um modelo

Compare modelos com uma coleção fixa de tarefas pequenas e critérios independentes: requisitos atendidos, build aprovado, testes funcionais e uso real da interface. Registre versões, contexto, parâmetros, máquina, memória, duração e quantidade de reparos. Inclua fracassos, respostas truncadas e alterações incrementais, além da primeira geração.

Meça separadamente preparação, geração e compilação. Um download inicial grande ou um modelo já carregado mudam a experiência. Relatos em uma máquina são evidência localizada, não requisitos universais nem comparação controlada.

Uma futura integração com modelos do sistema deve preservar a escolha local de provider, expor indisponibilidade sem substituição silenciosa e validar contratos, privacidade e limites de execução. Esses critérios permitem investigar novos backends sem apresentar pesquisa como funcionalidade entregue.
