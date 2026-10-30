import { defineConfig, presetUno, presetAttributify, presetIcons } from 'unocss'

export default defineConfig({
  presets: [
    presetUno(),
    presetAttributify(),
    presetIcons({
      scale: 1.2,
      warn: true,
    }),
  ],
  theme: {
    colors: {
      pcl: {
        primary: 'var(--pcl-color-primary)',
        hover: 'var(--pcl-color-hover)',
        active: 'var(--pcl-color-active)',
        bg: 'var(--pcl-color-bg)',
        card: 'var(--pcl-color-card)',
        cardHover: 'var(--pcl-color-card-hover)',
        border: 'var(--pcl-color-border)',
        text: 'var(--pcl-color-text)',
        subtext: 'var(--pcl-color-subtext)',
      },
    },
    boxShadow: {
      'pcl-card': '0 4px 16px -2px rgba(0, 0, 0, 0.08), 0 2px 6px -1px rgba(0, 0, 0, 0.04)',
      'pcl-hover': '0 8px 24px -4px rgba(0, 0, 0, 0.12), 0 4px 8px -2px rgba(0, 0, 0, 0.06)',
      'pcl-window': '0 12px 40px rgba(0, 0, 0, 0.25)',
    },
  },
})
