/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{vue,ts}'],
  darkMode: 'class',
  theme: {
    extend: {
      // Colors are driven by CSS variables (see src/assets/main.css) as RGB
      // channels so Tailwind's /opacity modifiers work and dark mode swaps
      // by toggling the `.dark` class on <html>.
      colors: {
        primary: 'rgb(var(--c-primary) / <alpha-value>)',
        secondary: 'rgb(var(--c-secondary) / <alpha-value>)',
        bg: 'rgb(var(--c-bg) / <alpha-value>)',
        card: 'rgb(var(--c-card) / <alpha-value>)',
        border: 'rgb(var(--c-border) / <alpha-value>)',
        accent: 'rgb(var(--c-accent) / <alpha-value>)',
        green: 'rgb(var(--c-green) / <alpha-value>)',
        amber: 'rgb(var(--c-amber) / <alpha-value>)',
        red: 'rgb(var(--c-red) / <alpha-value>)',
      },
      fontFamily: {
        sans: [
          '-apple-system',
          'BlinkMacSystemFont',
          '"Segoe UI"',
          'Roboto',
          'Helvetica',
          'Arial',
          'sans-serif',
        ],
      },
      fontSize: {
        display: ['34px', { lineHeight: '1.1', letterSpacing: '-0.02em', fontWeight: '700' }],
        h1: ['24px', { lineHeight: '1.2', fontWeight: '700' }],
        h2: ['18px', { lineHeight: '1.3', fontWeight: '600' }],
        body: ['14px', { lineHeight: '1.55' }],
        caption: ['12px', { lineHeight: '1.4', fontWeight: '500' }],
      },
      borderRadius: {
        control: '8px',
        card: '12px',
        pill: '999px',
      },
      boxShadow: {
        card: '0 1px 3px rgba(16,24,40,.08)',
        pop: '0 10px 30px -8px rgba(16,24,40,.25)',
      },
    },
  },
  plugins: [],
}
