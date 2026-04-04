<script setup lang="ts">
import { computed } from 'vue';
import { Plus, Import } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import ProviderCard from './ProviderCard.vue';

const { t } = useI18n();

interface ProviderCardItem {
  id: string;
  name: string;
  displayName: string;
  status: 'ready' | 'missingConfig' | 'active';
  currentModel?: string;
  apiBase?: string;
  isCustom?: boolean;
}

const props = defineProps<{
  providers: ProviderCardItem[];
  isLoading: boolean;
  activeProviderName?: string;
}>();

const emit = defineEmits<{
  (e: 'edit', provider: ProviderCardItem): void;
  (e: 'delete', provider: ProviderCardItem): void;
  (e: 'test', provider: ProviderCardItem): void;
  (e: 'create'): void;
  (e: 'import'): void;
}>();

const activeProvider = computed(() => 
  props.providers.find(p => p.name === props.activeProviderName)
);

const mapToCardProps = (provider: ProviderCardItem) => ({
  name: provider.name,
  displayName: provider.displayName,
  status: provider.name === props.activeProviderName ? 'active' : provider.status,
  currentModel: provider.currentModel,
  apiBase: provider.apiBase,
  isCustom: provider.isCustom,
});
</script>

<template>
  <div class="providers-card-view">
    <!-- Toolbar -->
    <div class="providers-toolbar">
      <div class="providers-toolbar-actions">
        <button class="providers-add-btn" @click="emit('create')">
          <Plus :size="16" />
          <span>{{ t('providers.createProviderAction') }}</span>
        </button>
        <button class="providers-import-btn" @click="emit('import')">
          <Import :size="16" />
          <span>{{ t('providers.importConfig') }}</span>
        </button>
      </div>
    </div>

    <!-- Loading State -->
    <div v-if="isLoading" class="providers-loading">
      <div class="providers-loading-spinner" />
      <p class="providers-loading-text">{{ t('providers.loading') }}</p>
    </div>

    <!-- Empty State -->
    <div v-else-if="providers.length === 0" class="providers-empty-state">
      <div class="providers-empty-icon">
        <Plus :size="40" />
      </div>
      <h3 class="providers-empty-title">{{ t('providers.emptyTitle') }}</h3>
      <p class="providers-empty-desc">{{ t('providers.emptyDesc') }}</p>
      <div class="providers-empty-actions">
        <button class="providers-empty-btn-primary" @click="emit('create')">
          <Plus :size="16" />
          <span>{{ t('providers.createProviderAction') }}</span>
        </button>
        <button class="providers-empty-btn-secondary" @click="emit('import')">
          <Import :size="16" />
          <span>{{ t('providers.importConfig') }}</span>
        </button>
      </div>
    </div>

    <!-- Cards Grid -->
    <div v-else class="providers-grid">
      <ProviderCard
        v-for="provider in providers"
        :key="provider.id"
        v-bind="mapToCardProps(provider)"
        @edit="emit('edit', provider)"
        @delete="emit('delete', provider)"
        @test="emit('test', provider)"
      />
    </div>
  </div>
</template>

<style scoped>
.providers-card-view {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.providers-toolbar {
  padding: 1rem 0;
  margin-bottom: 1rem;
}

.providers-toolbar-actions {
  display: flex;
  gap: 0.75rem;
}

.providers-add-btn,
.providers-import-btn {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.625rem 1rem;
  border-radius: var(--radius-sm);
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.providers-add-btn {
  background: var(--accent);
  color: white;
  border: none;
}

.providers-add-btn:hover {
  filter: brightness(1.1);
}

.providers-import-btn {
  background: var(--panel);
  color: var(--text);
  border: 1px solid var(--line);
}

.providers-import-btn:hover {
  background: var(--accent-bg-light);
  border-color: var(--accent);
  color: var(--accent);
}

.providers-loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 4rem 2rem;
}

.providers-loading-spinner {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  border: 3px solid var(--line);
  border-top-color: var(--accent);
  animation: spin 1s linear infinite;
  margin-bottom: 1rem;
}

.providers-loading-text {
  font-size: 0.875rem;
  color: var(--text-muted);
}

.providers-empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 4rem 2rem;
  text-align: center;
}

.providers-empty-icon {
  width: 80px;
  height: 80px;
  border-radius: 50%;
  background: var(--accent-bg-light);
  color: var(--accent);
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 1.5rem;
  opacity: 0.5;
}

.providers-empty-title {
  font-size: 1.125rem;
  font-weight: 600;
  color: var(--text);
  margin-bottom: 0.5rem;
}

.providers-empty-desc {
  font-size: 0.875rem;
  color: var(--text-muted);
  max-width: 400px;
  margin-bottom: 1.5rem;
}

.providers-empty-actions {
  display: flex;
  gap: 0.75rem;
}

.providers-empty-btn-primary,
.providers-empty-btn-secondary {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem 1.25rem;
  border-radius: var(--radius-sm);
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.providers-empty-btn-primary {
  background: var(--accent);
  color: white;
  border: none;
}

.providers-empty-btn-primary:hover {
  filter: brightness(1.1);
}

.providers-empty-btn-secondary {
  background: var(--panel);
  color: var(--text);
  border: 1px solid var(--line);
}

.providers-empty-btn-secondary:hover {
  background: var(--accent-bg-light);
  border-color: var(--accent);
  color: var(--accent);
}

.providers-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 1.5rem;
  padding: 0.5rem 0;
  overflow-y: auto;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
