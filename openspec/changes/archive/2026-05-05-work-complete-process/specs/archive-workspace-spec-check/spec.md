## REMOVED Requirements

### Requirement: Archive preflight warns on incomplete OpenSpec tasks

**Reason**: Replaced by the richer Work Complete preflight, which not only detects incomplete OpenSpec tasks but also classifies the situation as the `SpecIncomplete` scenario, exposes the verbatim remaining task lines, and requires an explicit "Complete anyway" force-confirm before continuing. Spec linkage is no longer derived from a workspace-name heuristic; it uses the persisted `active_change_id` column with a worktree-scan fallback.

**Migration**: The behaviour is subsumed by the `work-complete-process` capability's `Preflight classifies scenario when spec tasks remain` requirement and its `Force-complete sub-flow for incomplete spec or open issue` requirement. The legacy `GET /workspaces/{id}/archive/preflight` endpoint that hosted this check is being deleted; consumers SHALL switch to `GET /workspaces/{id}/work-complete/preflight`.

### Requirement: Archive preflight warns on incomplete Specify spec tasks

**Reason**: The Specify (`.specify/specs/<name>/tasks.md`) detection path is similarly subsumed. The Work Complete preflight handles spec context via the OpenSpec linkage; Specify support, if reintroduced, would be a follow-up extension to the new flow rather than a parallel preflight branch. Specify support is not in scope for v1 of Work Complete.

**Migration**: Workspaces previously relying on the Specify-tasks warning will not see an equivalent warning in the v1 Work Complete flow. A future change MAY add a `specify-context-links` capability mirroring `workspace-context-links` for `.specify/specs/<id>/tasks.md`. The legacy archive preflight endpoint that hosted this check is being deleted.
