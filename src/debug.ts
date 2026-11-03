import { ref, watch } from 'vue';
const key='agent-diva.debug-output';
function stored(){try{return localStorage.getItem(key)==='1';}catch{return false;}}
export const debugOutput=ref(stored());
watch(debugOutput,value=>{try{localStorage.setItem(key,value?'1':'0');}catch{/* storage unavailable: keep this session only */}});
