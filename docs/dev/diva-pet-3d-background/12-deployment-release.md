# 12 — 部署与发布方案

## 1. 构建

```bash
cd agent-diva/agent-diva-gui
pnpm install                    # 安装 @sparkjsdev/spark
npx vue-tsc --noEmit            # 类型检查
npm run test                    # 测试
npm run build                   # Vite 构建 → dist/
cd ..
just check && just test          # Rust 验证
```

## 2. 版本

建议 `0.4.11` (patch: 新增功能，不影响现有行为)。

CHANGELOG:
```markdown
## [0.4.11] - 2026-05-05

### Added
- DivaPetView 内嵌桌宠支持 3D 场景背景 (Gaussian Splatting)
- 预设场景: 室内(home)、海边(sea)、太空(space)
- DivaPetView 场景快速切换按钮
- PetSettings 场景配置区域
- 场景加载失败自动回退透明模式
```

## 3. 检查清单

**构建前**:
- [ ] `@sparkjsdev/spark` 在 package.json 中
- [ ] `.spz` 文件在 `public/vrm/scene/` 中
- [ ] `vue-tsc --noEmit` 通过
- [ ] `npm run test` 通过
- [ ] 手动验收通过 (参考 13-acceptance-criteria.md)

**构建后**:
- [ ] `dist/vrm/scene/` 含 3 个 .spz
- [ ] Tauri bundle 大小在可接受范围
- [ ] 安装/运行正常

## 4. 文件体积

| 文件 | 估 |
|------|-----|
| home.spz | 5-50 MB |
| sea.spz | 5-50 MB |
| space.spz | 3-30 MB |
| spark.module.js | ~200 KB |

如果体积超限: 提供按需下载、或使用更低精度压缩版本。

## 5. 回退

移除前端连接 (DivaVrmAvatar prop + DivaPetView 按钮 + PetSettings section)，运行时层不受影响。

## 6. Release Notes

```markdown
## Diva Pet 3D 背景场景

内嵌桌宠 (DivaPetView) 现支持 3D 背景场景:

- 🏠 室内场景
- 🌊 海边场景
- 🚀 太空场景

使用: DivaPetView 齿轮按钮旁的场景按钮 → 下拉选择
或在 设置 → Pet Settings → "📺 3D 背景场景" 中选择
```
