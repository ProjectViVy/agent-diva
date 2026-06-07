//! CC-P6-Real: 真实文本压缩效果验证测试
//!
//! 本测试使用真实的 LLM provider（teakacloud）对一段模拟对话进行压缩，
//! 并输出压缩前后的对比，验证压缩的实际效果。
//!
//! 运行方式：
//!   cargo test -p agent-diva-agent test_real_compaction -- --nocapture
//!
//! 环境变量（可选）：
//!   TEAKACLOUD_API_KEY  - API key（默认使用配置中的）
//!   COMPACTION_TEST_MODEL - 使用的模型（默认：MiniMax-M3）

use agent_diva_agent::compaction::ContextCompactor;
use agent_diva_agent::context_budget::{self, BudgetConfig};
use agent_diva_agent::token_estimate::{estimate_tokens, estimate_total_tokens};
use agent_diva_core::session::{ChatMessage, CompactTrigger, Session};
use agent_diva_providers::{LLMProvider, LiteLLMClient};
use std::sync::Arc;

/// 构造一段真实的中文对话（模拟项目开发场景）
fn build_realistic_conversation() -> Vec<(String, String)> {
    vec![
        ("user".to_string(), "你好，我想请你帮我设计一个 Rust 项目的架构。这个项目是一个 AI Agent 的桌面应用，需要支持多平台（Windows、macOS、Linux）。".to_string()),
        ("assistant".to_string(), "好的！基于你的需求，我建议采用以下架构：\n\n1. **后端核心**：使用 Rust 编写，利用其内存安全和高性能特性\n2. **桌面框架**：Tauri（基于 WebView）—— 比 Electron 更轻量，包体积小 90%\n3. **前端**：Vue 3 + TypeScript，配合 Tailwind CSS\n4. **AI 集成**：通过 LiteLLM 统一调用多种 LLM provider\n5. **状态管理**：使用 SQLite 做持久化，支持会话历史\n\n你觉得这个方向可以吗？".to_string()),
        ("user".to_string(), "可以，但我有几个问题：\n1. Tauri 的 IPC 通信会不会有性能瓶颈？\n2. 多平台打包怎么配置？\n3. 我需要支持插件系统，怎么设计？".to_string()),
        ("assistant".to_string(), "很好的问题！我来逐一回答：\n\n**1. IPC 性能**\nTauri 使用自定义协议进行 IPC，实测延迟 < 1ms。对于高频通信，建议：\n- 使用 `invoke` 而不是 `event` 进行请求/响应式通信\n- 大数据传输使用文件系统或共享内存\n- 批量操作合并为单次 IPC 调用\n\n**2. 多平台打包**\n```yaml\n# tauri.conf.json 配置示例\n{\"tauri\": {\"bundle\": {\"targets\": [\"msi\", \"dmg\", \"deb\"]}}}\n```\nCI/CD 使用 GitHub Actions，矩阵构建三个平台。\n\n**3. 插件系统**\n建议采用 WASM 插件方案：\n- 主进程提供 Host API（文件、网络、通知）\n- 插件编译为 WASM 模块，通过 wasmer 运行时加载\n- 沙盒隔离，防止插件访问敏感资源\n\n需要我详细展开哪个部分？".to_string()),
        ("user".to_string(), "WASM 插件方案听起来不错，但我担心调试体验。有没有更好的方案？".to_string()),
        ("assistant".to_string(), "理解你的顾虑。WASM 调试确实比较痛苦。另一个方案是 **动态链接库（DLL/so/dylib）**：\n\n**优点**：\n- 原生性能，无 WASM 开销\n- 可以使用标准调试工具（GDB、LLDB）\n- 插件可以直接调用 Rust API\n\n**缺点**：\n- ABI 兼容性问题（不同 Rust 版本可能不兼容）\n- 安全性较低（插件可以访问内存）\n- 跨平台构建更复杂\n\n**折中方案**：\n1. 开发阶段：使用 DLL 方便调试\n2. 发布阶段：切换到 WASM 增强安全性\n3. 提供统一的 Plugin trait，底层实现可以切换\n\n```rust\npub trait Plugin: Send + Sync {\n    fn name(&self) -> &str;\n    fn version(&self) -> &str;\n    fn on_load(&mut self, ctx: &PluginContext);\n    fn on_message(&mut self, msg: &Message) -> Option<Message>;\n}\n```\n\n这样你可以先实现 DLL 版本，后续无缝迁移到 WASM。".to_string()),
        ("user".to_string(), "这个折中方案不错！那插件的权限管理怎么做？我不想让插件随意读写文件。".to_string()),
        ("assistant".to_string(), "权限管理是插件系统的核心。我建议采用 **Capability-based 权限模型**：\n\n```rust\npub struct PluginCapabilities {\n    pub filesystem: FileSystemCapability,\n    pub network: NetworkCapability,\n    pub process: ProcessCapability,\n}\n\npub struct FileSystemCapability {\n    pub read_paths: Vec<PathBuf>,\n    pub write_paths: Vec<PathBuf>,\n    pub max_file_size: usize,\n}\n```\n\n**实现方式**：\n1. 插件 manifest 声明需要的权限\n2. 用户安装时确认权限列表\n3. 运行时通过沙盒拦截系统调用\n4. 敏感操作需要二次确认\n\n**具体实现**：\n- 文件操作：使用 `tokio::fs` 的包装器，检查路径白名单\n- 网络请求：通过代理转发，记录日志\n- 进程执行：完全禁止或限制为特定命令\n\n这样即使插件有漏洞，也无法突破权限边界。".to_string()),
        ("user".to_string(), "明白了。那现在帮我实现一个基础的插件加载器吧，能加载 DLL 就行，权限检查后面再加。".to_string()),
        ("assistant".to_string(), "好的，我们来实现一个基础的 DLL 插件加载器。首先创建项目结构：\n\n```\nplugin-system/\n├── Cargo.toml\n├── src/\n│   ├── lib.rs\n│   ├── loader.rs\n│   └── manifest.rs\n└── examples/\n    └── hello_plugin/\n        ├── Cargo.toml\n        └── src/\n            └── lib.rs\n```\n\n**核心代码** (`loader.rs`)：\n\n```rust\nuse std::path::Path;\nuse libloading::{Library, Symbol};\n\npub struct PluginLoader {\n    plugins: Vec<LoadedPlugin>,\n}\n\nstruct LoadedPlugin {\n    name: String,\n    library: Library,\n}\n\nimpl PluginLoader {\n    pub fn new() -> Self {\n        Self { plugins: vec![] }\n    }\n\n    pub unsafe fn load(&mut self, path: &Path) -> Result<(), PluginError> {\n        let lib = Library::new(path)?;\n        \n        // 获取插件信息\n        let info: Symbol<fn() -> PluginInfo> = lib.get(b\"plugin_info\")?;\n        let info = info();\n        \n        println!(\"加载插件: {} v{}\", info.name, info.version);\n        \n        self.plugins.push(LoadedPlugin {\n            name: info.name,\n            library: lib,\n        });\n        \n        Ok(())\n    }\n}\n\n#[derive(Debug)]\npub struct PluginInfo {\n    pub name: String,\n    pub version: String,\n}\n```\n\n这个实现使用了 `libloading` crate 来加载动态库。注意 `load` 方法是 `unsafe` 的，因为 FFI 边界无法保证安全。\n\n运行测试：\n```bash\ncargo test --example hello_plugin\n```\n\n需要我继续完善这个实现吗？".to_string()),
        ("user".to_string(), "可以，但我现在想先了解一下性能数据。这个插件系统的加载时间和内存占用大概是多少？".to_string()),
        ("assistant".to_string(), "我们来分析一下性能数据。基于我之前的测试：\n\n**加载时间**：\n- 小型插件（< 100KB）：~5-10ms\n- 中型插件（100KB - 1MB）：~20-50ms\n- 大型插件（> 1MB）：~100-200ms\n\n**内存占用**：\n- 基础开销（PluginLoader）：~2KB\n- 每个加载的插件：代码段 + 数据段\n- 实测：一个 500KB 的插件加载后 RSS 增加约 1-2MB\n\n**优化建议**：\n1. **延迟加载**：插件在首次使用时才加载\n2. **共享库**：多个插件依赖相同库时，使用动态链接\n3. **缓存**：已加载的插件保持内存驻留\n\n**对比 WASM**：\n- WASM 启动更慢（需要编译 JIT），但内存隔离更好\n- DLL 启动更快，但安全性较低\n\n你更关注哪个指标？我可以针对性优化。".to_string()),
        ("user".to_string(), "了解了。那我们现在来讨论一下项目的 CI/CD 流程吧。".to_string()),
        ("assistant".to_string(), "好的！对于 Rust + Tauri 项目，我建议以下 CI/CD 流程：\n\n**GitHub Actions 工作流**：\n\n```yaml\nname: Build and Release\n\non:\n  push:\n    tags: ['v*']\n\njobs:\n  build:\n    strategy:\n      matrix:\n        os: [ubuntu-latest, windows-latest, macos-latest]\n    runs-on: ${{ matrix.os }}\n    steps:\n      - uses: actions/checkout@v4\n      \n      - name: Setup Rust\n        uses: dtolnay/rust-action@stable\n        \n      - name: Setup Node\n        uses: actions/setup-node@v4\n        with:\n          node-version: 20\n          \n      - name: Install dependencies\n        run: |\n          npm install\n          cargo install tauri-cli\n          \n      - name: Build\n        run: cargo tauri build\n        \n      - name: Upload artifacts\n        uses: actions/upload-artifact@v4\n        with:\n          name: app-${{ matrix.os }}\n          path: src-tauri/target/release/bundle\n```\n\n**关键优化**：\n1. **缓存**：使用 `Swatinem/rust-cache` 缓存 cargo 依赖\n2. **并行构建**：三个平台同时构建\n3. **签名**：macOS 和 Windows 需要代码签名\n4. **自动发布**：构建成功后自动创建 GitHub Release\n\n**本地开发**：\n```bash\n# 一键启动开发环境\ncargo tauri dev\n\n# 热重载：修改前端代码自动刷新\n# Rust 代码修改后自动重新编译\n```\n\n需要我配置完整的 CI/CD 吗？".to_string()),
        ("user".to_string(), "好的，这个 CI/CD 流程看起来不错。那现在我们来讨论一下项目的文档结构吧。".to_string()),
        ("assistant".to_string(), "项目文档结构非常重要。我建议采用以下组织方式：\n\n```\ndocs/\n├── README.md                 # 项目概述、快速开始\n├── ARCHITECTURE.md           # 架构设计文档\n├── API.md                    # API 参考文档\n├── CONTRIBUTING.md           # 贡献指南\n├── CHANGELOG.md              # 版本变更记录\n├── runbooks/                 # 操作手册\n│   ├── deployment.md         # 部署指南\n│   ├── troubleshooting.md    # 故障排查\n│   └── plugin-development.md # 插件开发指南\n├── adr/                      # 架构决策记录\n│   ├── 001-use-tauri.md\n│   ├── 002-plugin-system.md\n│   └── 003-sqlite-storage.md\n└── diagrams/                 # 架构图\n    ├── system-overview.png\n    └── data-flow.png\n```\n\n**文档维护策略**：\n1. **代码即文档**：使用 `rustdoc` 生成 API 文档\n2. **自动化检查**：CI 中运行 `markdownlint` 和链接检查\n3. **版本控制**：文档与代码同步更新\n4. **定期审查**：每季度审查文档准确性\n\n**README 模板**：\n```markdown\n# Project Name\n\n[![CI](https://github.com/...)](https://github.com/...)\n[![License](https://img.shields.io/...)](...)\n\n## 快速开始\n\n### 前置条件\n- Rust 1.70+\n- Node.js 20+\n- pnpm\n\n### 安装\n```bash\ngit clone ...\ncd project\npnpm install\ncargo build\n```\n\n### 运行\n```bash\ncargo tauri dev\n```\n\n## 架构\n\n参见 [ARCHITECTURE.md](docs/ARCHITECTURE.md)\n\n## 贡献\n\n参见 [CONTRIBUTING.md](docs/CONTRIBUTING.md)\n\n## 许可证\n\nMIT\n```\n\n这样的结构清晰吗？需要调整吗？".to_string()),
        ("user".to_string(), "很好，文档结构很清晰。那我们现在来总结一下今天讨论的内容吧。".to_string()),
        ("assistant".to_string(), "好的，让我来总结一下我们今天讨论的内容：\n\n## 项目架构总结\n\n### 技术栈\n- **后端**：Rust（内存安全、高性能）\n- **桌面框架**：Tauri（轻量、跨平台）\n- **前端**：Vue 3 + TypeScript + Tailwind CSS\n- **AI 集成**：LiteLLM（多 provider 统一调用）\n- **数据库**：SQLite（会话持久化）\n\n### 核心功能\n1. **多平台支持**：Windows、macOS、Linux\n2. **插件系统**：DLL（开发）+ WASM（生产）双模式\n3. **权限管理**：Capability-based 模型\n4. **CI/CD**：GitHub Actions 矩阵构建\n5. **文档**：完整的文档体系\n\n### 待办事项\n- [ ] 实现基础插件加载器\n- [ ] 配置 CI/CD 流程\n- [ ] 完善权限管理系统\n- [ ] 编写插件开发指南\n- [ ] 性能基准测试\n\n### 关键决策\n1. 使用 Tauri 而非 Electron（包体积小 90%）\n2. 插件系统采用 DLL + WASM 双模式\n3. 数据库使用 SQLite（简单、可靠）\n4. CI/CD 使用 GitHub Actions（免费、成熟）\n\n还有什么需要补充或调整的吗？".to_string()),
    ]
}

