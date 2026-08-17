/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        yandere: {
          50: '#fff0f5',
          100: '#ffe3ee',
          200: '#ffc7df',
          300: '#ff9bc4',
          400: '#ff649f',
          500: '#ff3381',
          600: '#f01466',
          700: '#cc0a52',
          800: '#a80c46',
          900: '#8c103f',
        },
        /* 语义令牌：统一映射到 styles.css 的 CSS 变量（主题感知） */
        surface: {
          DEFAULT: 'var(--panel)',
          solid: 'var(--panel-solid)',
          raised: 'var(--surface-raised)',
          sunken: 'var(--surface-sunken)',
        },
        ink: {
          DEFAULT: 'var(--text)',
          muted: 'var(--text-muted)',
          faint: 'var(--text-faint)',
        },
        line: {
          DEFAULT: 'var(--line)',
          strong: 'var(--border-strong)',
        },
        brand: {
          DEFAULT: 'var(--brand)',
          light: 'var(--brand-light)',
        },
        accent: {
          DEFAULT: 'var(--accent)',
          light: 'var(--accent-light)',
        },
        state: {
          danger: 'var(--danger)',
          'danger-bg': 'var(--danger-bg)',
          success: 'var(--success)',
          'success-bg': 'var(--success-bg)',
          warning: 'var(--warning)',
          'warning-bg': 'var(--warning-bg)',
          info: 'var(--info)',
          'info-bg': 'var(--info-bg)',
        },
      },
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
        tk: 'var(--radius)',
        'tk-sm': 'var(--radius-sm)',
      },
      boxShadow: {
        tk: 'var(--shadow)',
      },
    },
  },
  plugins: [],
}
