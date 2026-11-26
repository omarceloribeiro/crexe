# CREXE — Compatibilidade temporal generativa

*Uma visão para software, bibliotecas e compatibilidade de hardware além dos sistemas operacionais e do tempo.*

**Status:** white paper de visão e pesquisa.

**Versão:** rascunho 0.1; edição editorial em português de 26/11/2026.

**Validação:** os conceitos de drivers generativos, compatibilidade temporal de hardware e pontes entre épocas são prospectivos. Não foram validados experimentalmente como parte do CREXE. `.crdll` e `.crdrv` são extensões propostas, ainda não implementadas.

Esta é uma tradução integral com esclarecimentos editoriais de escopo e evidência. O [Markdown original em inglês](originais/CREXE_Generative_Temporal_Compatibility_White_Paper.md) permanece disponível sem alterações. Os exemplos abaixo ilustram hipóteses de pesquisa; não são receitas executáveis nem especificações de formatos já suportados. As referências a geração reproduzível ou comportamento determinístico são objetivos a verificar, não garantias da engine atual.

## Resumo

Tradicionalmente, o software só sobrevive enquanto o ambiente necessário para executá-lo permanece disponível.

Executáveis dependem de sistemas operacionais, processadores, runtimes, frameworks, bibliotecas, APIs e sistemas de build. Drivers de dispositivos também dependem de arquiteturas de kernel, interfaces de hardware e suporte dos fabricantes.

Quando essas dependências desaparecem, software e hardware que continuariam funcionais podem se tornar inutilizáveis.

O CREXE propõe outro modelo.

Em vez de tratar uma implementação compilada como o artefato permanente, o CREXE trata a **intenção e a especificação executável** como o artefato durável.

Um arquivo `.crexe` descreve o que uma aplicação deve ser. Dentro dos perfis suportados e das ferramentas configuradas, uma engine CREXE executada no ambiente de destino pode gerar, compilar, verificar e executar uma implementação para esse ambiente. Passar nas verificações disponíveis não comprova a correção de qualquer comportamento da aplicação.

Este documento explora uma extensão futura desse princípio para além das aplicações.

Surgem três classes conceituais de artefatos:

```text
.crexe   Creative Executable
.crdll   Creative Dynamic Library
.crdrv   Creative Driver
```

Juntas, elas sugerem uma possibilidade mais ampla:

> A compatibilidade de software não precisa necessariamente ser preservada na forma de binários.
> Ela poderia ser regenerada a partir de descrições duráveis de intenção, interfaces e comportamento.

Levado adiante, o mesmo princípio poderia se aplicar ao hardware.

Um futuro artefato `.crdrv` poderia descrever as capacidades e o contrato de comunicação de um dispositivo com detalhes suficientes para que o CREXE gerasse a ponte de software necessária ao sistema operacional em que o artefato fosse aberto.

Isso poderia funcionar tanto em direção ao futuro quanto em direção ao passado.

Uma impressora antiga poderia ganhar suporte em um sistema operacional futuro.

Uma impressora moderna poderia, potencialmente, ganhar suporte em um sistema operacional criado décadas antes de a impressora existir.

A implementação mudaria.

O contrato permaneceria.

## 1. Contexto: o modelo CREXE

O CREXE — **Creative Executable** — parte de uma proposição simples:

Um programa não precisa necessariamente ser distribuído como um executável compilado.

O artefato durável pode, em vez disso, descrever o programa desejado.

Por exemplo:

```text
calculadora.crexe
```

pode conter:

```text
Crie uma calculadora gráfica com suporte a
adição, subtração, multiplicação e divisão.
```

A engine CREXE na máquina de destino determina como materializar essa intenção dentro das capacidades disponíveis.

O mesmo artefato conceitual poderia resultar em diferentes implementações nativas:

```text
calculadora.crexe

        ↓

Windows
        → aplicação nativa para Windows

Linux
        → aplicação nativa para Linux

macOS
        → aplicação nativa para macOS
```

A propriedade central é a **portabilidade da intenção**, em vez da portabilidade do binário.

O objetivo é manter o `.crexe` enquanto a implementação é regenerada, desde que seus requisitos possam ser atendidos no destino escolhido.

