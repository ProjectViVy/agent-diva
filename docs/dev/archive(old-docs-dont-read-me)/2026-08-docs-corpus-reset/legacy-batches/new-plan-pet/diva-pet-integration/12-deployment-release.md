# 12 - 部署与发布方案

> Diva Pet 模块的构建、打包与分发策略

---

## 1. 构建流程

### 1.1 开发构建

```bash
# 进入 GUI 目录
cd agent-diva-gui

# 安装依赖（含 Live2D 相关）
pnpm install

# 启动 Tauri 开发模式（热重载）
pnpm tauri dev
```

### 1.2 生产构建

```bash
# 类型检查
pnpm typecheck

# 构建 Vite 前端
pnpm build

# 构建 Tauri 桌面应用
pnpm tauri build
```

构建产物：
```
agent-diva-gui/src-tauri/target/release/
├── agent-diva-gui.exe           # Windows 可执行文件
├── agent-diva-gui               # macOS/Linux 可执行文件
└── bundle/                      # 安装包输出
    ├── msi/                     # Windows MSI 安装包
    ├── nsis/                    # Windows NSIS 安装包
    └── dmg/                     # macOS DMG
```

### 1.3 资源打包配置

```json
// src-tauri/tauri.conf.json
{
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/icon.ico"
    ],
    "resources": {
      // 打包 Live2D 运行时和着色器
      "../public/live2d/cubism5/shaders/*": "live2d/cubism5/shaders/",
      "../public/live2d/cubism5/framework/*.js": "live2d/cubism5/framework/",
      // 打包默认 Live2D 模型
      "../live2d_resource/**/*": "live2d_resource/"
    },
    "windows": {
      "wix": {
        "language": "zh-CN"
      }
    }
  }
}
```

---

## 2. 发布前检查清单

### 2.1 代码质量

- [ ] `pnpm typecheck` 通过（零 TS 错误）
- [ ] `cargo clippy -- -D warnings` 通过
- [ ] `cargo fmt --check` 通过
- [ ] 单元测试通过（`pnpm test`）
- [ ] 集成测试通过
- [ ] 无 `@ts-ignore` / `@ts-nocheck`（除 vendor 脚本）

### 2.2 功能验证

- [ ] Live2D 角色在 Windows 10/11 正常渲染
- [ ] TTS（Browser 模式）正常播放
- [ ] TTS（SiliconFlow 模式）正常合成
- [ ] ASR（Web Speech）正常识别
- [ ] 模型切换功能正常
- [ ] 配置保存/加载正常
- [ ] 暗色主题正常
- [ ] 中英文切换正常
- [ ] Diva Pet 禁用后不影响 Chat 功能

### 2.3 性能验证

- [ ] 首帧渲染 < 3s
- [ ] 空闲帧率 ≥ 30fps
- [ ] 活跃帧率 ≥ 55fps
- [ ] 内存占用 < 200MB（含 Live2D）
- [ ] 包体积增量 < 3MB

### 2.4 许可合规

- [ ] Live2D Cubism SDK 许可确认
- [ ] `live2dcubismcore` npm 包许可确认
- [ ] 所有第三方依赖许可已记录
- [ ] README 中包含 Live2D 许可声明

### 2.5 文档

- [ ] CHANGELOG.md 已更新
- [ ] README 更新（新增 Diva Pet 功能介绍）
- [ ] 13 份开发文档已完成
- [ ] 用户文档（如何导入模型、配置语音）

---

## 3. 版本策略

### 3.1 版本号规则

```
agent-diva 遵循语义化版本 (SemVer)：
  MAJOR.MINOR.PATCH

Diva Pet 作为新增功能（Feature）：
  当前版本: 0.4.10
  发布版本: 0.5.0  (MINOR bump for new feature)
```

### 3.2 Release Notes 模板

```markdown
## [0.5.0] - 2026-05-XX

### Added
- **Diva Pet**: 新增桌宠模块，支持 Live2D 角色渲染
  - WebGL 驱动的 Cubism 5 Live2D 渲染
  - 角色表情与动作系统
  - 模型导入与切换功能
- **语音合成 (TTS)**: 新增多 Provider 语音播报
  - SiliconFlow CosyVoice2 (支持声音克隆)
  - OpenAI TTS
  - 浏览器 SpeechSynthesis 兜底
  - 自动降级链
- **语音识别 (ASR)**: 新增 Web Speech API 语音输入
  - 中文 (zh-CN) 支持
  - 自动重启与错误恢复

### Changed
- NormalMode 侧边栏新增 "Diva Pet" 入口
- config.json 新增 `pet` 配置段

### Known Issues
- ASR 在 Linux 平台的 zh-CN 支持有限
- Live2D 模型需为 Cubism 5 格式（.model3.json / .moc3）
```

---

## 4. 分发策略

