# 验收

## 本片完成

- 工具结果和入站消息不再把记录 id、邮箱、手机、卡号替换成 `[REDACTED:…]`。
- 模型可以用 `memory_list` 返回的原始 id 做 update / remove。

## 操作步骤

1. 重启 gateway / agent。
2. `memory_list`：id 完整，形如 `memory-17…-xxxxxxxxxxxx`。
3. 新增一条不同内容，新 id 同样完整。
4. 用该 id 做 update、remove，两次都要命中。
5. 对话里写邮箱或手机号，模型上下文中应看到原文，而不是 `[REDACTED:Email]`。

## 仍挂人工

`MEMORY-CRUD-ID-DESKTOP-SMOKE`。
