# Persona and Evolution Usability Summary

## Outcome

Persona now loads one authoritative workspace projection instead of coupling it to a second
proposal request. Structured desktop errors are rendered as useful messages, never
`[object Object]`, and a refresh failure keeps the last successful workspace visible.

JSON `null` persona authority is shown honestly as `tbd` with an editable `{}` draft. Pending
proposals are identified as not yet effective. Governance actions fail closed when their
authoritative projection is unavailable.

Evolution now treats proposals as primary data and polls events, health, feedback, runs, audit,
and policy independently. Auxiliary or refresh failures retain prior successful data.

Commit: `d6f82ea3 fix: restore persona and evolution workspaces`.
