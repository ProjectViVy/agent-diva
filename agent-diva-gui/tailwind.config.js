/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        /** UX-IMPL-5：过程条弱对比附属色（可在组件内配合 CSS 变量使用） */
        'process-muted': {
          DEFAULT: 'rgb(148 163 184)',
          fg: 'rgb(71 85 105)',
          bg: 'rgb(248 250 252 / 0.92)',
          border: 'rgb(203 213 225 / 0.6)',
        },
        neuro: {
          synaptic: 'rgb(100 116 139)',
          pulse: 'rgb(244 114 182 / 0.35)',
        },
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
        }
      }
    },
  },
  plugins: [],
}
