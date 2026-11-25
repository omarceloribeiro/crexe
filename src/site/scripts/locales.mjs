export const supportedLocales = ['pt-BR', 'en'];

const commonGroups = [
  ['whitepapers', ['whitepaper']], ['rfcs', ['rfc']], ['guias', ['guide']],
  ['artigos', ['article', 'manifesto']], ['referencias', ['reference']],
];

const editions = {
  'pt-BR': {
    language: 'pt-BR', root: '', library: 'biblioteca', assetPrefix: '../',
    categoryLabels: { whitepaper: 'White paper', guide: 'Guia', article: 'Artigo', manifesto: 'Manifesto', reference: 'Referência', rfc: 'RFC' },
    groupText: [
      ['White papers', 'Fundamentos da implementação e propostas para investigar o futuro.'],
      ['RFCs', 'Rascunhos da especificação, disponíveis em português e no original em inglês.'],
      ['Guias e análises', 'Contexto para avaliar modelos, preparar o ambiente e entender as fronteiras de confiança.'],
      ['Artigos, ensaio e manifesto', 'Perspectivas editoriais sobre intenção, criação e transformação de software.'],
      ['Referências da implementação', 'Documentação mantida no repositório, com os contratos e limites da versão atual.'],
    ],
    libraryTitle: 'Biblioteca', libraryDescription: 'White papers, artigos, guias e referências para compreender o CREXE, sua implementação e suas propostas de futuro.',
    introEyebrow: 'BIBLIOTECA CREXE', introTitle: 'Ideias para ler.<br><em>Software para repensar.</em>',
    intro: 'Fundamentos técnicos, perspectivas editoriais e documentação para explorar a computação por intenção. Cada texto identifica seu escopo: implementação atual, análise ou visão de futuro.',
    skip: 'Pular para o conteúdo', home: 'CREXE, início', navigation: 'Navegação principal', languages: 'Idioma',
    categories: 'Categorias da biblioteca', featured: 'COMECE PELOS FUNDAMENTOS', minutes: 'min de leitura', read: 'Ler',
    toc: 'Sumário do texto', inThisText: 'Neste texto', scope: 'Status e escopo do texto',
    download: 'Baixar fonte em Markdown', original: 'Baixar original em inglês', print: 'Imprimir texto', back: 'Voltar à biblioteca', related: 'Continue a leitura',
    historyTitle: 'Os rascunhos também fazem parte da história.',
    history: 'Os documentos originais foram preservados nos idiomas em que foram escritos. Algumas propostas, contratos e afirmações não descrevem a versão atual.',
    historyLink: 'Consultar o arquivo histórico em Markdown', projectLinks: 'Links do projeto', project: 'Conheça o projeto', security: 'Segurança',
    table: 'Tabela com rolagem horizontal',
  },
  en: {
    language: 'en', root: 'en/', library: 'en/library', assetPrefix: '../../',
    categoryLabels: { whitepaper: 'White paper', guide: 'Guide', article: 'Article', manifesto: 'Manifesto', reference: 'Reference', rfc: 'RFC' },
    groupText: [
      ['White papers', 'Implementation foundations and proposals for exploring the future.'],
      ['RFCs', 'Specification drafts, available in Portuguese and in their original English.'],
      ['Guides and analysis', 'Context for evaluating models, preparing your environment, and understanding trust boundaries.'],
      ['Articles, essay, and manifesto', 'Editorial perspectives on intent, creation, and software transformation.'],
      ['Implementation references', 'Documentation maintained in the repository, describing the current version’s contracts and limits.'],
    ],
    libraryTitle: 'Library', libraryDescription: 'White papers, articles, guides, and references on CREXE, its implementation, and its proposals for the future.',
    introEyebrow: 'CREXE LIBRARY', introTitle: 'Ideas to read.<br><em>Software to rethink.</em>',
    intro: 'Technical foundations, editorial perspectives, and documentation for exploring intent-based computing. Each text identifies its scope: current implementation, analysis, or a vision for the future.',
    skip: 'Skip to content', home: 'CREXE, home', navigation: 'Main navigation', languages: 'Language',
    categories: 'Library categories', featured: 'START WITH THE FOUNDATIONS', minutes: 'min read', read: 'Read',
    toc: 'Table of contents', inThisText: 'In this text', scope: 'Status and scope',
    download: 'Download Markdown source', original: 'Download English original', print: 'Print text', back: 'Back to the library', related: 'Continue reading',
    historyTitle: 'Drafts are part of the story, too.',
    history: 'The original documents are preserved in the languages in which they were written. Some proposals, contracts, and claims do not describe the current version.',
    historyLink: 'Browse the historical Markdown archive', projectLinks: 'Project links', project: 'Explore the project', security: 'Security',
    table: 'Horizontally scrollable table',
  },
};

export function getLocale(language = 'pt-BR') {
  const edition = editions[language];
  if (!edition) throw new Error(`Unsupported site language: ${language}`);
  return { ...edition, groups: commonGroups.map(([id, categories], index) => ({ id, categories, title: edition.groupText[index][0], description: edition.groupText[index][1] })) };
}

export function publicationPath(language, slug = 'index') {
  return `${getLocale(language).library}/${slug}.html`;
}

export function canonicalPath(page) {
  return page.endsWith('index.html') ? page.slice(0, -10) : page.replace(/\.html$/, '');
}