## 2. Além de “escreva uma vez, execute em qualquer lugar”

As estratégias tradicionais de portabilidade geralmente preservam uma implementação.

Alguns exemplos são:

- máquinas virtuais;
- bytecode;
- camadas de compatibilidade;
- contêineres;
- ambientes de execução;
- emulação;
- frameworks multiplataforma.

Essas técnicas procuram fazer uma implementação substancialmente igual funcionar em vários ambientes.

O CREXE propõe outra direção:

> **Escreva a intenção uma vez. Gere a implementação onde ela for necessária.**

A distinção é fundamental.

Portabilidade tradicional:

```text
Código-fonte
  ↓
Implementação
  ↓
Portar a implementação entre ambientes
```

CREXE:

```text
Intenção
  ↓
Ambiente de destino
  ↓
Gerar uma implementação adequada
```

Para especificações suficientemente portáveis, isso poderia fazer do sistema operacional de destino, do framework e, potencialmente, da linguagem de programação escolhas da implementação, em vez de propriedades permanentes do artefato. Dependências ou restrições explícitas de plataforma em uma receita ainda precisam ser respeitadas.

## 3. A dimensão temporal

O mesmo modelo introduz uma possibilidade que vai além da portabilidade convencional entre plataformas.

Um arquivo `.crexe` criado hoje poderia, em tese, ser interpretado por uma engine CREXE implementada décadas depois.

Por exemplo:

```text
2026

calculadora.crexe
        ↓
Implementação para Windows
```

O mesmo artefato poderia, potencialmente, existir em:

```text
2046

calculadora.crexe
        ↓
Sistema operacional futuro
        ↓
Linguagem/framework futuro
        ↓
Calculadora
```

Uma especificação suficientemente independente não precisaria exigir que a implementação futura usasse as tecnologias disponíveis quando o arquivo `.crexe` foi criado. Essa aspiração depende de preservar semântica interpretável, contratos versionados e informação suficiente para testar o comportamento pretendido.

O artefato durável preservaria **o que o software deve fazer**, em vez de uma implementação histórica específica.

O CREXE introduz, portanto, a possibilidade de:

### Portabilidade temporal generativa

## 4. Bibliotecas dinâmicas criativas — `.crdll`

Aplicações são apenas uma forma de software.

O software moderno é composto por bibliotecas, serviços, drivers e outros componentes.

Uma extensão possível do CREXE seria:

```text
.crdll
Creative Dynamic Library
```

Um `.crdll` descreveria o contrato e o comportamento de um componente de software reutilizável.

Por exemplo:

```text
math.crdll
```

poderia descrever:

```text
Funções exportadas:

add(a, b)
subtract(a, b)
multiply(a, b)
divide(a, b)

Requisitos:

- determinístico
- seguro para uso concorrente entre threads
- sem acesso à rede
```

O ambiente de destino poderia determinar o artefato resultante.

```text
Windows
    → math.dll

Linux
    → libmath.so

macOS
    → libmath.dylib
```

Uma plataforma futura poderia usar um formato binário inteiramente diferente.

O objetivo seria reutilizar o contrato `.crdll` sem mudar o comportamento pretendido. Isso exigiria definições precisas de tipos, convenções de chamada, responsabilidade pela memória, erros e concorrência, com compatibilidade de API/ABI verificada em cada destino. O comportamento determinístico do componente é um requisito a testar, não uma garantia de geração determinística.

## 5. Drivers criativos — `.crdrv`

A compatibilidade de hardware introduz um problema mais difícil.

Um driver tradicional está profundamente ligado ao seu ambiente.

Ele pode depender de:

- APIs do kernel;
- arquitetura do processador;
- interfaces de barramento;
- modelos de drivers do sistema operacional;
- políticas de segurança;
- mecanismos de assinatura;
- SDKs dos fabricantes;
- protocolos de comunicação.

Por isso, drivers podem enfrentar dificuldades particulares ao envelhecer.

Um dispositivo perfeitamente funcional pode se tornar inutilizável simplesmente porque seu driver deixou de receber suporte.

O CREXE sugere um artefato alternativo:

```text
.crdrv
Creative Driver
```

