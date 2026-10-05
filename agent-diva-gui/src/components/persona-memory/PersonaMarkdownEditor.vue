<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { EditorState } from '@codemirror/state';
import { EditorView, highlightActiveLine, keymap, lineNumbers } from '@codemirror/view';
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
import { markdown } from '@codemirror/lang-markdown';

const props = withDefaults(defineProps<{ modelValue: string; readonly?: boolean }>(), {
  readonly: false,
});
const emit = defineEmits<{ (event: 'update:modelValue', value: string): void }>();
const host = ref<HTMLElement | null>(null);
let view: EditorView | null = null;

onMounted(() => {
  if (!host.value) return;
  view = new EditorView({
    parent: host.value,
    state: EditorState.create({
      doc: props.modelValue,
      extensions: [
        lineNumbers(),
        highlightActiveLine(),
        history(),
        keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
        markdown(),
        EditorState.readOnly.of(props.readonly),
        EditorView.lineWrapping,
        EditorView.updateListener.of((update) => {
          if (update.docChanged) emit('update:modelValue', update.state.doc.toString());
        }),
        EditorView.theme({
          '&': { height: '100%', backgroundColor: 'transparent', color: 'var(--text)' },
          '.cm-scroller': { overflow: 'auto', fontFamily: 'var(--font-mono, ui-monospace)' },
          '.cm-gutters': { backgroundColor: 'var(--panel-solid)', color: 'var(--text-muted)', border: 'none' },
          '.cm-activeLine, .cm-activeLineGutter': { backgroundColor: 'color-mix(in srgb, var(--accent) 8%, transparent)' },
          '.cm-content': { padding: '14px 4px' },
          '.cm-line': { padding: '0 12px' },
          '&.cm-focused': { outline: 'none' },
        }),
      ],
    }),
  });
});

watch(() => props.modelValue, (value) => {
  if (!view || value === view.state.doc.toString()) return;
  view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: value } });
});

onBeforeUnmount(() => view?.destroy());
</script>

<template><div ref="host" class="persona-markdown-editor" /></template>

<style scoped>
.persona-markdown-editor { height: 100%; min-height: 0; overflow: hidden; }
</style>
