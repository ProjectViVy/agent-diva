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
        }
      }
    },
  },
  plugins: [],
}
