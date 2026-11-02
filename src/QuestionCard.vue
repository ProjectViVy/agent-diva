<script setup lang="ts">
import { ref } from 'vue';
import { runtime } from './ipc';
import type { Question } from './generated/protocol';
const props=defineProps<{question:Question}>();
const text=ref('');const choices=ref<string[]>([]);const busy=ref(false);const error=ref('');
function choose(id:string) { choices.value=props.question.multiple ? (choices.value.includes(id)?choices.value.filter(c=>c!==id):[...choices.value,id]) : [id]; }
async function answer() {
  busy.value=true;error.value='';
  try { await runtime.request({method:'answer',params:{instance:runtime.state.instance,question:props.question.id,choices:choices.value,text:text.value||null}});await runtime.synchronize(); }
  catch(e){error.value=String(e);}finally{busy.value=false;}
}
</script>
<template>
  <section class="question"><strong>{{question.title}}</strong><p>{{question.body}}</p>
    <div class="choices"><button v-for="choice in question.choices" :key="choice.id" :aria-pressed="choices.includes(choice.id)" :disabled="busy" @click="choose(choice.id)">{{choice.label}}</button></div>
    <textarea v-if="question.free_text" v-model="text" aria-label="回答" :disabled="busy"></textarea>
    <p v-if="error" role="alert">{{error}}</p><button :disabled="busy||(!choices.length&&!text.trim())" @click="answer">{{busy?'正在提交…':'回答'}}</button>
  </section>
</template>
<style scoped>
.question{padding:16px;margin:16px 0;border:1px solid #a96f7c55;border-radius:12px;background:#fff8;}p{white-space:pre-wrap;overflow-wrap:anywhere;font-size:13px}.choices{display:flex;flex-wrap:wrap;gap:8px;margin:12px 0}button{border:1px solid #a96f7c55;border-radius:6px;padding:7px 10px;background:#fff6;color:inherit;cursor:pointer}button[aria-pressed=true]{background:#a96f7c;color:white}textarea{width:100%}
</style>