O artefato durável não conteria necessariamente um driver Windows, um módulo de kernel Linux ou uma extensão de sistema do macOS.

Ele descreveria o hardware.

Por exemplo:

```text
printer.crdrv
```

poderia descrever:

```text
Dispositivo:
  classe: impressora
  fabricante: Exemplo
  modelo: Exemplo 600

Identificação:
  usb_vid: ...
  usb_pid: ...

Capacidades:
  A4
  monocromática
  600 dpi

Protocolo:
  PCL5

Comportamento:
  inicializar
  receber trabalho de impressão
  informar estado
  ejetar página
```

Uma futura engine CREXE poderia determinar uma forma candidata de materializar a compatibilidade com o ambiente de destino, sujeita a documentação suficientemente completa do dispositivo e validação específica para esse destino.

## 6. Driver como compatibilidade, além do código de kernel

Um princípio de projeto essencial é que `.crdrv` não se limite ao significado:

> Gere um driver de kernel.

Seu significado deveria ser:

> **Materialize a compatibilidade entre este hardware e este ambiente.**

Dependendo do sistema de destino, essa compatibilidade poderia exigir:

- um driver de kernel;
- um serviço em espaço de usuário;
- uma biblioteca dinâmica;
- um daemon;
- uma aplicação de impressão;
- firmware;
- um proxy;
- um tradutor de protocolos;
- uma ponte de hardware;
- ou vários desses componentes em conjunto.

Essa abstração é importante porque as arquiteturas dos sistemas operacionais mudam.

Um sistema operacional futuro pode nem ter o conceito de driver como ele é entendido hoje.

O `.crdrv` deveria, portanto, descrever a **relação**, sem definir prematuramente sua implementação.

## 7. Compatibilidade temporal em direção ao futuro

A aplicação mais evidente envolve hardware antigo e sistemas operacionais novos.

Considere uma impressora fabricada em 1998.

O hardware ainda funciona.

Seu fabricante deixou de produzir drivers há anos.

Um ecossistema convencional de software pode chegar à seguinte situação quando não existe uma implementação alternativa compatível:

```text
Hardware funcionando
+
Ausência de driver suportado
=
Hardware inutilizável naquele ambiente
```

Com um `.crdrv` suficientemente completo, a hipótese seria:

```text
Impressora de 1998
      +
printer.crdrv
      +
Sistema operacional de 2040
      ↓
CREXE
      ↓
Implementação compatível com 2040
```

O driver original não precisaria sobreviver.

O conhecimento necessário para se comunicar com o dispositivo, sim.

## 8. Compatibilidade temporal em direção ao passado

A possibilidade inversa é menos comum.

Considere um hardware fabricado em 2030 e um sistema operacional criado em 1984.

Nenhum driver para essa combinação poderia ter existido em 1984.

O hardware ainda não existia.

Um sistema de compatibilidade generativa poderia, potencialmente, criar um.

Por exemplo:

```text
Impressora moderna
      +
Apple Lisa
      +
printer.crdrv
      ↓
Compatibilidade gerada depois que ambos existem
```

Isso criaria **uma nova compatibilidade retroativamente**, em vez de preservar uma compatibilidade histórica.

Um dispositivo moderno poderia se tornar utilizável por um sistema operacional criado décadas antes de o dispositivo existir.

Esse conceito pode ser descrito como:

### Compatibilidade temporal reversa

Ou, de forma mais geral:

### Compatibilidade temporal generativa bidirecional

## 9. O padrão de ponte

Limitações físicas impedem muitas combinações de se comunicar diretamente.

Um Apple Lisa não passa a entender USB-C apenas porque uma IA gerou código-fonte.

Um computador sem PCI Express não consegue conectar diretamente um dispositivo NVMe que dependa dessa interface.

A compatibilidade generativa exige, portanto, outra abstração:

### A ponte

Exemplo:

```text
Apple Lisa
    │
    │ Serial
    ▼
Componente gerado para o lado Lisa
    │
    ▼
Ponte moderna
    │
    │ USB / Wi-Fi / Bluetooth
    ▼
Hardware moderno
```

O computador histórico se comunica por uma interface que já conhece.

A ponte traduz essa interface para o protocolo do hardware moderno.

