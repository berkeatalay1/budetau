/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{vue,ts}'],
  theme: {
    extend: {
      colors: {
        primary: '#1f2937',
        secondary: '#4b5563',
        bg: '#f9fafb',
        card: '#ffffff',
        border: '#e5e7eb',
        accent: '#2563eb',
      },
    },
  },
  plugins: [],
}
