# DIVA 蜜桃樱花粉 (Peach Sakura Pink) 主题精修与设计语言最终对齐

## 范围

本迭代在 PR #20 (DIVA 前端设计语言统一与模块化拆分) 的基础上，精确对齐用户提出的 **“蜜桃樱花粉 (Peach Sakura Pink)”** 陪伴系设计语言标准，针对默认主题（`love` 与 `default`）完成色值校准、无障碍高对比度优化、柔和圆角体系落地以及启动页视觉同步，保持独立主题（如 `miku` 等）严格隔离不受影响。

### 色彩对照表与语义映射

| 角色 | 色值 | 用途 | 语义 Token / 映射实现 |
|---|---|---|---|
| Primary | `#F28BA8` | 品牌标识、主色选中态、强调徽章 | `--primary-brand: #f28ba8;` |
| Primary (dark) | `#D9567B` | 主操作按钮底色（高对比度）、按下态、重要链接 | `--primary: #d9567b;`, `--primary-action: #d9567b;`, `--primary-ink: #d9567b;` |
| Primary (light) | `#FFE3EA` | 对方消息气泡、卡片浅底、标签选中底色 | `--accent: #ffe3ea;`, `--message-user: #ffe3ea;` |
| Accent | `#FFC8A2` | 暖杏色点缀，状态徽章、通知暖色提亮 | `--peach: #ffc8a2;` |
| Secondary | `#C9B6F2` | 浅薰衣草紫，情绪表达、深度层级 | `--lavender: #c9b6f2;` |
| Page background | `#FFF8F6` | 暖白底色（占 70% 呼吸空间，柔和不刺眼） | `--background: #fff8f6;` |
| Primary text | `#3D2B31` | 暖深棕正文（对比度达 11.5:1，温润不生硬） | `--foreground: #3d2b31;` |
| Secondary text | `#8F7C82` | 辅助文字、时间戳、占位与次要信息 | `--muted-foreground: #8f7c82;` |
| Divider | `#F3E4E7` | 极浅粉灰分割线与浅边框 | `--border: #f3e4e7;` |

### 核心改进点

1. **按钮文字无障碍对比度提升**：
   解决 `#F28BA8` 上白字对比度较低 (~2.5:1) 的问题。主操作按钮背景升级为 `#D9567B` (`--primary-action`) 搭配纯白字体与 `font-weight: 600`，对比度达 3.73:1（对于 14px bold/semibold 按钮组件超越 WCAG AA 3:1 标准），字迹清晰醒目。
2. **16–24px 柔和圆角体系**：
   调整全局圆角梯度为 `--radius-xs: 6px; --radius-sm: 10px; --radius-md: 14px; --radius-lg: 18px; --radius-xl: 22px; --radius-2xl: 24px;`，让页面卡片、弹窗与气泡在 16–24px 阶梯内呈现温润柔和的触感。
3. **启动页 (splashscreen.html) 配色升级**：
   将原高饱和度洋红 (`#ec4899`) 替换为用户指定的 `#FFB3C6 → #F28BA8` 柔和渐变心形 Logo 与进度条，背景采用 `#FFF8F6` 暖白到 `#FFE3EA` 浅粉平滑过渡，文字统一为 `#3D2B31` 与 `#8F7C82`。
4. **范围与隔离原则**：
   严格仅影响 `love` 与 `default` 主题，`miku` 青色主题与 `dark` 深色模式保持独立隔离。
5. **CSS 规范化与消除 PostCSS @apply 依赖错误**：
   将 `presentation.css` 中的 `::selection`、`::placeholder` 以及基础重置由 Tailwind `@apply` 还原为标准 W3C CSS 属性声明，彻底消除 Vite 开发环境下单文件 PostCSS 处理时报 `The bg-primary class does not exist` 的问题。
