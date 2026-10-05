import '../styles.css';
import { afterEach, describe, expect, it } from 'vitest';

type NavItem = {
  row: HTMLElement;
  icon: SVGElement;
};

function createNavItem(className: string): NavItem {
  const row = document.createElement(className.includes('nav-group-header') ? 'div' : 'button');
  row.className = className;

  const icon = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  row.append(icon, document.createElement('span'));
  return { row, icon };
}

function mountExpandedSidebar() {
  const shell = document.createElement('div');
  shell.className = 'app-shell sidebar-expanded';

  const sidebar = document.createElement('aside');
  sidebar.className = 'sidebar';
  const menuToggle = createNavItem('menu-toggle');
  const nav = document.createElement('nav');
  nav.className = 'sidebar-nav';

  const primary = createNavItem('nav-item');
  const group = createNavItem('nav-group-header');
  const nestedItems = document.createElement('div');
  nestedItems.className = 'nav-group-items';
  const nested = createNavItem('nav-item nav-item-sub');
  const footer = document.createElement('div');
  footer.className = 'sidebar-footer';
  const settings = createNavItem('nav-item');
  footer.append(settings.row);
  nestedItems.append(nested.row);
  nav.append(primary.row, group.row, nestedItems);
  sidebar.append(menuToggle.row, nav, footer);
  shell.append(sidebar);
  document.body.append(shell);

  return { primary, group, nested, nestedItems, menuToggle, footer, settings, sidebar };
}

afterEach(() => {
  document.body.replaceChildren();
  document.documentElement.removeAttribute('data-theme');
});

describe('expanded navigation style contract', () => {
  it('gives primary, group, and nested rows the same full-width 40px hit area', () => {
    const { primary, group, nested, nestedItems } = mountExpandedSidebar();

    for (const row of [primary.row, group.row, nested.row]) {
      const style = getComputedStyle(row);
      expect(style.height).toBe('40px');
      expect(style.minHeight).toBe('40px');
      expect(style.boxSizing).toBe('border-box');
      expect(style.width).toBe('100%');
    }

    expect(getComputedStyle(nestedItems).paddingLeft).toBe('0px');
    expect(getComputedStyle(nested.row).paddingLeft).toBe('22px');
  });

  it('aligns the expanded menu toggle and footer with the navigation scrollport', () => {
    const { menuToggle, footer, sidebar } = mountExpandedSidebar();
    const gutter = getComputedStyle(sidebar).getPropertyValue('--sidebar-scrollbar-gutter').trim();
    const expectedWidth = `calc(100% - ${gutter})`;

    expect(gutter).toBe('10px');
    expect(getComputedStyle(menuToggle.row).width).toBe(expectedWidth);
    expect(getComputedStyle(footer).width).toBe(expectedWidth);
  });

  it.each(['love', 'dark', 'default', 'miku'])('uses the same inactive foreground and icon color in %s', (theme) => {
    document.documentElement.setAttribute('data-theme', theme);
    const { primary, group, nested } = mountExpandedSidebar();
    const rows = [primary, group, nested];

    expect(new Set(rows.map(({ row }) => getComputedStyle(row).color)).size).toBe(1);
    expect(new Set(rows.map(({ icon }) => getComputedStyle(icon).color)).size).toBe(1);
  });

  it.each(['love', 'dark', 'default', 'miku'])('uses one active surface and icon color in %s', (theme) => {
    document.documentElement.setAttribute('data-theme', theme);
    const { primary, group, nested } = mountExpandedSidebar();
    const rows = [primary, group, nested];
    rows.forEach(({ row }) => row.classList.add('active'));

    expect(new Set(rows.map(({ row }) => getComputedStyle(row).backgroundColor)).size).toBe(1);
    expect(new Set(rows.map(({ icon }) => getComputedStyle(icon).color)).size).toBe(1);
  });
});
