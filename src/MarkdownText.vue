<script setup lang="ts">
import { computed } from 'vue';
import MarkdownIt from 'markdown-it';
const props=defineProps<{text:string}>();
const markdown=new MarkdownIt({html:false,linkify:false,breaks:true});
const html=computed(()=>markdown.render(props.text));
</script>
<template><div class="markdown" v-html="html"></div></template>
<style scoped>
.markdown { min-width:0; overflow-wrap:anywhere; }
.markdown :deep(p:first-child) { margin-top:0; } .markdown :deep(p:last-child) { margin-bottom:0; }
.markdown :deep(pre) { max-width:100%; overflow:auto; padding:12px; background:#ffffff60; border-radius:8px; }
.markdown :deep(code) { font-family:Consolas,monospace; font-size:12px; }
.markdown :deep(table) { display:block; max-width:100%; overflow:auto; border-collapse:collapse; }
.markdown :deep(td),.markdown :deep(th) { padding:6px; border:1px solid #a96f7c40; }
</style>
