# GMH-11 Acceptance

1. Confirm the evaluator is pure and has no runtime, database, Manager, or GUI
   dependency.
2. Confirm precedence is hard prohibition, explicit rejection, restriction,
   authorization, then safe default.
3. Confirm incompatible capability/resource pairs and unknown inputs deny.
4. Confirm L2 accepts only session/rule authorization, L3 accepts only exact
   once authorization, and critical actions require exact once authorization.
5. Confirm Plan, Memory, shell, filesystem, network, MCP, spawn, and schedule
   appear in the tested matrix.
6. Confirm the focused and complete workspace gates pass.
