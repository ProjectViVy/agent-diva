<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";

const canvas = ref<HTMLCanvasElement | null>(null);
let observer: ResizeObserver | undefined;
let frame = 0;

onMounted(() => {
  const element = canvas.value;
  const context = element?.getContext("2d");
  if (!element || !context) return;

  const tile = document.createElement("canvas");
  tile.width = tile.height = 256;
  const tileContext = tile.getContext("2d");
  if (!tileContext) return;
  const pixels = tileContext.createImageData(tile.width, tile.height);
  for (let i = 0; i < pixels.data.length; i += 4) {
    const shade = Math.random() < 0.5 ? 0 : 255;
    pixels.data[i] = pixels.data[i + 1] = pixels.data[i + 2] = shade;
    pixels.data[i + 3] = Math.floor(Math.random() * 105);
  }
  tileContext.putImageData(pixels, 0, 0);
  const pattern = context.createPattern(tile, "repeat");
  if (!pattern) return;

  const draw = () => {
    const bounds = element.getBoundingClientRect();
    const scale = Math.min(window.devicePixelRatio || 1, 2);
    element.width = Math.max(1, Math.round(bounds.width * scale));
    element.height = Math.max(1, Math.round(bounds.height * scale));
    context.fillStyle = pattern;
    context.fillRect(0, 0, element.width, element.height);
  };
  observer = new ResizeObserver(() => {
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(draw);
  });
  observer.observe(element);
  draw();
});

onUnmounted(() => {
  observer?.disconnect();
  cancelAnimationFrame(frame);
});
</script>

<template>
  <div class="atmosphere" aria-hidden="true">
    <div class="light light-one"></div>
    <div class="light light-two"></div>
    <canvas ref="canvas" class="grain"></canvas>
  </div>
</template>

<style scoped>
.atmosphere { position: absolute; inset: 0; overflow: hidden; pointer-events: none; background: #f8d4aa; }
.light { position: absolute; inset: -30%; will-change: transform; }
.light-one { background: radial-gradient(ellipse at 23% 20%, #ffe6a1 0%, transparent 46%), radial-gradient(ellipse at 83% 72%, #f5a9ad 0%, transparent 48%), radial-gradient(ellipse at 75% 10%, #ffc486 0%, transparent 42%); animation: drift-one 24s ease-in-out infinite alternate; }
.light-two { background: radial-gradient(ellipse at 16% 85%, rgba(248, 169, 145, .75), transparent 43%), radial-gradient(ellipse at 58% 47%, rgba(255, 231, 169, .72), transparent 48%), radial-gradient(ellipse at 95% 14%, rgba(244, 167, 185, .65), transparent 38%); animation: drift-two 30s ease-in-out infinite alternate; }
.grain { position: absolute; inset: 0; width: 100%; height: 100%; mix-blend-mode: soft-light; opacity: .55; }
@keyframes drift-one { from { transform: translate3d(-2%, -2%, 0) scale(1); } to { transform: translate3d(3%, 2%, 0) scale(1.06); } }
@keyframes drift-two { from { transform: translate3d(2%, 2%, 0) scale(1.04); } to { transform: translate3d(-3%, -2%, 0) scale(1); } }
@media (prefers-reduced-motion: reduce) { .light { animation: none; } }
</style>
