# Release

- No separate deployment step is required inside the repository.
- Monthly productionization becomes active when the updated manager runtime starts:
  - manager bootstrap auto-installs/updates the `Notebook Monthly Productionize` cron job
  - GUI manual monthly generation now calls the same manager/AutoDream trigger path
- Existing monthly markdown files remain compatible because output path stays `reports/monthly/{YYYY-MM}.md`.
