/** @type {import('tailwindcss').Config} */
module.exports = {
  darkMode: 'class',
  content: ['./src/**/*.{html,js,ts,svelte}'],
  theme: {
    extend: {
      colors: {
        ink: {
          50: '#f5f7fb',
          100: '#e8edf5',
          200: '#c6d0e3',
          300: '#97a7c2',
          400: '#6f7f9d',
          500: '#51607e',
          600: '#39465f',
          700: '#273246',
          800: '#171f2e',
          900: '#0b1220',
          950: '#050816'
        },
        accent: {
          50: '#ecfeff',
          100: '#cffafe',
          200: '#a5f3fc',
          300: '#67e8f9',
          400: '#22d3ee',
          500: '#06b6d4',
          600: '#0891b2',
          700: '#0e7490',
          800: '#155e75',
          900: '#164e63'
        },
        warm: {
          50: '#fff7ed',
          100: '#ffedd5',
          200: '#fed7aa',
          300: '#fdba74',
          400: '#fb923c',
          500: '#f97316',
          600: '#ea580c',
          700: '#c2410c',
          800: '#9a3412',
          900: '#7c2d12'
        }
      },
      boxShadow: {
        panel: '0 20px 60px rgba(3, 7, 18, 0.22)',
        soft: '0 8px 24px rgba(15, 23, 42, 0.12)'
      },
      fontFamily: {
        sans: ['"Avenir Next"', '"Segoe UI Variable"', '"SF Pro Text"', 'system-ui', 'sans-serif']
      },
      backgroundImage: {
        'mesh-dark':
          'radial-gradient(circle at top left, rgba(34, 211, 238, 0.18), transparent 35%), radial-gradient(circle at top right, rgba(249, 115, 22, 0.12), transparent 30%), linear-gradient(180deg, rgba(5, 8, 22, 0.96), rgba(11, 18, 32, 1))',
        'mesh-light':
          'radial-gradient(circle at top left, rgba(34, 211, 238, 0.12), transparent 35%), radial-gradient(circle at top right, rgba(249, 115, 22, 0.08), transparent 30%), linear-gradient(180deg, rgba(245, 247, 251, 1), rgba(233, 238, 247, 1))'
      }
    }
  },
  plugins: []
};

