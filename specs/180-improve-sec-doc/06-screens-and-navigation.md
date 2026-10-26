# 06 — Screens and navigation

[Index](README.md) · Behavior: [Product](03-product-spec.md) · Visual/interaction contract: [UX/UI](05-ux-ui-spec.md) · State diagrams: [Security](10-security-and-sequences.md)

These are proposed ASCII wireframes, not screenshots of implemented functionality. Desktop boxes use a fixed 76-column grid; mobile uses 44 columns. The visual implementation follows the gr8r-studio mapping in [UX/UI](05-ux-ui-spec.md).

## Navigation

```ascii
Documents
  +-- Upload files / drop / keyboard choose
  |     +-- Review upload (local queue)
  |           +-- Shared access fields
  |           +-- Per-file override -> same access fields
  |           +-- Upload -> queued/processing -> saved properties
  +-- Select document -> preview drawer -> Access properties
  |     +-- Edit -> preview -> review broadening if needed -> Save
  +-- Select 1-100 documents -> Edit access -> bulk preview -> Apply
  +-- Needs review filter -> visible unverified documents
Settings -> Workspace -> Access
  +-- Govern security tags / profiles / member assignments
  +-- Return to originating draft, revalidate epoch and selection
```

Settings is optional administration, not a required hop for ordinary labeling. Return intent is in memory and scoped; no labels or file names in navigation URLs. If the page unloads, ask the user to reselect local files and do not claim their upload draft was persisted.

## W-180-01

