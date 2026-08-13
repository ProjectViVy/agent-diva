# Windows gateway bootstrap summary

The first post-clean-break real gateway start reproduced a main-thread stack
overflow. The Windows PE stack reserve and Tokio worker stack sizing were
incorrectly removed with the retired runtime even though current provider,
channel, SQLite and HTTP bootstrap still requires the general protection.

The CLI now restores a backend-neutral 16 MiB Windows main/worker stack without
restoring any removed dependency or product surface.
