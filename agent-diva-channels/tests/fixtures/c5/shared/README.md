# Shared C5 adapter fixtures

Shared contract tests use private in-memory fakes rather than platform payloads or
network calls. Platform-specific wire fixtures belong under the owning directory:

`telegram/`, `discord/`, `feishu/`, `dingtalk/`, `email/`, and `qq/`.

The shared TCK currently proves:

- content-addressed `sha256:<lowercase-hex>` attachment identity;
- MIME, size, file-name, and digest validation;
- empty-list allow-all and wildcard/compound sender matching; and
- enabled-channel factory failure without a silent no-op.
