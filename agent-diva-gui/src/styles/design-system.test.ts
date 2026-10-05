import { readFileSync, existsSync, readdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import postcss from 'postcss';
import { describe, expect, it } from 'vitest';

function stylesheet(path: string): string {
  const source = readFileSync(path, 'utf8');
  return source.replace(/@import\s+["']([^"']+)["'];/g, (_, name: string) => {
    const dependency = resolve(dirname(path), name);
    return existsSync(dependency) ? stylesheet(dependency) : '';
  });
}

const css = stylesheet(resolve(import.meta.dirname, '../styles.css'));
const root = postcss.parse(css);
function themeTokens(theme: string): Record<string, string> {
  const tokens: Record<string, string> = {};
  root.walkRules((rule) => {
    if (rule.selector.split(',').some((s) => s.trim() === ':root' || s.trim() === `:root[data-theme="${theme}"]`)) {
      rule.walkDecls(/^--/, (decl) => { tokens[decl.prop] = decl.value; });
    }
  });
  return tokens;
}
function luminance(hex: string): number {
  const rgb = hex.replace('#', '').match(/.{2}/g)!.map((c) => parseInt(c, 16) / 255)
    .map((c) => c <= .04045 ? c / 12.92 : ((c + .055) / 1.055) ** 2.4);
  return .2126 * rgb[0] + .7152 * rgb[1] + .0722 * rgb[2];
}

describe('theme design contract', () => {
  it.each(['love', 'dark', 'default', 'miku'])('provides paired visual roles in %s', (theme) => {
    const tokens = themeTokens(theme);
    for (const role of ['background', 'card', 'popover', 'primary', 'secondary', 'muted', 'accent']) {
      expect(tokens[`--${role}`], `${theme}: ${role}`).toBeTruthy();
      expect(tokens[role === 'background' ? '--foreground' : `--${role}-foreground`], `${theme}: ${role} foreground`).toBeTruthy();
    }
    expect(tokens['--primary']).not.toEqual(tokens['--accent']);
  });

  it('keeps Miku primary text readable against the existing cyan', () => {
    const tokens = themeTokens('miku');
    expect(tokens['--primary']).toBe('#39c5bb');
    const foreground = tokens['--primary-foreground'];
    expect(foreground).toMatch(/^#[0-9a-f]{6}$/i);
    const light = luminance(tokens['--primary']);
    const dark = luminance(foreground);
    expect((Math.max(light, dark) + .05) / (Math.min(light, dark) + .05)).toBeGreaterThanOrEqual(4.5);
  });

  it('does not flatten text hierarchy through theme utility overrides', () => {
    const selectors: string[] = [];
    root.walkRules((rule) => {
      if (/\.theme-\w+\s+\.(?:text-gray|bg-white|border-gray)/.test(rule.selector)) selectors.push(rule.selector);
    });
    expect(selectors).toEqual([]);
  });

  it('does not redefine Tailwind semantic text utilities with a different role', () => {
    const mismatches: string[] = [];
    root.walkRules((rule) => {
      const role = rule.selector.match(/^\.text-(foreground|primary|secondary|muted|accent|success|warning|destructive|info)$/)?.[1];
      if (role) rule.walkDecls('color', (decl) => {
        if (decl.value !== `var(--${role})`) mismatches.push(`${rule.selector}: ${decl.value}`);
      });
    });
    expect(mismatches).toEqual([]);
  });

  it('pairs strong surfaces with their own foreground in shared and scoped CSS', () => {
    const sourceDir = resolve(import.meta.dirname, '..');
    const scoped = readdirSync(sourceDir, { recursive: true })
      .filter((file) => typeof file === 'string' && file.endsWith('.vue') && !file.endsWith('DivaVrmAvatar.vue'))
      .flatMap((file) => [...readFileSync(resolve(sourceDir, file as string), 'utf8')
        .matchAll(/<style[^>]*>([\s\S]*?)<\/style>/g)].map((match) => match[1]));
    const errors: string[] = [];
    for (const source of [css, ...scoped]) {
      postcss.parse(source).walkRules((rule) => {
        let surface = '';
        rule.walkDecls(/^(background|background-color)$/, (decl) => {
          surface = decl.value.match(/^var\(--(primary|success|warning|destructive|info|message-user)\)$/)?.[1] || surface;
        });
        if (surface) rule.walkDecls('color', (decl) => {
          if (decl.value !== `var(--${surface}-foreground)`) errors.push(`${rule.selector}: ${surface} / ${decl.value}`);
        });
      });
    }
    expect(errors).toEqual([]);
  });
});
