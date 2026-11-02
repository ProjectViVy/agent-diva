<script setup lang="ts">
import { debugOutput } from './debug';
</script>
<template>
  <div class="panel general-panel">
    <div class="setting">
      <div class="setting-copy">
        <h3>调试输出</h3>
        <p>在对话中显示工具调用记录与模型推理。关闭时只保留消息与失败结论。</p>
      </div>
      <button type="button" class="switch" role="switch" :aria-checked="debugOutput" aria-label="调试输出" @click="debugOutput=!debugOutput"><span class="thumb"></span></button>
    </div>
    <p class="hint">{{debugOutput?'已开启：工具与推理记录会在对话中显示。':'已关闭：工具与推理记录不会出现在对话中。'}}</p>
  </div>
</template>
<style scoped>
.setting{display:flex;align-items:flex-start;justify-content:space-between;gap:24px;padding:4px 0 18px;border-bottom:1px solid #a96f7c30}
.setting-copy h3{margin:0 0 6px;font-size:16px;font-weight:500;color:#6d3f4c}
.setting-copy p{margin:0;max-width:46ch;font-size:13px;line-height:1.7;color:#987b7a}
.switch{flex:0 0 auto;position:relative;width:46px;height:26px;padding:0;border:1px solid #a96f7c55;border-radius:999px;background:#f0d5d0;cursor:pointer;transition:background .2s ease,border-color .2s ease}
.switch .thumb{position:absolute;top:3px;left:3px;width:18px;height:18px;border-radius:50%;background:#fff;box-shadow:0 1px 4px rgba(122,63,80,.35);transition:transform .2s ease}
.switch[aria-checked="true"]{background:#b87886;border-color:#b87886}
.switch[aria-checked="true"] .thumb{transform:translateX(20px)}
.hint{margin:14px 0 0;font-size:12px;color:#a77a7b}
</style>