O CREXE poderia, potencialmente, gerar software para os dois lados. A ponte física ainda precisaria existir ou ser projetada, construída e testada separadamente; gerar código não cria interfaces elétricas, conectores ou hardware de barramento.

## 10. Exemplo: impressora moderna no Windows 3.1

Conceitualmente:

```text
Windows 3.1
     ↓
Componente de impressão legado gerado
     ↓
Ponte local ou em rede
     ↓
IPP / protocolo moderno de impressão
     ↓
Impressora moderna
```

Do ponto de vista do Windows 3.1, o destino poderia se comportar como uma impressora que ele conhece.

A ponte absorveria a diferença entre as épocas.

## 11. Exemplo: controle Bluetooth em um retrocomputador

Um controle Bluetooth moderno poderia, potencialmente, ser usado com hardware anterior à existência do Bluetooth.

```text
Controle Bluetooth
        ↓
Ponte moderna
        ↓
Tradução gerada
        ↓
Interface de joystick / serial / teclado
        ↓
Retrocomputador
```

A máquina antiga não precisaria entender Bluetooth.

Ela enxergaria apenas uma interface nativa de sua época.

## 12. Exemplo: armazenamento NVMe e DOS

Um computador DOS não pode receber capacidades de PCI Express apenas por software.

Entretanto, a compatibilidade poderia ser representada como:

```text
DOS
 ↓
Interface de disco gerada
 ↓
Abstração compatível com IDE/SCSI
 ↓
Ponte de hardware/software
 ↓
Dispositivo NVMe
```

A unidade NVMe não se tornaria uma unidade IDE.

O sistema observaria uma abstração que sabe utilizar.

## 13. O experimento com o Lisa como precedente conceitual

O autor relata experimentação com o CREXE e plataformas históricas, incluindo o Lisa: software destinado a um ambiente de outra época foi gerado e compilado em um ambiente moderno de desenvolvimento, para posterior transferência ao destino histórico.

O relato inspira uma propriedade conceitual importante, mas esta publicação não inclui uma reprodução independente nem evidências suficientes para auditar o experimento.

Ele não valida drivers generativos, acesso a hardware moderno ou pontes entre épocas.

O princípio arquitetural que motiva a pesquisa é:

> O ambiente que realiza a geração não precisa necessariamente ser o ambiente histórico que originalmente produzia software para o destino.

Essa distinção é essencial para aplicações futuras de retrocomputação.

Geração, compilação e execução podem, potencialmente, ocorrer em lugares diferentes.

## 14. Um CREXE distribuído

Para dispositivos com poucos recursos, a própria engine CREXE não precisaria realizar todas as operações localmente.

Um sistema histórico pode não ter:

- memória suficiente;
- criptografia moderna;
- conectividade de rede;
- capacidade computacional;
- um compilador moderno;
- um modelo de IA.

Uma arquitetura distribuída poderia separar essas responsabilidades.

```text
Dispositivo de destino
      ↓
Descrição do destino
      ↓
Gerador CREXE em máquina moderna
      ↓
Artefato gerado compatível com o destino
      ↓
Dispositivo histórico
```

O artefato concebido para ser durável poderia continuar reutilizável entre os destinos suportados, enquanto a infraestrutura de geração mudaria. Compatibilidade universal é uma aspiração, não uma propriedade estabelecida: interpretação, recursos no destino, transporte e validação ainda precisariam estar disponíveis.

## 15. Preservação de hardware

Isso introduz outro papel possível para o CREXE:

### Preservação de hardware

Atualmente, projetos de preservação frequentemente procuram guardar:

- drivers originais;
- mídias originais de instalação;
- sistemas operacionais originais;
- emuladores;
- acervos de binários.

Esses materiais continuam importantes.

O CREXE acrescentaria outra estratégia de preservação:

> Preservar conhecimento suficiente sobre o dispositivo, em formato legível por máquina, para reconstruir a compatibilidade mais tarde.

Um acervo `.crdrv` poderia conter:

- identificação do hardware;
- descrições de protocolos;
- mapas de registradores;
- sequências de comandos;
- requisitos de temporização;
- capacidades;
- particularidades conhecidas;
- exemplos de transações;
- comportamento histórico dos drivers;
- vetores de teste;
- referências de documentação.

