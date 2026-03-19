import { createHighlighter, type Highlighter } from 'shiki';
import type { ThemeRegistration } from '@shikijs/types';
import type { Theme, ThemeSyntaxSurface } from './themes';

let highlighterPromise: Promise<Highlighter> | null = null;
const loadedThemeNames = new Set<string>();

const SUPPORTED_LANGUAGES = [
  'typescript',
  'javascript',
  'python',
  'rust',
  'go',
  'bash',
  'shell',
  'json',
  'yaml',
  'toml',
  'markdown',
  'html',
  'css',
  'sql',
  'diff',
  'tsx',
  'jsx',
  'scss',
  'c',
  'cpp',
  'java',
  'ruby',
  'php',
  'swift',
  'kotlin',
  'lua',
  'r',
  'xml',
] as const;

export type SupportedLanguage = (typeof SUPPORTED_LANGUAGES)[number];
export type CodeSurface = 'markdownBlock' | 'markdownInline' | 'sourceFile';

export async function getHighlighter(): Promise<Highlighter> {
  if (!highlighterPromise) {
    highlighterPromise = createHighlighter({
      themes: ['github-dark'],
      langs: [...SUPPORTED_LANGUAGES],
    });
  }
  return highlighterPromise;
}

export async function highlightCodeToHtml(
  code: string,
  language: string | undefined,
  surface: Exclude<CodeSurface, 'markdownInline'>,
  theme: Theme
): Promise<string> {
  const highlighter = await getHighlighter();
  const themeRegistration = createThemeRegistration(theme, surface);
  const themeName = shikiThemeName(theme, surface);
  await ensureThemeLoaded(highlighter, themeRegistration);

  return highlighter.codeToHtml(code, {
    lang: normalizeLanguage(language),
    theme: themeName,
  });
}

export function getInlineCodeStyles(theme: Theme): {
  color: string;
  borderColor: string;
  backgroundColor: string;
} {
  const surface = theme.syntax.markdownInline;
  return {
    color: surface.foreground,
    borderColor: `${theme.colors.borderDefault}80`,
    backgroundColor: 'transparent',
  };
}

export function normalizeLanguage(lang: string | undefined): string {
  if (!lang) return 'text';

  const normalized = lang.toLowerCase().trim();
  const aliases: Record<string, string> = {
    ts: 'typescript',
    js: 'javascript',
    py: 'python',
    rs: 'rust',
    sh: 'bash',
    zsh: 'bash',
    yml: 'yaml',
    md: 'markdown',
    htm: 'html',
    cxx: 'cpp',
    h: 'c',
    hpp: 'cpp',
    kt: 'kotlin',
    rb: 'ruby',
    svg: 'xml',
  };

  const mapped = aliases[normalized] || normalized;
  if (SUPPORTED_LANGUAGES.includes(mapped as SupportedLanguage)) {
    return mapped;
  }

  return 'text';
}

export function isSupportedLanguage(lang: string): boolean {
  return SUPPORTED_LANGUAGES.includes(normalizeLanguage(lang) as SupportedLanguage);
}

async function ensureThemeLoaded(
  highlighter: Highlighter,
  themeRegistration: ThemeRegistration
): Promise<void> {
  const themeName = themeRegistration.name ?? 'conduit-syntax-theme';
  if (loadedThemeNames.has(themeName)) {
    return;
  }

  await highlighter.loadTheme(themeRegistration);
  loadedThemeNames.add(themeName);
}

function createThemeRegistration(
  theme: Theme,
  surface: Exclude<CodeSurface, 'markdownInline'>
): ThemeRegistration {
  const syntaxSurface = theme.syntax[surface];

  return {
    name: shikiThemeName(theme, surface),
    type: theme.isLight ? 'light' : 'dark',
    colors: {
      'editor.foreground': syntaxSurface.foreground,
      'editor.background': syntaxSurface.background,
      'editor.selectionBackground': syntaxSurface.selection,
      'editor.selectionForeground': syntaxSurface.selectionForeground,
      'editor.lineHighlightBackground': syntaxSurface.lineHighlight,
      'editorCursor.foreground': syntaxSurface.caret,
      'editorGutter.background': syntaxSurface.gutter,
      'editorLineNumber.foreground': syntaxSurface.gutterForeground,
    },
    settings: [
      tokenRule(
        'comment',
        syntaxSurface.tokens.comment,
        undefined,
        ['comment', 'punctuation.definition.comment']
      ),
      tokenRule(
        'keyword',
        syntaxSurface.tokens.keyword,
        syntaxSurface.fontStyles.keywordBold ? 'bold' : undefined,
        ['keyword', 'keyword.control', 'keyword.operator.word', 'storage', 'storage.modifier']
      ),
      tokenRule(
        'type',
        syntaxSurface.tokens.type,
        syntaxSurface.fontStyles.typeBold ? 'bold' : undefined,
        [
          'storage.type',
          'entity.name.type',
          'entity.name.class',
          'entity.name.struct',
          'support.type',
          'support.class',
        ]
      ),
      tokenRule(
        'function',
        syntaxSurface.tokens.function,
        undefined,
        ['entity.name.function', 'meta.function-call', 'support.function', 'variable.function']
      ),
      tokenRule(
        'string',
        syntaxSurface.tokens.string,
        undefined,
        ['string', 'string.regexp', 'string.quoted', 'punctuation.definition.string']
      ),
      tokenRule(
        'constant',
        syntaxSurface.tokens.constant,
        undefined,
        ['constant.numeric', 'constant.language', 'constant.character.escape', 'constant.other']
      ),
      tokenRule(
        'property',
        syntaxSurface.tokens.property,
        undefined,
        [
          'entity.name.tag',
          'entity.other.attribute-name',
          'support.type.property-name',
          'variable.other.member',
        ]
      ),
      tokenRule(
        'parameter',
        syntaxSurface.tokens.parameter,
        undefined,
        ['variable.parameter']
      ),
      tokenRule(
        'punctuation',
        syntaxSurface.tokens.punctuation,
        undefined,
        ['punctuation', 'meta.brace', 'meta.delimiter', 'meta.separator']
      ),
      tokenRule(
        'invalid',
        syntaxSurface.tokens.invalid,
        syntaxSurface.fontStyles.invalidUnderline ? 'underline' : undefined,
        ['invalid', 'invalid.illegal']
      ),
    ],
  };
}

function shikiThemeName(theme: Theme, surface: Exclude<CodeSurface, 'markdownInline'>): string {
  return `${theme.name}-${surface}`;
}

function tokenRule(
  name: string,
  foreground: string,
  fontStyle: string | undefined,
  scope: string[]
) {
  return {
    name,
    scope,
    settings: {
      foreground,
      ...(fontStyle ? { fontStyle } : {}),
    },
  };
}

export function syntaxSurfaceFor(theme: Theme, surface: CodeSurface): ThemeSyntaxSurface {
  return theme.syntax[surface];
}
