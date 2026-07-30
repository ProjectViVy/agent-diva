# Release

This slice is an internal policy-hardening release. It changes no public GUI,
Manager request, proposal, receipt or Memory DTO.

Release order:

1. deploy with the E7 integrated release after GMH-42;
2. run `just e7-automated-release-gate`;
3. only after every automated gate passes, enter deferred G2D+ desktop
   acceptance.

No push or production deployment is part of this slice.