Em vez de preservar uma única implementação, esse acervo tentaria preservar o **conhecimento necessário para criar implementações**.

## 16. Um artefato atemporal

Isso conduz ao conceito mais amplo do CREXE.

Os artefatos tradicionais de software estão fortemente vinculados a uma época.

```text
Executável para Windows 95
Driver para Windows 7
Aplicação para macOS Intel
Módulo de kernel Linux
```

Cada artefato carrega pressupostos de sua época.

Um artefato CREXE busca conter menos detalhes de uma implementação histórica e mais intenção durável.

Conceitualmente:

```text
.crexe
.crdll
.crdrv
```

poderiam se tornar:

### Artefatos de software atemporais

Eles estariam menos ligados a uma geração específica de sistemas operacionais e mais à funcionalidade subjacente que descrevem.

## 17. A camada permanente

A arquitetura pode ser vista como duas camadas.

### Camada temporária

- código de máquina;
- linguagem de programação;
- toolkit de interface gráfica;
- API do kernel;
- compilador;
- formato de pacote;
- ABI;
- sistema operacional;
- arquitetura do processador.

Esses elementos mudam.

### Camada durável

- intenção;
- comportamento;
- interfaces;
- restrições;
- protocolo;
- capacidades;
- testes;
- contrato semântico.

O CREXE procura transferir o máximo possível de informação da camada temporária para a camada durável.

## 18. Limites do conceito

A compatibilidade generativa não pode eliminar a realidade física.

O CREXE não pode criar capacidades de hardware inexistentes.

Não pode fazer uma máquina sem um caminho físico de comunicação interagir diretamente com um dispositivo inacessível.

Não pode inferir com certeza um protocolo proprietário não documentado apenas a partir do nome de um produto.

Não pode garantir o comportamento correto do hardware a partir de uma especificação incompleta.

Alguns cenários podem exigir:

- documentação de protocolos;
- engenharia reversa;
- drivers originais;
- capturas de comunicação do hardware;
- emulação;
- adaptadores físicos;
- novo hardware para a ponte.

A segurança também se torna substancialmente mais importante quando a geração interage com componentes do kernel ou hardware físico.

Uma calculadora defeituosa pode produzir um resultado incorreto.

Um driver defeituoso pode derrubar um sistema, corromper o armazenamento ou, potencialmente, danificar equipamentos.

Por isso, `.crdrv` exigiria um modelo de confiança e validação substancialmente mais forte do que o de geração de aplicações `.crexe`.

## 19. Segurança e verificação

Uma futura arquitetura Creative Driver deveria tratar código gerado como não confiável até que houvesse evidências adequadas para seu uso.

Entre os requisitos possíveis estão:

- geração em ambiente isolado;
- análise estática;
- simulação de protocolos;
- vetores de teste;
- emulação de hardware;
- permissões baseadas em capacidades;
- especificações assinadas;
- geração reproduzível;
- componentes determinísticos quando viável;
- fronteiras explícitas de privilégios;
- implantação em etapas;
- reversão de alterações;
- aprovação humana para operações privilegiadas.

A geração de drivers não deve ser considerada equivalente à geração de aplicações comuns. Esses mecanismos são requisitos de pesquisa, não proteções entregues pela engine CREXE atual. Fixar sementes, versões de modelos ou artefatos em cache, por si só, não comprova geração reproduzível nem comportamento correto do hardware.

## 20. Família de artefatos CREXE

O modelo conceitual de longo prazo seria:

```text
CREXE
│
├── .crexe
│      Creative Executable
│      descreve uma aplicação
│
├── .crdll
│      Creative Dynamic Library
│      descreve um componente de software reutilizável
│
└── .crdrv
       Creative Driver
       descreve compatibilidade de hardware
```

Essa taxonomia trata das funções conceituais dos artefatos. Ela não substitui `.crexp` e `.crexb`, formatos já presentes no projeto para pacotes de especificações e bundles com receita e geração aceita.

Esses artefatos poderiam referenciar uns aos outros.

Por exemplo:

```text
scanner.crexe
   │
   ├── image-processing.crdll
   │
   └── scanner.crdrv
```

