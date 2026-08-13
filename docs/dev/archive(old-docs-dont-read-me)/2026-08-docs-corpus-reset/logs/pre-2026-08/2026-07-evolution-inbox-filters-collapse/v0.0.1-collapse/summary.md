## Iteration Completion Summary
- **What was changed**: 
  - Added a toggle button in ProposalInbox.vue to allow expanding and collapsing the proposal inbox filter section.
  - Added a filtersExpanded reactive reference (defaulting to true).
  - Added icons (ChevronUp, ChevronDown, Filter) to represent the toggle state.
  - Updated localization files (zh.ts, en.ts) to include strings for hideFilters and showFilters.
  - Added CSS for .proposal-inbox__filter-toggle to style the button correctly.
- **Impact range**: Evolution feature, Proposal Inbox component. Users can now collapse the search and filter controls to make more room for the proposal list.