/// 将对话转换为 Session
fn conversation_to_session(conversation: &[(String, String)]) -> Session {
    let mut session = Session::new("real-compaction-test");
    for (role, content) in conversation {
        session.add_message(role, content.clone());
    }
    session
}

/// 创建真实的 LiteLLM provider（连接到 teakacloud）
fn create_real_provider() -> Arc<LiteLLMClient> {
    // 从环境变量读取 API key，如果没有则使用默认值
    let api_key = std::env::var("TEAKACLOUD_API_KEY")
        .unwrap_or_else(|_| {
            // 从 Hermes config.yaml 读取 teakacloud 的 API key
            let config_path = dirs::home_dir()
                .unwrap_or_default()
                .join("AppData/Local/hermes/config.yaml");

            if let Ok(content) = std::fs::read_to_string(&config_path) {
                // 简单解析：查找 teakacloud 的 api_key
                let mut in_teakacloud = false;
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("- name: teakacloud") {
                        in_teakacloud = true;
                    } else if in_teakacloud && trimmed.starts_with("- name: ") {
                        in_teakacloud = false;
                    } else if in_teakacloud && trimmed.starts_with("api_key: ") {
                        if let Some(key) = trimmed.strip_prefix("api_key: ") {
                            return key.trim().to_string();
                        }
                    }
                }
            }

            panic!("无法找到有效的 API key。请设置 TEAKACLOUD_API_KEY 环境变量，或确保 Hermes config.yaml 中配置了 custom_providers.teakacloud.api_key")
        });

    println!("使用 API key: {}...", &api_key[..10.min(api_key.len())]);

    let provider = LiteLLMClient::new(
        Some(api_key),
        Some("http://api.tokenplan.fun:3000/v1".to_string()),
        std::env::var("COMPACTION_TEST_MODEL").unwrap_or_else(|_| "MiniMax-M3".to_string()),
        None,
        Some("teakacloud".to_string()),
        None,
    );

    Arc::new(provider)
}

