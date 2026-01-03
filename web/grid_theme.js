function resolveCssVar(style, name, fallback) {
  const value = style.getPropertyValue(name);
  return value && value.trim() ? value.trim() : fallback;
}

export function readGridTheme(rootEl = document.documentElement) {
  const style = getComputedStyle(rootEl);
  return {
    bgPrimary: resolveCssVar(style, "--bg-primary", "#0d1117"),
    bgSecondary: resolveCssVar(style, "--bg-secondary", "#161b22"),
    bgTertiary: resolveCssVar(style, "--bg-tertiary", "#21262d"),
    borderPrimary: resolveCssVar(style, "--border-primary", "#30363d"),
    borderSecondary: resolveCssVar(style, "--border-secondary", "#21262d"),
    textPrimary: resolveCssVar(style, "--text-primary", "#e6edf3"),
    textSecondary: resolveCssVar(style, "--text-secondary", "#8b949e"),
    textMuted: resolveCssVar(style, "--text-muted", "#6e7681"),
    accentBlue: resolveCssVar(style, "--accent-blue", "#58a6ff"),
    accentGreen: resolveCssVar(style, "--accent-green", "#3fb950"),
    accentRed: resolveCssVar(style, "--accent-red", "#f85149"),
    accentYellow: resolveCssVar(style, "--accent-yellow", "#d29922"),
    accentPurple: resolveCssVar(style, "--accent-purple", "#a371f7"),
    diffAddBg: resolveCssVar(style, "--diff-add-bg", "rgba(46, 160, 67, 0.15)"),
    diffRemoveBg: resolveCssVar(style, "--diff-remove-bg", "rgba(248, 81, 73, 0.15)"),
    diffModifyBg: resolveCssVar(style, "--diff-modify-bg", "rgba(210, 153, 34, 0.15)"),
    diffMoveBg: resolveCssVar(style, "--diff-move-bg", "rgba(163, 113, 247, 0.15)"),
    diffMoveBorder: resolveCssVar(style, "--diff-move-border", "rgba(163, 113, 247, 0.4)"),
    diffMoveDstBg: resolveCssVar(style, "--diff-move-dst-bg", "rgba(163, 113, 247, 0.25)")
  };
}