Library and upload entry. [R-180-01](03-product-spec.md#r-180-01), [T-180-01](12-e2e-test-matrix.md#t-180-01).

```ascii
+--------------------------------------------------------------------------+
| Documents / Legal                              [Upload files]            |
| [Search documents...]  [Sensitivity v] [Security tags v] [Needs review]  |
|                                                                          |
| + Drop files or choose them                                              |
| Access for this upload  [Needs review v] [Add security tags +]           |
| You and authorized security stewards, until reviewed.                    |
| Parser and extraction settings  [Workspace default v]                    |
|                                                                          |
| Name                    Status         Sensitivity       Security tags   |
| Employment.pdf          Completed      Confidential      HR  Atlas +1    |
| Annual-report.pdf       Processing     Needs review      None            |
|                                                                          |
| Selecting files opens review. Access stays visible in compact layouts.   |
+--------------------------------------------------------------------------+
```

## W-180-02

Upload review: defaults and per-file override. [R-180-01/08](03-product-spec.md), [T-180-25](12-e2e-test-matrix.md#t-180-25).

```ascii
+--------------------------------------------------------------------------+
| Review upload                                             [Close]        |
| 3 files selected    Workspace: Legal                                     |
|                                                                          |
| Access for all files                                                     |
| Sensitivity       [Confidential v]                                       |
| Security tags     [HR x] [Atlas x] [+ Add]                               |
| Who can access    Confidential or higher AND HR AND Atlas.               |
|                                                                          |
| File                      Access                         Action          |
| Employment.pdf            Uses batch access              [Customize]     |
| Handbook.md               Internal / HR                  [Reset]         |
| Expenses.png              Uses batch access              [Customize]     |
|                                                                          |
| Handbook.md has a per-file override. Shared changes preserve it.         |
|                                                [Cancel] [Upload 3 files] |
+--------------------------------------------------------------------------+
```

## W-180-03

Searchable governed security tags. [R-180-06](03-product-spec.md#r-180-06), [T-180-06](12-e2e-test-matrix.md#t-180-06).

```ascii
+--------------------------------------------------------------------------+
| Required security tags                                                   |
| [Search security tags...                                      ]          |
| Selected 2                         Readers need EVERY selected tag.      |
| [x] HR                         Department                                |
| [x] Atlas                      Project                                   |
|                                                                          |
| Available                                                                |
| [ ] Finance                    Department                                |
| [ ] Legal                      Department                                |
| [ ] Research                   Department                                |
| [Load more]                                                              |
|                                                                          |
| Empty search: No matching security tags. Clear search.                   |
| Catalog error: Could not load security tags. [Retry]                     |
| Manager only: [Manage security tags in Settings]                         |
|                                                         [Done]           |
+--------------------------------------------------------------------------+
```

## W-180-04

Document properties beside preview. [R-180-02](03-product-spec.md#r-180-02), [T-180-02](12-e2e-test-matrix.md#t-180-02).

```ascii
+--------------------------------------------------------------------------+
| Employment.pdf                           [Expand] [More] [Close]         |
|                                                                          |
| Document preview                 Properties                              |
|                                  Sensitivity   [Confidential v]          |
|   PDF / Markdown                 Security tags [HR] [Atlas] [+]          |
|   stays in context                                                       |
|                                  Who can access                          |
|                                  Confidential or higher, HR AND Atlas.   |
|                                  Authorized stewards may also qualify.   |
|                                                                          |
| Access editor                                                            |
| Sensitivity       [Restricted v]                 Unsaved changes         |
| Security tags     [HR x] [Atlas x] [+ Add]                               |
| Preview           Checking access... / Ready                             |
|                                            [Cancel] [Save access]        |
+--------------------------------------------------------------------------+
```

## W-180-05

Broadening review and explicit reason. [R-180-10](03-product-spec.md#r-180-10), [T-180-10](12-e2e-test-matrix.md#t-180-10).

```ascii
+--------------------------------------------------------------------------+
| Review access change                                                     |
|                                                                          |
| Before   Confidential / HR / Atlas                                       |
| After    Internal / HR                                                   |
| Changes  Lower sensitivity; remove required tag Atlas.                   |
|                                                                          |
| This broadens the access rule. Actual access depends on member profiles. |
| Only an authorized security steward can save this change.                |
|                                                                          |
| Reason for change                                                        |
| [Approved for the wider internal audience.                    ]          |
| 10-500 characters. Visible to authorized security auditors.              |
|                                                                          |
| [Back to editing]                                  [Save access change]  |
+--------------------------------------------------------------------------+
```

## W-180-06

Existing-document bulk edit. [R-180-15](03-product-spec.md#r-180-15), [T-180-15](12-e2e-test-matrix.md#t-180-15).

```ascii
+--------------------------------------------------------------------------+
| Edit access for 20 selected documents                                    |
| Operation    (o) Add tags  ( ) Remove tags  ( ) Replace access           |
| Sensitivity  Mixed (preserved when adding tags)                          |
| Add tags     [Legal x] [+ Add]                                           |
|                                                                          |
| Preview   17 change / 3 already match / 0 blocked                        |
| Show per-document changes [v]                                            |
|                                                                          |
| Any blocked or stale item prevents the entire commit.                    |
| Removing tags requires steward authority and a reason.                   |
| No off-page selection. Maximum 100 explicit documents.                   |
|                                                                          |
|                                       [Cancel] [Apply to 20 documents]   |
+--------------------------------------------------------------------------+
```

## W-180-07

Conflict and uncertain outcome. [R-180-09/21](03-product-spec.md), [T-180-09/21](12-e2e-test-matrix.md).

```ascii
+--------------------------------------------------------------------------+
| Access changed while you were editing.                                   |
|                                                                          |
| Your draft        Restricted / HR / Atlas                                |
| Current saved     Confidential / HR / Finance                            |
|                                                                          |
| [Compare latest] [Reload saved values] [Keep draft for review]           |
| A new preview and explicit Save are required. No automatic merge.        |
|                                                                          |
| Alternate network state:                                                 |
| We could not confirm whether the change was saved.                       |
| Operation is pending or the response was lost.                           |
| [Check status]   Retry uses the same operation key.                      |
| Do not show Saved or Failed until the server outcome is known.           |
+--------------------------------------------------------------------------+
```

## W-180-08

Off, read-only and unavailable states. [R-180-13](03-product-spec.md#r-180-13), [T-180-13](12-e2e-test-matrix.md#t-180-13).

```ascii
+--------------------------------------------------------------------------+
| Access                                                                   |
|                                                                          |
| Off          Document access rules are off for this workspace.           |
|              [Open Access settings] (owner/admin only)                   |
|                                                                          |
| Read-only    Sensitivity  Confidential                                   |
|              Security tags  HR, Atlas                                    |
|              Your role cannot change document access.                    |
|                                                                          |
| Unavailable  Access rules could not be checked. [Retry]                  |
|              Save is disabled; draft values are retained.                |
|                                                                          |
| Needs repair Access needs repair. Unavailable until reviewed.            |
| Emergency    Emergency access is active. Access is audited.              |
+--------------------------------------------------------------------------+
```

## W-180-09

Mobile editor. [R-180-19](03-product-spec.md#r-180-19), [T-180-19](12-e2e-test-matrix.md#t-180-19).

```ascii
+------------------------------------------+
| Document access                    [X]   |
| Employment.pdf                           |
|                                          |
| Sensitivity                              |
| [Confidential                       v]   |
|                                          |
| Required security tags                   |
| [HR x] [Atlas x] [+ Add]                 |
| Readers must hold every selected tag.    |
|                                          |
| Who can access                           |
| Confidential or higher, HR AND Atlas.    |
| Authorized stewards may also qualify.    |
|                                          |
| Scrollable body; focused fields stay     |
| above the sticky action footer.          |
|                                          |
| [Cancel]                 [Save access]   |
+------------------------------------------+
```

## Shared state requirements

Every entry point must render loading, ready, dirty, validating, forbidden, no results, empty catalog, policy off, unavailable, saving, conflict and unknown-outcome states where applicable. Loading uses fixed-height skeletons rather than disappearing properties. Empty catalog and error cannot share the same empty array presentation. A role change while open immediately removes mutation capability and requires fresh preview. See [edge cases](11-edge-cases.md) and [test catalog](12-e2e-test-matrix.md).

On list rows, chip overflow never increases row height unexpectedly. In detail editors, full tag names wrap. Preserve preview scroll position when opening/closing access controls; avoid opening a second full-screen document viewer solely to edit a property.
