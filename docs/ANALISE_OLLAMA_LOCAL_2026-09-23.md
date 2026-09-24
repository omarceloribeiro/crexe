# Análise do Ollama local — 23/09/2026

## Objetivo e conclusão

Este documento reúne os testes realizados neste chat para servir de contexto a outro chat Codex sobre a capacidade e as limitações do Ollama disponível no host Windows. Não é um benchmark controlado nem uma validação de código gerado.

**O `gemma4:26b` conseguiu carregar e gerar respostas depois que o usuário reduziu o uso prévio de RAM da máquina para aproximadamente 9 GB.** Esse patamar foi informado pelo usuário. Durante a operação bem-sucedida, ele observou que o uso total de RAM não ultrapassou aproximadamente 25 GB.

Antes disso, ocorreram falhas no carregamento com `CUDA error: out of memory`, e o monitoramento chegou a registrar 100% de uso da RAM. Depois de carregado, o modelo respondeu a pedidos sucessivos, com e sem streaming, sem repetir os erros. A evidência é compatível com um pico de memória durante o carregamento maior que o consumo para manter o modelo em operação.

Os 9 GB de uso prévio são uma condição observada de sucesso, não um limite garantido. Não foi isolada a alocação exata responsável pela falha; RAM, memória utilizada pelo CUDA e comportamento de recuperação do Ollama podem estar envolvidos.

## Ambiente disponível

| Item | Observação |
|---|---|
| Máquina | Lenovo Legion 5, conforme informado pelo usuário |
| Sistema | Windows |
| CPU | Intel Core i7-12700H |
| RAM utilizável | 31,73 GiB, aproximadamente 32 GB instalados |
| GPU dedicada | NVIDIA GeForce RTX 3060 Laptop GPU, 6.144 MiB de VRAM |
| GPU integrada | Intel Iris Xe Graphics |
| Ollama | Versão 0.34.2 |
| Executável | `C:\Users\marce\AppData\Local\Programs\Ollama\ollama.exe` |
| Endpoint local | `http://127.0.0.1:11434` |
| Interface de escuta | `127.0.0.1:11434`, confirmada no host |
| Plano de energia Windows | `Legion Performance Mode` |

O comando `ollama` não foi encontrado no PATH do sandbox, mas foi encontrado e executado normalmente no host, fora do sandbox. Os testes HTTP foram executados nesse contexto do host. Uma falha de acesso pelo sandbox não demonstra indisponibilidade do serviço no host.

O endpoint está restrito ao loopback da máquina. Não foi configurado acesso remoto pela rede.

A configuração térmica exata do atalho Fn+Q não pôde ser confirmada pela interface Lenovo. Também não foi possível confirmar diretamente o modo gráfico configurado na Lenovo: a leitura retornou acesso negado. As duas GPUs apareceram habilitadas no Windows; isso, isoladamente, não confirma a configuração do MUX/modo híbrido.

Na consulta inicial, o disco C: tinha 29,30 GiB livres de 474,72 GiB, e o D: tinha 98,42 GiB livres de 931,51 GiB. Esses valores são uma fotografia daquele momento, não uma consulta atual.

## Modelos instalados

Tamanhos abaixo conforme exibidos por `ollama list`; não equivalem ao consumo total de memória em execução.

| Modelo | ID | Tamanho em disco |
|---|---|---:|
| `gemma4:12b` | `4eb23ef187e2` | 7,6 GB |
| `gemma4:26b` | `5571076f3d70` | 17 GB |
| `embeddinggemma:latest` | `85462619ee72` | 621 MB |
| `qwen3-embedding:0.6b` | `ac6da0dfba84` | 639 MB |

## Testes de embeddings

Endpoint: `POST /api/embed`. Entrada em ambos os testes: `hello world`.

| Modelo | Resultado | Dimensões | Duração total da API | Tokens de entrada reportados |
|---|---|---:|---:|---:|
| `embeddinggemma:latest` | Sucesso | 768 | 16,36 s | 4 |
| `qwen3-embedding:0.6b` | Sucesso | 1.024 | 1,34 s | 3 |

Primeiros dez valores de `embeddinggemma:latest`:

```json
[-0.21401396, 0.026527284, 0.06660981, -0.016701901, 0.007585998, 0.010813499, -0.014047482, -0.002663262, -0.011941605, -0.044114355]
```

Primeiros dez valores de `qwen3-embedding:0.6b`:

```json
[-0.015692139, 0.014777487, -0.011916919, -0.07097554, 0.0015077019, -0.021623502, -0.015204296, 0.014881436, -0.103949234, -0.0053205383]
```

Os testes confirmam geração de vetores, não qualidade semântica, capacidade de recuperação ou desempenho em lote. Não foi controlado o estado de carregamento de cada modelo; portanto, os tempos não sustentam uma comparação direta de velocidade entre eles.

