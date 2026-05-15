# 12 - VRM 部署与发布

> VRM 模块的构建、打包与分发策略

---

## 1. 构建流程

### 1.1 开发构建

```bash
cd agent-diva-gui
pnpm install                    # 安装含 VRM 依赖
pnpm tauri dev                  # 启动 Tauri 开发模式
```

### 1.2 生产构建

```bash
pnpm typecheck                  # TypeScript 检查
pnpm build                      # Vite 前端构建
pnpm tauri build                # Tauri 桌面应用打包
```

### 1.3 资源打包

```json
{
  "bundle": {
    "resources": {
      "../public/vrm/models/**/*.vrm": "vrm/models/",
      "../public/vrm/animations/**/*.vrma": "vrm/animations/"
    }
  }
}
```

---

## 2. 发布前检查清单

### 2.1 代码质量
- [ ] `pnpm typecheck` 零错误
- [ ] Vue 组件无 console.error
- [ ] WebGL context 正确释放

### 2.2 功能验证
- [ ] VRM 模型加载 < 5s
- [ ] 表情随对话变化
- [ ] 口型同步正常
- [ ] 模型切换不泄漏
- [ ] 低端 GPU 渲染不崩溃

### 2.3 许可合规
- [ ] three MIT ✅
- [ ] @pixiv/three-vrm MIT ✅
- [ ] 默认 VRM 模型许可明确
- [ ] README 含模型许可说明

---

## 3. 版本策略

```
当前: v0.4.10
  │
  └─ v0.5.0 (VRM + Live2D 桌宠功能)
      ├── 新增: VRM 3D 角色渲染
      ├── 新增: Live2D 2D 角色渲染
      └── 新增: ASR + TTS 语音交互
```

---

## 4. Release Notes 模板

```markdown
## [0.5.0] - 2026-05-XX

### Added
- **VRM 3D 角色**: 基于 @pixiv/three-vrm 的 3D 虚拟角色
  - 支持 VRM 0.x 和 VRM 1.0 模型
  - 表情自动推断（情绪关键词）
  - 口型同步（TTS 播报时）
  - 3D 视角旋转/缩放
  - 模型导入与切换
- **Live2D 2D 角色** (备选): 基于 Cubism SDK 5
- **语音**: ASR 语音输入 + TTS 语音播报

### Security
- VRM 方案使用 MIT 许可库，无专有 EULA 限制
```

---

## 5. 用户文档

```markdown
## 如何获取 VRM 模型

VRM 是一种开放的 3D 虚拟角色格式。你可以：

1. **VRoid Studio** (免费): https://vroid.com/studio
   - 制作你自己的 3D 角色
   - 导出为 .vrm 文件

2. **VRoid Hub**: https://hub.vroid.com/
   - 下载社区分享的 VRM 模型
   - 注意查看每个模型的许可条款

3. **Booth.pm**: https://booth.pm/
   - 购买/下载创作者制作的 VRM 模型

导入方法：设置 → Diva Pet → VRM → 导入模型 → 选择 .vrm 文件
```

---

## 6. CI 配置

```yaml
# .github/workflows/build-gui.yml
- name: Install dependencies
  run: pnpm install

- name: Build
  run: pnpm tauri build
  env:
    TAURI_PRIVATE_KEY: ${{ secrets.TAURI_PRIVATE_KEY }}
```
