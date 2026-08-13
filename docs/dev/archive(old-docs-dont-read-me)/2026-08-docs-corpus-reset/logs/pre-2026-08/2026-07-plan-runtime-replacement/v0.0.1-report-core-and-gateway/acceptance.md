# Acceptance

1. Creating a report persists canonical Markdown as revision 1.
2. Editing appends a new immutable revision and retains the original.
3. Approval requires the current revision hash and creates an execution session with the selected context policy.
4. The manager exposes report list, create, revision append, and approval endpoints.
