# Phase 07.2.25 reduced-run execution

Initial screening: 5 conditions (control, x4, x10, x20, x50) x 4 seeds = 20 runs.
After examining results, optionally run 10 additional simulations. Default allocation: x50 +6, x20 +4, yielding 30 runs total.
The follow-up stage is **not automatic**.

From the project root:

```powershell
.\tools\run_phase07225.ps1 -Stage Pilot -DryRun
.\tools\run_phase07225.ps1 -Stage Pilot
# Only after reviewing Pilot results:
.\tools\run_phase07225.ps1 -Stage FollowUp -DryRun
.\tools\run_phase07225.ps1 -Stage FollowUp
```

To reallocate the 10 follow-up runs, e.g. 10 extra x10 runs:

```powershell
.\tools\run_phase07225.ps1 -Stage FollowUp -FollowUpCounts @{10=10}
```

Pilot seeds are StartSeed through StartSeed+3. FollowUp seeds start at StartSeed+4, so the new runs do not overlap the pilot. Existing completed runs are skipped when both CSVs exist. Both stages run the analysis with condition-specific expected run counts. `-DryRun` does not run cargo or analysis.

Caution: this is an exploratory screening with small samples, not confirmatory statistical evidence.
