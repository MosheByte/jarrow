// Expressive Code, the docs' code blocks, on the design tokens.
//
// Horizon's docs set code in one colour on a dark block: no syntax colours, radius 12 px,
// padding 1em 1.15em, Geist Mono at 13.5 px, line height 1.65
// (docs/design-research/horizon.md, section 2.1). Jarrow does the same with its tokens: code
// in --jw-fg on the card surface (white at 4 %), no border, no shadow. Every colour here is a
// var(--jw-*) or a value read from site/tokens.css, so scripts/check-colors.mjs passes the
// stylesheet Expressive Code generates.
//
// Starlight loads this file by itself (it sits next to astro.config.mjs).

import { defineEcConfig } from '@astrojs/starlight/expressive-code';

import { readTokens, token } from './scripts/tokens.mjs';

const tokens = readTokens();

/** A token as the theme's colour (#rrggbb); Expressive Code needs a value it can parse. */
const hex = (name) => {
  const value = token(tokens, name);
  if (!/^#[0-9a-f]{6}$/i.test(value)) throw new Error(`ec.config.mjs: ${name} is not an opaque colour`);
  return value;
};

const page = hex('--jw-page');
const fg = hex('--jw-fg');

/** One theme, dark, monochrome: every scope in the body text colour. */
const jarrow = {
  name: 'jarrow',
  type: 'dark',
  colors: {
    'editor.background': page,
    'editor.foreground': fg,
    'editor.selectionBackground': page,
    'editorLineNumber.foreground': fg,
    'editorLineNumber.activeForeground': fg,
    focusBorder: hex('--jw-accent'),
    'titleBar.border': page,
    'tab.activeBackground': page,
    'tab.activeForeground': fg,
    'tab.border': page,
    'editorGroupHeader.tabsBackground': page,
    'editorGroupHeader.tabsBorder': page,
    'terminal.background': page,
    'terminal.foreground': fg,
    'menu.selectionBackground': page,
    'menu.selectionForeground': fg,
    'scrollbarSlider.background': page,
    'scrollbarSlider.hoverBackground': page,
  },
  tokenColors: [{ settings: { foreground: fg, background: page } }],
};

const v = (name) => {
  token(tokens, name);
  return `var(${name})`;
};

export default defineEcConfig({
  themes: [jarrow],
  useStarlightDarkModeSwitch: false,
  useStarlightUiThemeColors: false,
  // Monochrome code has nothing to adjust, and an adjustment would invent a colour.
  minSyntaxHighlightingColorContrast: 0,
  styleOverrides: {
    borderRadius: v('--jw-radius-12'),
    borderWidth: '0px',
    borderColor: 'transparent',
    codeBackground: v('--jw-white-4'),
    codeForeground: v('--jw-fg'),
    codeFontFamily: v('--jw-font-mono'),
    codeFontSize: v('--jw-pre-size'),
    codeLineHeight: v('--jw-pre-leading'),
    codePaddingBlock: '1em',
    codePaddingInline: '1.15em',
    codeSelectionBackground: v('--jw-white-20'),
    gutterForeground: v('--jw-white-50'),
    gutterHighlightForeground: v('--jw-white-80'),
    gutterBorderColor: 'transparent',
    gutterBorderWidth: '0px',
    uiFontFamily: v('--jw-font-body'),
    uiFontSize: v('--jw-label-size'),
    uiSelectionBackground: v('--jw-white-20'),
    uiSelectionForeground: v('--jw-white'),
    focusBorder: v('--jw-accent'),
    scrollbarThumbColor: v('--jw-white-15'),
    scrollbarThumbHoverColor: v('--jw-white-20'),
    frames: {
      shadowColor: 'transparent',
      frameBoxShadowCssValue: 'none',
      editorBackground: v('--jw-white-4'),
      editorActiveTabBackground: v('--jw-white-4'),
      editorActiveTabForeground: v('--jw-white-80'),
      editorActiveTabBorderColor: 'transparent',
      editorActiveTabIndicatorHeight: '0px',
      editorActiveTabIndicatorTopColor: 'transparent',
      editorActiveTabIndicatorBottomColor: 'transparent',
      editorTabBarBackground: v('--jw-white-3'),
      editorTabBarBorderColor: 'transparent',
      editorTabBarBorderBottomColor: 'transparent',
      terminalBackground: v('--jw-white-4'),
      terminalTitlebarBackground: v('--jw-white-3'),
      terminalTitlebarForeground: v('--jw-white-60'),
      terminalTitlebarBorderBottomColor: 'transparent',
      terminalTitlebarDotsForeground: v('--jw-white-20'),
      terminalTitlebarDotsOpacity: '1',
      inlineButtonBackground: v('--jw-white'),
      inlineButtonForeground: v('--jw-white-80'),
      inlineButtonBorder: 'transparent',
      tooltipSuccessBackground: v('--jw-tier-green'),
      tooltipSuccessForeground: v('--jw-page'),
    },
    textMarkers: {
      // Marked lines sit on a lighter surface; inserted and deleted lines are told apart by
      // their indicator in the result tiers' green and red, never by a tinted background.
      markBackground: v('--jw-white-8'),
      markBorderColor: v('--jw-white-15'),
      insBackground: v('--jw-white-8'),
      insBorderColor: v('--jw-tier-green'),
      insDiffIndicatorColor: v('--jw-tier-green'),
      delBackground: v('--jw-white-4'),
      delBorderColor: v('--jw-tier-red'),
      delDiffIndicatorColor: v('--jw-tier-red'),
      lineMarkerLabelColor: v('--jw-white'),
      indicatorFontColor: v('--jw-white'),
    },
  },
});