Uma futura engine CREXE que levasse em conta o ambiente de destino poderia materializar e validar a implementação de cada componente.

## 21. Uma possível propriedade de longo prazo

Se essa arquitetura amadurecesse suficientemente, o seguinte poderia, em tese, ser verdadeiro:

```text
Mesmo artefato
Década diferente
Sistema operacional diferente
Processador diferente
Framework diferente
Implementação diferente
Mesma intenção
```

Para hardware:

```text
Mesmo contrato do dispositivo
Década diferente
Sistema operacional diferente
Arquitetura de driver diferente
Mesma capacidade de hardware
```

Essa propriedade iria além da portabilidade.

Seria **regeneração**.

## 22. Questões de pesquisa

As perguntas a seguir permanecem abertas e devem ser tratadas como temas de pesquisa, não como capacidades existentes do CREXE.

1. Quanta informação um `.crdrv` precisa preservar para permitir uma regeneração futura confiável?
2. Descrições de protocolos podem se tornar suficientemente legíveis por máquina para sobreviver a mudanças na arquitetura de drivers?
3. Como representar compatibilidade de ABI e API em um `.crdll`?
4. Como validar drivers gerados com segurança antes de conceder acesso privilegiado?
5. Quais partes da compatibilidade de hardware podem funcionar em espaço de usuário, em vez de espaço de kernel?
6. Como o CREXE deveria determinar quando uma ponte física é necessária?
7. O próprio firmware da ponte pode se tornar um artefato generativo CREXE?
8. Como as especificações devem evoluir e continuar interpretáveis por futuras engines CREXE?
9. É possível reproduzir implementações geradas de forma determinística?
10. Como converter documentação histórica de software e hardware em contratos CREXE duráveis?

## 23. Estado atual

O projeto CREXE oferece a base conceitual desta visão por meio da intenção executável e da geração específica para um ambiente.

Os conceitos introduzidos neste documento ampliam esse modelo.

Eles não devem ser interpretados como alegações de que o CREXE já gera drivers de hardware prontos para produção ou garante compatibilidade entre sistemas históricos.

Especificamente:

```text
.crexe
```

representa o conceito fundamental já existente.

```text
.crdll
```

é uma extensão proposta para bibliotecas generativas.

```text
.crdrv
```

é uma direção de pesquisa proposta para compatibilidade generativa de hardware.

A geração de drivers entre épocas, tanto em direção ao futuro quanto ao passado, permanece um objetivo futuro.

## 24. Visão de longo prazo

A história da computação descarta repetidamente software e hardware funcionais porque o ambiente necessário à implementação deixa de existir.

O CREXE investiga se outro tipo de artefato poderia sobreviver por mais tempo.

A proposta é preservar a informação necessária para criar uma nova implementação, sem depender de manter cada implementação para sempre.

Para drivers, isso significa preservar o contrato entre a máquina e o dispositivo.

Para computadores futuros, significa permitir a reinterpretação de intenções antigas, além da preservação de binários antigos.

Para computadores antigos, significa gerar pontes que traduzam entre seu mundo e o dos dispositivos futuros.

A visão de longo prazo do CREXE pode ser expressa assim:

> **Preserve a intenção. Preserve os contratos. Regenere a implementação.**

Aplicações podem mudar.

Bibliotecas podem mudar.

Drivers podem mudar.

Sistemas operacionais podem desaparecer.

Linguagens de programação podem desaparecer.

Arquiteturas de processadores podem desaparecer.

O artefato que descreve o que o sistema deve realizar pode permanecer.

## 25. Declaração de visão

**O CREXE explora um software cuja forma durável é a intenção que o descreve, em vez de uma implementação específica.**

O `.crexe` aplica esse princípio às aplicações.

O `.crdll` poderia estendê-lo a componentes de software reutilizáveis.

O `.crdrv` poderia estendê-lo à compatibilidade de hardware.

Juntos, eles introduzem a possibilidade de artefatos de software menos dependentes de sistemas operacionais, linguagens de programação, gerações de hardware e até da época em que foram criados.

A visão vai além de “escreva uma vez, execute em qualquer lugar” e de “compile uma vez, preserve para sempre”:

### Descreva uma vez. Regenere através do tempo.
