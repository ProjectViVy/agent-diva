// subtitle-overlay.ts — 字幕浮层 composable
// 通过 Tauri event 接收字幕文本（从主窗口 TTS 流推送到 desktop-pet 窗口）

import { ref, onMounted, onUnmounted } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export interface SubtitleState {
  visible: boolean
  text: string
  isDragging: boolean
  position: { x: number; y: number }
}

export function useSubtitleOverlay() {
  const subtitle = ref<SubtitleState>({
    visible: false,
    text: '',
    isDragging: false,
    position: { x: 0, y: 0 },
  })

  let element: HTMLDivElement | null = null
  let dragOffset = { x: 0, y: 0 }
  const unlisteners: UnlistenFn[] = []

  function init(elementRef: HTMLDivElement): void {
    element = elementRef
  }

  function startDrag(event: PointerEvent): void {
    const el = element ?? (event.target as HTMLDivElement)
    subtitle.value.isDragging = true
    const rect = el.getBoundingClientRect()
    dragOffset = {
      x: event.clientX - rect.left,
      y: event.clientY - rect.top,
    }
  }

  function onDrag(event: PointerEvent): void {
    if (!subtitle.value.isDragging) return
    subtitle.value.position = {
      x: event.clientX - dragOffset.x,
      y: event.clientY - dragOffset.y,
    }
  }

  function endDrag(): void {
    subtitle.value.isDragging = false
  }

  function show(text: string): void {
    subtitle.value.text = text
    subtitle.value.visible = true
  }

  function hide(): void {
    subtitle.value.visible = false
    subtitle.value.text = ''
  }

  onMounted(async () => {
    // 初始位置：水平居中，底部 12%
    subtitle.value.position = {
      x: window.innerWidth / 2,
      y: window.innerHeight * 0.88,
    }

    // 通过 Tauri event 接收字幕（从主窗口 TTS 流推送）
    unlisteners.push(await listen<string>('desktop-pet-subtitle', (event) => {
      console.log('[SubtitleOverlay] desktop-pet-subtitle event received. payload:', JSON.stringify(event.payload)?.slice(0, 100))
      if (event.payload) {
        show(event.payload)
      } else {
        hide()
      }
    }))
  })

  onUnmounted(() => {
    unlisteners.forEach(fn => fn())
  })

  return {
    subtitle,
    init,
    startDrag,
    onDrag,
    endDrag,
  }
}
