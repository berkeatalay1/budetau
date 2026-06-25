import vue from 'eslint-plugin-vue'
import vueTsConfig from '@vue/eslint-config-typescript'
import prettierConfig from '@vue/eslint-config-prettier'

export default [
  {
    ignores: ['dist/**', 'src-tauri/**', 'node_modules/**', 'src/vite-env.d.ts'],
  },
  ...vue.configs['flat/recommended'],
  ...vueTsConfig(),
  prettierConfig,
]
