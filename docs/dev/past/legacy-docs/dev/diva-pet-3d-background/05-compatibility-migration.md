# 05 — 兼容性与迁移分析

## 1. 向后兼容

### 1.1 配置兼容

旧版本 `PetConfig` 不含场景字段 → 自动合并默认值：

```typescript
// pet-config.ts 加载逻辑
function loadConfig(): PetConfig {
  const raw = localStorage.getItem('diva-pet-config')
  const parsed = raw ? JSON.parse(raw) : {}
  return { ...DEFAULT_PET_CONFIG, ...parsed }
  // 旧配无 selectedGaussSceneId → 变成 'transparent'
  // 旧配无 gaussSceneList → 变成预设 4 项
}
```

### 1.2 运行时兼容

- `GaussSceneController` 处理 `'transparent'` → 仅阴影地面 (无 .spz 加载)
- sceneId 不匹配 → 自动使用默认高度/缩放
- .spz URL 无效 → SplatMesh 内部处理错误

### 1.3 UI 兼容

- DivaPetView: 新增按钮不破坏现有布局
- PetSettings: 新增 section 独立于现有区域
- 桌面覆盖层模式 (DesktopPetOverlay) **不受影响** (不在本次范围)

---

## 2. 迁移路径

### 2.1 用户无需手动迁移

| 场景 | 处理 |
|------|------|
| 旧用户首次升级 | 自动默认 `transparent`，行为不变 |
| 发现新功能 | DivaPetView 齿轮旁出现场景按钮 |
| 选择场景 | 配置持久化，重启后保留 |

### 2.2 配置文件版本

| 版本 | 场景字段 | 处理 |
|------|---------|------|
| ≤ 0.4.10 | 无 | 自动补默认值 |
| ≥ 0.4.11 | 有 | 正常读取 |

### 2.3 .spz 文件缺失

```
用户选 'home' → runtime.setBackgroundScene('home')
  → GaussSceneController.loadScene('home')
    → new SplatMesh({ url: '/vrm/scene/home.spz' })
      → 加载失败
        → controller.loadScene('transparent') ← 自动回退
```

用户看到: 背景透明，模型正常。Console: `warn: 场景设置失败, 已回退透明`

---

## 3. 跨平台

| 平台 | WebGL | GPU 要求 | 备注 |
|------|-------|---------|------|
| Windows (WebView2) | ✅ | 集成 GPU 即可 | Tauri 默认 |
| macOS (WKWebView) | ✅ | Metal 后端 | |
| Linux (WebKitGTK) | ✅ | 需 GPU 驱动 | 软件渲染性能差 |
| 虚拟机 (无 GPU) | ⚠️ | 自动回退 transparent | Three.js 降级 |

---

## 4. 回退计划

如需回退: 移除 `DivaVrmAvatar.vue` 中 `backgroundScene` prop + watch，移除 DivaPetView 场景按钮，保留 `@sparkjsdev/spark` (可选移除)。运行时层 GaussSceneController 不被调用 → 无影响。
