# Iteration Summary

## Changed

- Added missing Evolution inbox and retry i18n messages in Chinese and English.
- Added locale coverage tests for every `evolution.*` key used by the Evolution page components.

## Impact

- The Evolution page no longer renders raw keys such as `evolution.inbox.all` when using the normal Vue i18n runtime.
- The focused regression test now catches future missing Evolution UI translation keys.
