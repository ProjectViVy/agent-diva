#!/bin/bash
# 本地代码审查脚本
# 用法: ./.claude/scripts/review.sh [文件路径]

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "🔍 代码预审查开始..."
echo ""

# 检查参数
if [ -z "$1" ]; then
    echo "用法: $0 <文件路径>"
    echo "例如: $0 agent-diva-files/src/index.rs"
    exit 1
fi

FILE="$1"

if [ ! -f "$FILE" ]; then
    echo -e "${RED}错误: 文件不存在: $FILE${NC}"
    exit 1
fi

echo "📄 审查文件: $FILE"
echo ""

# 1. 检查大文件读取问题
echo "🧠 检查内存安全问题..."
if grep -q "fs::read_to_string" "$FILE"; then
    echo -e "${YELLOW}  ⚠️  发现 fs::read_to_string，可能在大文件时导致 OOM${NC}"
    echo "     建议使用 BufReader 逐行读取"
fi

if grep -q "\.bytes()\.await" "$FILE"; then
    echo -e "${YELLOW}  ⚠️  发现 field.bytes().await，可能无限制读取 multipart 数据${NC}"
    echo "     建议添加 DefaultBodyLimit 限制"
fi

# 2. 检查数据库迁移问题
echo ""
echo "🗄️  检查数据库安全问题..."
if grep -q "ALTER TABLE.*ADD COLUMN" "$FILE"; then
    if ! grep -q "pragma_table_info\|table_info" "$FILE"; then
        echo -e "${YELLOW}  ⚠️  发现 ALTER TABLE ADD COLUMN，但没有检查列是否已存在${NC}"
        echo "     建议使用 PRAGMA table_info 检查后再执行"
    fi
fi

if grep -q "ON CONFLICT.*DO UPDATE" "$FILE"; then
    if ! grep -q "deleted_at = NULL" "$FILE"; then
        echo -e "${YELLOW}  ⚠️  ON CONFLICT UPDATE 可能没有清除软删除标记${NC}"
        echo "     建议设置 deleted_at = NULL, deleted_by = NULL"
    fi
fi

# 3. 检查 unwrap 使用
echo ""
echo "⚠️  检查错误处理..."
UNWRAP_COUNT=$(grep -c "\.unwrap()\|\.expect(" "$FILE" 2>/dev/null || echo 0)
if [ "$UNWRAP_COUNT" -gt 5 ]; then
    echo -e "${YELLOW}  ⚠️  发现 $UNWRAP_COUNT 处 unwrap/expect，建议适当处理错误${NC}"
fi

# 4. 检查 TODO/FIXME
echo ""
echo "📝 检查待办事项..."
grep -n "TODO\|FIXME\|XXX" "$FILE" 2>/dev/null | while read line; do
    echo -e "${YELLOW}  $line${NC}"
done

# 5. 检查新添加的 unsafe 代码
echo ""
echo "🔒 检查 unsafe 代码..."
if grep -q "unsafe" "$FILE"; then
    echo -e "${RED}  ⚠️  发现 unsafe 代码块，需要额外审查${NC}"
fi

echo ""
echo "✅ 基础检查完成"
echo ""
echo "💡 提示: 完整的审查清单见 .claude/scripts/pre-review-checklist.md"