## Testes do `gemma4:12b`

Endpoint: `POST /api/generate`, com `stream: false`. Não foram enviados ajustes de contexto, distribuição GPU/CPU ou thinking.

| Pedido | Duração total | Observações |
|---|---:|---|
| Hello World em C, primeiro teste | 38,74 s | Resposta concluída; 178 tokens gerados reportados |
| Hello World em C, com monitoramento NVIDIA | 28,71 s | GPU utilizada; 27 amostras |
| Hello World em Java, com monitoramento das duas GPUs | 20,47 s | NVIDIA utilizada; Intel em 0% nas 13 amostras |

No teste monitorado em C:

- Uso NVIDIA inicial: 0%; VRAM inicial: 746 MiB.
- Pico de utilização NVIDIA: 41%; média: 20,9%.
- Pico observado de VRAM: 5.086 MiB.

No teste monitorado em Java:

- Pico de utilização NVIDIA: 40%; média: 28,9%.
- Pico observado de VRAM: 5.086 MiB.
- Intel Iris Xe: pico e média de 0% nas amostras coletadas.
- Contadores atribuídos aos processos Ollama na Intel também registraram 0%.

`ollama ps` mostrou tamanho carregado de 9,0 GB, contexto de 16.384 tokens e distribuição **55% CPU / 45% GPU**. Essa distribuição descreve onde o modelo foi carregado; não significa utilização instantânea da CPU ou GPU.

Os logs registraram cerca de 9 tokens por segundo durante a geração. No primeiro Hello World em C, aproximadamente 20,9 s foram gastos no processamento do prompt e geração, dentro dos 38,74 s totais.

O monitoramento NVIDIA utilizou `nvidia-smi`; o da Intel utilizou contadores de mecanismos de GPU do Windows, associados ao adaptador via identificador DirectX. As métricas dos dois mecanismos de medição não são perfeitamente equivalentes. Amostras periódicas podem perder picos curtos, e 0% amostrado não prova ausência absoluta de qualquer atividade entre amostras.

## Falhas de carregamento do `gemma4:26b`

Pedidos de Hello World em C e Java falharam antes da geração. As falhas ocorreram tanto com `stream: false` quanto com `stream: true`.

Erro retornado pela API:

```text
HTTP 500
llama-server startup failed before projector CPU offload retry:
llama-server reported out-of-memory during startup:
CUDA error: out of memory
CUDA error; error stopping failed process: TerminateProcess: Access is denied.
```

Em uma tentativa monitorada:

- RAM atingiu 100%.
- NVIDIA atingiu 25% de utilização; pico amostrado de VRAM de 2.710 MiB.
- Intel ficou em 0% nas amostras.
- Ao final, `ollama ps` respondeu sem modelos carregados.

Os logs de uma falha mostraram:

- 11,6 GiB de RAM livre antes do carregamento.
- Buffer do modelo em `CUDA_Host` de 14.712,02 MiB, aproximadamente 14,4 GiB, além de outras alocações.
- Buffer do modelo em CUDA0 de 1.866,95 MiB.
- Intenção de tentar novamente com o componente de visão/projetor na CPU, impedida pelo erro ao encerrar o processo que falhou.

O baixo pico de VRAM observado não demonstra que havia memória suficiente para concluir a carga: o carregamento falhou, a amostragem não captura necessariamente todos os picos e a RAM também estava sob pressão.

O agente não enviou comandos para encerrar o Ollama. A tentativa de encerramento mencionada no erro foi interna ao próprio Ollama.

## Redução de consumo e carregamento bem-sucedido

O Firefox era o maior consumidor identificado, com aproximadamente 6.194,5 MiB somados entre 25 processos. A pedido do usuário, esses processos foram encerrados. A RAM livre subiu de 13,96 para 20,15 GiB e o uso caiu de 56% para 36,5%.

Uma nova tentativa do 26B ainda falhou, com RAM atingindo 100% e erro CUDA. Depois da falha, uma consulta mostrou 10,41 GiB usados e 21,32 GiB livres. Isso reforça que o consumo em repouso e o pico durante a carga são diferentes.

O usuário fechou mais aplicações e informou posteriormente que **o modelo carregou quando o uso prévio de RAM caiu para cerca de 9 GB**. Depois de carregado, ele observou consumo total de RAM de até aproximadamente 25 GB.

| Pedido após liberar mais memória | Streaming | Resultado | Tempo total | Carregamento |
|---|---|---|---:|---:|
| Hello World em Java | Habilitado | Sucesso | 41,19 s de tempo observado no cliente | 24,53 s reportados pela API |
| Mesmo Hello World em Java | Desabilitado | Sucesso | 8,07 s reportados pela API | Praticamente zero |
| CRUD de produtos em FastAPI | Desabilitado | Resposta concluída | 76,57 s | Praticamente zero |
| CRUD em Blazor WebAssembly + API ASP.NET Core | Desabilitado | Resposta concluída, com erros no código | 133,69 s | Praticamente zero |

