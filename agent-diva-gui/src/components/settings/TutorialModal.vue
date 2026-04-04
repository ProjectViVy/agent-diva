<script setup lang="ts">
import { ref, watch, computed } from 'vue';
import { X, BookOpen, LoaderCircle } from 'lucide-vue-next';
import type { ChannelPlatformInfo } from './channel-platforms';

const props = defineProps<{
  open: boolean;
  platform: ChannelPlatformInfo | null;
}>();

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void;
  (e: 'start-config'): void;
}>();

const tutorialContent = ref<string>('');
const isLoading = ref(false);

// 平台概览信息
const difficultyStars = computed(() => {
  if (!props.platform) return '';
  return '⭐'.repeat(props.platform.difficulty);
});

const publicIPRequired = computed(() => {
  if (!props.platform) return '';
  return props.platform.requiresPublicIP ? '需要' : '不需要';
});

// 加载教程 Markdown 文件
async function loadTutorial() {
  if (!props.platform) return;
  
  isLoading.value = true;
  try {
    // 尝试从 public 目录加载 Markdown 文件
    const response = await fetch(props.platform.tutorialPath);
    if (response.ok) {
      tutorialContent.value = await response.text();
    } else {
      // 如果文件不存在，显示占位内容
      tutorialContent.value = generatePlaceholderContent();
    }
  } catch (error) {
    console.warn('Failed to load tutorial:', error);
    tutorialContent.value = generatePlaceholderContent();
  } finally {
    isLoading.value = false;
  }
}

// 生成占位内容（当教程文件不存在时）
function generatePlaceholderContent(): string {
  if (!props.platform) return '';
  
  return `
## 平台概览

- **接入方式**: ${props.platform.accessMethod}
- **需要公网 IP**: ${publicIPRequired.value}
- **配置难度**: ${difficultyStars.value}

## 前置条件

- 准备相关平台的开发者账号
- 确保网络环境正常

## 平台端申请步骤

请参考相关平台的官方文档完成应用创建和凭证获取。

## Agent Diva 配置

1. 在上方表单中填写凭证信息
2. 点击"下一步"进行测试
3. 测试通过后完成配置

## 验证与测试

- 启动服务后观察日志输出
- 发送测试消息验证连接

## 常见问题

如有问题，请查看项目文档或提交 Issue。
`;
}

// 开始配置
function startConfiguration() {
  emit('update:open', false);
  emit('start-config');
}

// 监听 platform 变化
watch(
  () => props.platform,
  (newPlatform) => {
    if (newPlatform) {
      loadTutorial();
    }
  },
  { immediate: true }
);

// 监听 open 状态
watch(
  () => props.open,
  (newOpen) => {
    if (newOpen && props.platform) {
      loadTutorial();
    }
  }
);
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="open" class="tutorial-overlay" @click.self="emit('update:open', false)">
        <div class="tutorial-modal">
          <!-- Header -->
          <div class="tutorial-header">
            <div class="tutorial-title-wrapper">
              <BookOpen :size="20" />
              <h2 class="tutorial-title">
                {{ platform?.displayName }} 配置教程
              </h2>
            </div>
            <button class="tutorial-close" @click="emit('update:open', false)">
              <X :size="18" />
            </button>
          </div>

          <!-- Content -->
          <div class="tutorial-content">
            <!-- Platform Overview -->
            <div v-if="platform" class="platform-overview">
              <div class="overview-item">
                <span class="overview-label">接入方式</span>
                <span class="overview-value">{{ platform.accessMethod }}</span>
              </div>
              <div class="overview-item">
                <span class="overview-label">需要公网 IP</span>
                <span class="overview-value">{{ publicIPRequired }}</span>
              </div>
              <div class="overview-item">
                <span class="overview-label">配置难度</span>
                <span class="overview-value">{{ difficultyStars }}</span>
              </div>
            </div>

            <!-- Tutorial Body -->
            <div class="tutorial-body markdown-body" v-if="!isLoading">
              <div v-html="tutorialContent"></div>
            </div>

            <!-- Loading State -->
            <div v-else class="tutorial-loading">
              <LoaderCircle :size="24" class="animate-spin" />
              <p>加载教程中...</p>
            </div>
          </div>

          <!-- Footer -->
          <div class="tutorial-footer">
            <button class="tutorial-btn tutorial-btn-secondary" @click="emit('update:open', false)">
              关闭
            </button>
            <button class="tutorial-btn tutorial-btn-primary" @click="startConfiguration">
              开始配置
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.tutorial-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.tutorial-modal {
  background: var(--panel-solid);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  width: 100%;
  max-width: 900px;
  max-height: 90vh;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.tutorial-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1.25rem 1.5rem;
  border-bottom: 1px solid var(--line);
}

