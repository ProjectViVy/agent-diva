<script setup lang="ts">
import { onMounted, ref } from "vue";

const noiseImage = ref("");

onMounted(() => {
  const size = 640;
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = size;
  const context = canvas.getContext("2d");
  if (!context) return;
  const pixels = context.createImageData(size, size);
  let state = 271828;
  const random = () => {
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
    return state / 4294967296;
  };
  for (let i = 0; i < pixels.data.length; i += 4) {
    const choice = random();
    if (choice < .50) {
      pixels.data[i] = 255;
      pixels.data[i + 1] = 255;
      pixels.data[i + 2] = 244;
      pixels.data[i + 3] = 110 + Math.floor(random() * 120);
    } else if (choice < .87) {
      pixels.data[i] = 126;
      pixels.data[i + 1] = 70;
      pixels.data[i + 2] = 23;
      pixels.data[i + 3] = 55 + Math.floor(random() * 125);
    }
  }
  context.putImageData(pixels, 0, 0);
  noiseImage.value = `url("${canvas.toDataURL("image/png")}")`;
});
</script>

<template>
  <div class="atmosphere" aria-hidden="true">
    <div class="flow"></div>
    <div class="wash"></div>
    <div class="grain" :style="{ backgroundImage: noiseImage }"></div>
  </div>
</template>

<style scoped>
.atmosphere {
  position: absolute; inset: 0; overflow: hidden; pointer-events: none;
  background: radial-gradient(ellipse 62% 83% at 5% 15%, rgba(255,250,231,.98), transparent 77%),
    radial-gradient(ellipse 62% 73% at 66% 2%, rgba(247,164,139,.74), transparent 76%),
    radial-gradient(ellipse 59% 74% at 20% 96%, rgba(255,218,140,.85), transparent 84%),
    radial-gradient(ellipse 67% 78% at 97% 83%, rgba(244,172,184,.82), transparent 83%),
    linear-gradient(120deg, #fff2d3 0%, #f5b59f 50%, #f8cbd0 100%);
}
.flow { position: absolute; inset: -22%; opacity: .88; }
.flow::before, .flow::after { content: ""; position: absolute; inset: 0; will-change: transform; }
.flow::before { background: radial-gradient(ellipse 38% 45% at 24% 36%, rgba(255,249,220,.96), transparent 82%), radial-gradient(ellipse 42% 46% at 75% 55%, rgba(238,130,138,.52), transparent 81%); animation: flow-a 28s ease-in-out infinite alternate; }
.flow::after { background: radial-gradient(ellipse 39% 47% at 67% 19%, rgba(255,214,175,.7), transparent 83%), radial-gradient(ellipse 37% 41% at 35% 80%, rgba(255,249,220,.96), transparent 81%); animation: flow-b 37s ease-in-out infinite alternate; }
.wash { position: absolute; inset: 0; background: rgba(255, 252, 249, .52); }
.grain { position: absolute; inset: -3%; background-repeat: repeat; background-size: 640px 640px; opacity: .82; mix-blend-mode: soft-light; animation: grain-drift 46s ease-in-out infinite alternate; will-change: transform; }
.grain::after { content: ""; position: absolute; inset: 0; background-image: inherit; background-size: inherit; background-repeat: inherit; opacity: .55; mix-blend-mode: multiply; }
@keyframes flow-a { from { transform: translate3d(-6%, -4%, 0) rotate(-7deg) scale(1.05); } to { transform: translate3d(8%, 6%, 0) rotate(6deg) scale(1.13); } }
@keyframes flow-b { from { transform: translate3d(6%, 4%, 0) rotate(7deg) scale(1.08); } to { transform: translate3d(-7%, -5%, 0) rotate(-6deg) scale(1.14); } }
@keyframes grain-drift { from { transform: translate3d(-.7%, -.8%, 0); } to { transform: translate3d(.7%, .8%, 0); } }
@media (prefers-reduced-motion: reduce) { .flow::before, .flow::after, .grain { animation: none; } }
</style>