### 4.1 分发渠道

| 渠道 | 分发物 | 用户 |
|------|--------|------|
| **GitHub Releases** | `.exe`, `.msi`, `.dmg` | 终端用户 |
| **cargo install** | CLI 版本（不含 GUI） | 开发者 |
| **源码构建** | `git clone` + `pnpm tauri build` | 贡献者 |

### 4.2 平台构建矩阵

| 平台 | CI Runner | 构建命令 | 产物 |
|------|-----------|----------|------|
| Windows x64 | `windows-latest` | `pnpm tauri build` | `.exe`, `.msi` |
| macOS x64 | `macos-latest` | `pnpm tauri build` | `.dmg` |
| macOS arm64 | `macos-latest` | `pnpm tauri build --target aarch64-apple-darwin` | `.dmg` |
| Linux x64 | `ubuntu-latest` | `pnpm tauri build` | `.deb`, `.AppImage` |

### 4.3 GitHub Actions CI 配置

```yaml
# .github/workflows/build-gui.yml 新增

jobs:
  build-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v2
        with:
          version: 9
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: 'pnpm'
          cache-dependency-path: agent-diva-gui/pnpm-lock.yaml

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install Dependencies
        working-directory: agent-diva-gui
        run: pnpm install

      - name: Build Tauri
        working-directory: agent-diva-gui
        run: pnpm tauri build

      - name: Upload Artifacts
        uses: actions/upload-artifact@v4
        with:
          name: agent-diva-windows
          path: |
            agent-diva-gui/src-tauri/target/release/bundle/msi/*.msi
            agent-diva-gui/src-tauri/target/release/bundle/nsis/*.exe
```

---

## 5. 升级与迁移

### 5.1 从旧版本升级

```
v0.4.x → v0.5.0 升级路径：
1. 安装新版本（覆盖安装）
2. 启动 Agent Diva GUI
3. config.json 自动兼容（无 pet section → 默认禁用 Diva Pet）
4. 用户手动启用：Settings → 勾选 "Diva Pet" → 配置 Live2D 模型路径

零数据丢失，零配置破坏。
```

### 5.2 降级路径

```
v0.5.0 → v0.4.x 降级：
1. 卸载 v0.5.0
2. 安装 v0.4.x
3. config.json 中的 `pet` section 被安全忽略（旧版本不识别新字段）

无副作用。
```

---

## 6. 监控与回滚

### 6.1 运行时指标

| 指标 | 收集方式 | 目的 |
|------|----------|------|
| Live2D 加载成功率 | `pet_load_success` / `pet_load_error` 事件计数 | 监控模型兼容性 |
| TTS 降级率 | `tts_fallback_count` 日志 | 监控 API 稳定性 |
| WebGL 可用率 | `webgl_available` 检测 | 监控兼容性 |
| 崩溃率 | Tauri Crash Reporter | 稳定性监控 |

### 6.2 紧急回滚指令

```bash
# 如果发布后出现严重问题，发布 hotfix 版本：
# v0.5.0 → v0.5.1，在 config.json 模板中设置 pet.enabled = false

# 或在 config.rs 中添加 feature flag：
# 紧急情况下可通过环境变量全局禁用 Diva Pet
export AGENT_DIVA_DISABLE_PET=true
```

---

## 7. 用户文档

### 7.1 README 新增内容

```markdown
## Diva Pet（桌宠）

Agent Diva 提供可选的桌宠角色功能：

### 快速开始

1. 启动 Agent Diva GUI
2. 左侧导航栏点击 **Diva Pet**
3. 导入 Live2D 模型（支持 Cubism 5 .model3.json 格式）
4. （可选）配置语音合成 API Key

### 语音功能

- **语音输出（TTS）**: 自动播报 Agent 回复
  - 默认使用系统语音（免费）
  - 支持 SiliconFlow CosyVoice2 高质量中文语音合成
- **语音输入（ASR）**: 点击麦克风按钮开始语音输入
  - 使用系统语音识别（Windows/macOS 内置）

### 自定义角色

支持导入自定义 Live2D 模型。模型需为 Cubism 5 格式（.model3.json + .moc3 + 贴图）。

### 许可说明

Live2D 渲染功能使用 [Live2D Cubism SDK](https://www.live2d.com/)。使用 Live2D 模型时，请遵守相应模型和 SDK 的许可条款。
```

---

## 8. 发布节奏

```
当前:    v0.4.10  (稳定)
         │
         ├─ v0.5.0-beta.1  (内部测试，Day 13)
         │   └─ 功能冻结，仅修 Bug
         │
         ├─ v0.5.0-rc.1    (候选发布，Day 14)
         │   └─ 全平台构建验证
         │
         └─ v0.5.0         (正式发布，Day 15)
             └─ CHANGELOG + Release Notes + 公告
```
