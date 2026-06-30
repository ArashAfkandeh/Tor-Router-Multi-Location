const path = require('path');
/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    path.join(__dirname, 'index.html'),
    path.join(__dirname, 'Countries.html'),
    path.join(__dirname, 'src/**/*.js'),
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