No teste com streaming, os fragmentos foram acumulados no terminal e a resposta completa foi apresentada somente ao final. Não houve execução do código retornado.

A comparação de 41,19 s com 8,07 s não isola o efeito do streaming: o primeiro pedido precisou carregar o modelo, enquanto o segundo o reutilizou. Streaming, sozinho, não resolveu as falhas anteriores.

Não foi medida a distribuição CPU/GPU do 26B nas execuções bem-sucedidas. Não transferir para ele a divisão 55%/45% observada no 12B.

## Qualidade das respostas de programação

### Hello World

O 12B retornou programas simples em C e Java, e o 26B retornou o programa em Java após carregar com sucesso. Os trechos foram apresentados no chat, sem compilação ou execução.

### FastAPI

O 26B retornou um exemplo de arquivo único com:

- Modelos Pydantic para produto e criação de produto.
- Armazenamento em dicionário na memória do servidor.
- Operações POST, GET da lista, GET por ID, PUT e DELETE.
- Resposta 404 para produto inexistente.
- Comandos de instalação e inicialização com Uvicorn.

A resposta foi concluída normalmente, mas não foi salva, executada ou testada. Sucesso da geração não equivale a validação funcional, de concorrência ou de adequação para produção.

### Blazor WebAssembly e ASP.NET Core

O 26B gerou 3.471 tokens e apresentou dois projetos, `ProductAPI` e `ProductClient`, com controller CRUD, modelos, CORS, HttpClient e página Razor com formulário e listagem.

Escolheu .NET 8. Sua afirmação de que essa era a versão LTS mais atual não foi validada e não deve ser tomada como informação atualizada.

Foram identificados erros visíveis que impediriam a compilação do exemplo como entregue:

```csharp
ss        var index = _products.FindIndex(p => p.Id == id);
```

```csharp
get => name = value ?? "";
```

```csharp
='        products = await Http.GetFromJsonAsync<List<Product>>("api/products") ?? new();
```

Os dois primeiros caracteres estranhos do primeiro e do terceiro trecho vieram na resposta do modelo. No getter, `value` foi usado indevidamente. O agente não corrigiu esses erros nem compilou os projetos.

**Avaliação observada:** o modelo consegue produzir a estrutura de um CRUD com cliente e servidor, mas a saída exige revisão e validação. A resposta de maior complexidade continha erros de sintaxe evidentes.

## Contexto para uso em outro chat Codex

- O Ollama está funcional no host; falhas no PATH ou acesso do sandbox devem ser distinguidas de falhas do serviço.
- Os dois modelos de embedding responderam; preservar a distinção entre vetores de 768 e 1.024 dimensões ao integrar armazenamento e consultas.
- O 12B funcionou com aceleração parcial na NVIDIA; a Intel não apresentou atividade nas amostras dos testes correspondentes.
- O 26B funcionou depois de reduzir o consumo prévio de RAM para aproximadamente 9 GB; seu carregamento foi o ponto crítico observado.
- Separar tempo de carregamento, processamento do prompt e geração ao avaliar desempenho. Pedidos com modelo já carregado não equivalem a partidas a frio.
- O modelo permaneceu carregado entre os pedidos bem-sucedidos. `ollama ps` permite verificar o estado atual; não assumir que continuará carregado após inatividade ou troca de modelo.
- Não foram ajustados contexto, quantização, número de camadas na GPU, cache ou parâmetros de thinking nos pedidos deste chat. Um teste futuro com outros parâmetros não será idêntico a estes.
- Não foram testados ferramentas/function calling, execução autônoma, contexto longo, concorrência, RAG, qualidade de busca ou correção automática do código.
- Não inferir qualidade de código somente pela ausência de erro HTTP. O exemplo Blazor demonstra a necessidade de revisão e compilação.
- O usuário pediu explicitamente para não encerrar processos do Ollama; esse limite foi respeitado.

Durante os testes, nenhum código retornado foi salvo ou executado e nenhuma dependência foi instalada. Este relatório foi criado posteriormente, mediante solicitação explícita do usuário. Os números descrevem observações da sessão e não o estado atual permanente da máquina.

## Fontes

- Saídas dos comandos executados no host, respostas da API local e `server.log` em `%LOCALAPPDATA%\Ollama`.
- Observação do usuário sobre aproximadamente 9 GB de RAM em uso antes do carregamento bem-sucedido e até aproximadamente 25 GB durante a operação.
- [Documentação do endpoint de geração do Ollama](https://docs.ollama.com/api/generate).
- [FAQ do Ollama: carregamento, contexto e distribuição CPU/GPU](https://docs.ollama.com/faq).