.tutorial-title-wrapper {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  color: var(--accent);
}

.tutorial-title {
  font-size: 1.125rem;
  font-weight: 600;
  color: var(--text);
  margin: 0;
}

.tutorial-close {
  width: 32px;
  height: 32px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
}

.tutorial-close:hover {
  background: var(--accent-bg-light);
  color: var(--accent);
}

.tutorial-content {
  flex: 1;
  overflow-y: auto;
  padding: 1.5rem;
}

.platform-overview {
  display: flex;
  gap: 1.5rem;
  padding: 1rem;
  background: var(--panel);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  margin-bottom: 1.5rem;
}

.overview-item {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.overview-label {
  font-size: 0.75rem;
  color: var(--text-muted);
  font-weight: 500;
}

.overview-value {
  font-size: 0.875rem;
  color: var(--text);
  font-weight: 600;
}

.tutorial-body {
  line-height: 1.7;
}

.markdown-body {
  color: var(--text);
}

.markdown-body :deep(h1) {
  font-size: 1.5rem;
  font-weight: 600;
  margin-top: 1.5rem;
  margin-bottom: 1rem;
  color: var(--text);
}

.markdown-body :deep(h2) {
  font-size: 1.25rem;
  font-weight: 600;
  margin-top: 1.25rem;
  margin-bottom: 0.75rem;
  color: var(--text);
}

.markdown-body :deep(h3) {
  font-size: 1.125rem;
  font-weight: 600;
  margin-top: 1rem;
  margin-bottom: 0.5rem;
  color: var(--text);
}

.markdown-body :deep(p) {
  margin-bottom: 1rem;
  line-height: 1.7;
}

.markdown-body :deep(ul),
.markdown-body :deep(ol) {
  margin-bottom: 1rem;
  padding-left: 1.5rem;
}

.markdown-body :deep(li) {
  margin-bottom: 0.5rem;
}

.markdown-body :deep(code) {
  background: var(--panel);
  padding: 0.125rem 0.375rem;
  border-radius: 4px;
  font-family: 'Courier New', monospace;
  font-size: 0.875em;
  color: var(--accent);
}

.markdown-body :deep(pre) {
  background: var(--panel);
  padding: 1rem;
  border-radius: var(--radius-sm);
  overflow-x: auto;
  margin-bottom: 1rem;
}

.markdown-body :deep(pre code) {
  background: transparent;
  padding: 0;
}

.markdown-body :deep(table) {
  width: 100%;
  border-collapse: collapse;
  margin-bottom: 1rem;
}

.markdown-body :deep(th),
.markdown-body :deep(td) {
  border: 1px solid var(--line);
  padding: 0.5rem 0.75rem;
  text-align: left;
}

.markdown-body :deep(th) {
  background: var(--panel);
  font-weight: 600;
}

.markdown-body :deep(blockquote) {
  border-left: 3px solid var(--accent);
  padding-left: 1rem;
  margin: 1rem 0;
  color: var(--text-muted);
}

.tutorial-loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 3rem 0;
  gap: 1rem;
  color: var(--text-muted);
}

.tutorial-footer {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  padding: 1.25rem 1.5rem;
  border-top: 1px solid var(--line);
}

.tutorial-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.625rem 1.25rem;
  border-radius: var(--radius-sm);
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.tutorial-btn-primary {
  background: var(--accent);
  color: white;
  border: none;
}

.tutorial-btn-primary:hover {
  filter: brightness(1.1);
}

.tutorial-btn-secondary {
  background: var(--panel);
  color: var(--text);
  border: 1px solid var(--line);
}

.tutorial-btn-secondary:hover {
  background: var(--accent-bg-light);
}

/* Modal transitions */
.modal-enter-active,
.modal-leave-active {
  transition: all 0.2s ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-from .tutorial-modal,
.modal-leave-to .tutorial-modal {
  transform: scale(0.95) translateY(-20px);
}
</style>
