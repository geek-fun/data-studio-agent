import eslint from '@eslint/js';
import tsparser from '@typescript-eslint/parser';
import tseslint from '@typescript-eslint/eslint-plugin';
import prettier from 'eslint-plugin-prettier';
import globals from 'globals';

export default [
  { ignores: ['node_modules', 'dist', '*.d.ts', 'src/**/*.d.ts', 'coverage', 'src/**/*.js'] },

  eslint.configs.recommended,

  {
    files: ['src/**/*.ts', 'bin/**/*.js'],
    languageOptions: {
      parser: tsparser,
      parserOptions: { sourceType: 'module', ecmaVersion: 'latest' },
      globals: {
        ...globals.node,
        ...globals.es2021,
      },
    },
    plugins: { '@typescript-eslint': tseslint, prettier },
    rules: {
      'prettier/prettier': 'error',
      'arrow-parens': [2, 'as-needed'],
      'no-var': 'error',
      'no-console': ['warn', { allow: ['error', 'warn'] }],
      'no-debugger': 'error',
      'no-useless-catch': 'warn',
      'no-empty': 'warn',
      'no-case-declarations': 'warn',
      'no-unused-vars': 'off',
      '@typescript-eslint/no-unused-vars': [
        'warn',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
          caughtErrors: 'all',
          caughtErrorsIgnorePattern: '^_',
        },
      ],
    },
  },

  {
    files: ['tests/**/*.ts'],
    languageOptions: {
      parser: tsparser,
      parserOptions: { sourceType: 'module', ecmaVersion: 'latest' },
      globals: {
        ...globals.node,
        ...globals.es2021,
        describe: 'readonly',
        it: 'readonly',
        test: 'readonly',
        expect: 'readonly',
        beforeEach: 'readonly',
        afterEach: 'readonly',
        beforeAll: 'readonly',
        afterAll: 'readonly',
      },
    },
    plugins: { '@typescript-eslint': tseslint, prettier },
    rules: {
      'prettier/prettier': 'error',
      'arrow-parens': [2, 'as-needed'],
      'no-var': 'error',
      'no-console': ['warn', { allow: ['error', 'warn'] }],
      'no-debugger': 'error',
      'no-useless-catch': 'warn',
      'no-empty': 'warn',
      'no-case-declarations': 'warn',
      'no-unused-vars': 'off',
      '@typescript-eslint/no-unused-vars': [
        'warn',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
          caughtErrors: 'all',
          caughtErrorsIgnorePattern: '^_',
        },
      ],
    },
  },
];
