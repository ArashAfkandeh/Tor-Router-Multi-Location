/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    './index.html',
    './Countries.html',
    './app.js',
  ],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        brand: {
          50: '#f5f3f7',
          100: '#e1d4e6',
          500: '#7d4698',
          600: '#64387a',
          900: '#2b1436',
        },
      },
    },
  },
  plugins: [],
};