/// 打印分隔线
fn print_separator(title: &str) {
    println!("\n{}", "=".repeat(80));
    println!("{}", title);
    println!("{}", "=".repeat(80));
}

#[tokio::test]
async fn test_real_compaction() {
    // ═══════════════════════════════════════════════════════════════════════
    // 阶段 1: 构建真实对话
    // ═══════════════════════════════════════════════════════════════════════
    println!("\n🚀 启动真实文本压缩测试...\n");

    let conversation = build_realistic_conversation();
    let session = conversation_to_session(&conversation);
    let config = BudgetConfig {
        max_tokens: 100_000,
        system_budget_ratio: 0.0,
        compact_threshold_ratio: 0.80,
        keep_recent_count: 10,
    };

    // 计算压缩前的信息
    let pre_history = session.get_history(50);
    let pre_message_count = pre_history.len();
    let pre_tokens = estimate_total_tokens(&pre_history);
    let pre_chars: usize = pre_history.iter().map(|m| m.content.len()).sum();

    print_separator("📋 压缩前信息");
    println!("消息数量: {}", pre_message_count);
    println!("总字符数: {}", pre_chars);
    println!("预估 token 数: {}", pre_tokens);

    // ═══════════════════════════════════════════════════════════════════════
    // 阶段 2: 执行压缩（使用真实 LLM）
    // ═══════════════════════════════════════════════════════════════════════
    print_separator("🤖 正在调用 LLM 进行压缩...");

    let provider = create_real_provider();

    let result = ContextCompactor::compact(
        &session,
        &config,
        provider.clone(),
        "MiniMax-M3",
        CompactTrigger::Auto,
        &session.compaction_history,
    )
    .await
    .expect("压缩应该成功");

    // ═══════════════════════════════════════════════════════════════════════
    // 阶段 3: 输出压缩结果
    // ═══════════════════════════════════════════════════════════════════════
    print_separator("📦 压缩结果");

    let summary = &result.summary;
    let summary_tokens = estimate_tokens(&summary.summary);
    let savings_ratio = 1.0 - (summary_tokens as f64 / pre_tokens as f64);

    println!("压缩 ID: {}", summary.compact_id);
    println!("创建时间: {}", summary.created_at);
    println!("触发方式: {:?}", summary.trigger);
    println!(
        "源消息范围: {} - {}",
        summary.source_range.start_index, summary.source_range.end_index
    );
    println!("保留最近消息数: {}", summary.kept_recent_count);
    println!("压缩前消息数: {}", summary.pre_compact_message_count);
    println!("压缩前预估 token: {}", summary.pre_compact_estimated_tokens);
    println!("质量评分: {:.2}", summary.quality_score.unwrap_or(0.0));
    println!("重试次数: {}", summary.retry_count);

    print_separator("📝 压缩后的摘要内容");
    println!("{}", summary.summary);

    // ═══════════════════════════════════════════════════════════════════════
    // 阶段 4: 压缩前后对比
    // ═══════════════════════════════════════════════════════════════════════
    print_separator("📊 压缩前后对比");

    println!(
        "\n{:<25} {:>12} {:>12} {:>12}",
        "指标", "压缩前", "压缩后", "变化"
    );
    println!("{}", "-".repeat(65));
    println!(
        "{:<25} {:>12} {:>12} {:>12}",
        "消息数量",
        pre_message_count,
        "1 (摘要)",
        format!("-{}", pre_message_count - 1)
    );
    println!(
        "{:<25} {:>12} {:>12} {:>12}",
        "字符数",
        pre_chars,
        summary.summary.len(),
        format!(
            "{:.1}%",
            (1.0 - summary.summary.len() as f64 / pre_chars as f64) * 100.0
        )
    );
    println!(
        "{:<25} {:>12} {:>12} {:>12}",
        "预估 token",
        pre_tokens,
        summary_tokens,
        format!("{:.1}%", savings_ratio * 100.0)
    );

    // ═══════════════════════════════════════════════════════════════════════
    // 阶段 5: 验证压缩质量
    // ═══════════════════════════════════════════════════════════════════════
    print_separator("✅ 质量验证");

    // 检查摘要是否包含关键信息
    let summary_lower = summary.summary.to_lowercase();
    let key_terms = [
        "rust", "tauri", "vue", "插件", "ci/cd", "架构", "设计", "文档",
    ];

    let mut found_terms = vec![];
    let mut missing_terms = vec![];

    for term in &key_terms {
        if summary_lower.contains(term) {
            found_terms.push(*term);
        } else {
            missing_terms.push(*term);
        }
    }

    println!(
        "找到的关键术语 ({}): {}",
        found_terms.len(),
        found_terms.join(", ")
    );
    if !missing_terms.is_empty() {
        println!(
            "缺失的关键术语 ({}): {}",
            missing_terms.len(),
            missing_terms.join(", ")
        );
    }

    // 检查摘要长度
    let summary_len = summary.summary.len();
    println!("\n摘要长度: {} 字符", summary_len);
    if summary_len > 2000 {
        println!("⚠️  警告: 摘要超过 2000 字符限制");
    } else if summary_len > 1000 {
        println!("✅ 摘要长度适中 ({} 字符)", summary_len);
    } else {
        println!("ℹ️  摘要较短 ({} 字符)，可能遗漏了部分信息", summary_len);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 阶段 6: 验证压缩后连续性
    // ═══════════════════════════════════════════════════════════════════════
    print_separator("🔗 连续性验证");

    // 模拟压缩后的对话上下文
    let mut compacted_session = Session::new("compacted-test");
    compacted_session
        .compaction_history
        .push(result.summary.clone());
    compacted_session.last_compacted = result.new_compacted_index;

    // 添加保留的最近消息
    let kept_messages = &session.messages[result.new_compacted_index..];
    for msg in kept_messages {
        compacted_session.add_message(&msg.role, &msg.content);
    }

    let post_history = compacted_session.get_history(50);
    let post_tokens = estimate_total_tokens(&post_history);

    println!("压缩后上下文 token 数: {}", post_tokens);
    println!("原始上下文 token 数: {}", pre_tokens);
    println!(
        "Token 节省: {:.1}%",
        (1.0 - post_tokens as f64 / pre_tokens as f64) * 100.0
    );

    // ═══════════════════════════════════════════════════════════════════════
    // 最终断言
    // ═══════════════════════════════════════════════════════════════════════
    print_separator("🎯 测试断言");

    assert!(!summary.summary.is_empty(), "摘要不应为空");
    assert!(summary.pre_compact_message_count > 0, "应压缩了部分消息");
    assert!(
        summary.quality_score.unwrap_or(0.0) >= 0.5,
        "质量评分应 >= 0.5"
    );

    // 至少包含部分关键术语
    let found_count = found_terms.len();
    println!(
        "找到的关键术语: {}/{} ({:.0}%)",
        found_count,
        key_terms.len(),
        found_count as f64 / key_terms.len() as f64 * 100.0
    );
    assert!(found_count >= 3, "应至少包含 3 个关键术语");

    println!("\n✅ 所有测试通过！");
}
