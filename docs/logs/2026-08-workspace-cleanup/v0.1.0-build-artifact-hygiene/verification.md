# 验证记录

执行于 2026-08-21：

- `git check-ignore -v target agent-diva-gui/src-tauri/gen agent-diva-gui/src-tauri/resources/bin agent-diva-gui/src-tauri/resources/manifests`：全部命中根 `.gitignore` 的递归规则。
- `git ls-files --others --exclude-standard`：无输出，表示没有未跟踪且未忽略的文件。
- `git ls-files | rg "(^|/)(target|node_modules|dist|build|coverage)(/|$)"`：无输出，未发现已纳入版本控制的常见构建目录。
- `git check-ignore -v --no-index build.json build_utf8.json`：历史 Cargo 输出命中根 `.gitignore`；`build.json` 已从版本库移除。
- `git diff --check`：清理提交的暂存差异无空白错误。
- `just fmt-check`：通过。
- `just check`：通过（仅有既有 upstream future-incompatibility 提示）。
- `just test`：未完成；构建阶段无法替换被运行中的 `target\debug\agent-diva.exe` 和 `target\debug\agent-diva-gui.exe`，Windows 返回 `拒绝访问`。需停止占用进程后重跑。

本次只调整忽略规则和文档；测试阻塞来自本地进程锁，不涉及本次提交的源码。
