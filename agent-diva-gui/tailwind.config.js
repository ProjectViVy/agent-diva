/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: Object.fromEntries([
        'background', 'foreground', 'card', 'card-foreground', 'popover', 'popover-foreground',
        'primary', 'primary-foreground', 'primary-hover', 'secondary', 'secondary-foreground',
        'muted', 'muted-foreground', 'muted-foreground-subtle', 'accent', 'accent-foreground',
        'border', 'border-strong', 'input', 'ring', 'overlay', 'sidebar', 'sidebar-foreground',
        'sidebar-accent', 'sidebar-accent-foreground', 'destructive', 'destructive-foreground',
        'destructive-soft', 'success', 'success-soft', 'warning', 'warning-soft', 'info', 'info-soft',
      ].map(role => [role, `color-mix(in srgb, var(--${role}) calc(<alpha-value> * 100%), transparent)`])),
      /* 令牌字号阶梯（tk-* 前缀，避免覆盖 Tailwind 默认 text-* 语义） */
      fontSize: {
        'tk-xs': ['var(--font-size-xs)', { lineHeight: 'var(--line-height-xs)' }],
        'tk-sm': ['var(--font-size-sm)', { lineHeight: 'var(--line-height-sm)' }],
        'tk-base': ['var(--font-size-base)', { lineHeight: 'var(--line-height-base)' }],
        'tk-md': ['var(--font-size-md)', { lineHeight: 'var(--line-height-md)' }],
        'tk-lg': ['var(--font-size-lg)', { lineHeight: 'var(--line-height-lg)' }],
        'tk-xl': ['var(--font-size-xl)', { lineHeight: 'var(--line-height-xl)' }],
      },
      fontWeight: {
        'tk-normal': 'var(--font-weight-normal)',
        'tk-medium': 'var(--font-weight-medium)',
        'tk-semibold': 'var(--font-weight-semibold)',
      },
      borderRadius: {
        tk: 'var(--radius-lg)',
        'tk-sm': 'var(--radius-md)',
      },
      boxShadow: {
        tk: 'var(--shadow-md)',
        sm: 'var(--shadow-sm)',
        md: 'var(--shadow-md)',
        lg: 'var(--shadow-lg)',
        xl: 'var(--shadow-lg)',
        '2xl': 'var(--shadow-lg)',
      },
    },
  },
  plugins: [],
}
