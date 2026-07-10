# Summary

Windows gateway startup now disables the Mentle native `turso/simsimd` runtime path, which was causing `STATUS_STACK_OVERFLOW` during bootstrap. Laputa and Markdown memory providers remain available.

