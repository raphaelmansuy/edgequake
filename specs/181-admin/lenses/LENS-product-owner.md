# Lens — Product Owner

Parent: [README](../README.md) · Spec: [03](../03-product-spec.md) · [06](../06-user-journeys.md)

## The bet

We are not adding features. We are removing the distance between "I need a workspace for Legal"
and "Legal exists, I own it, and my two paralegals can open it". Today that distance is a 16-step
wizard, a 403, or an organization nobody owns.

Three numbers tell the story:

| | today | after |
|--|-------|-------|
| decisions to create an empty workspace | 8 steps, 14 form controls | 3 |
| organizations created with zero owners | all of them | 0 |
| delete planes covered | 3 of 9 | 9 of 9 |

## Why now

1. **The product cannot be sold multi-tenant.** `POST /tenants` produces an organization whose
   only possible administrator is a platform admin. Any customer who creates their own second
   organization is stuck at the first invitation.
2. **The enterprise shape is blocked.** The most common enterprise request is "administer one
   department, not the company". `memberships.workspace_id` exists, is enforced, and has no
   write path. That is a half-built feature shipping as a bug.
3. **Support cost.** Every "I can't see my organization" ticket is this defect. Every
   "the button 403s" ticket is the ungated create.

## What ships first, and why that order

```text
  W1 ownership + quota   ← correctness. Nothing else matters if the org has no owner.
  W2 grants API          ← the missing axis.
  W3 access + nav        ← stop offering what we refuse.
  W4 purge + schema      ← stop leaking what we delete.
  W5 dialogs             ← the visible win, once the foundation is honest.
```

The order is deliberate: **a beautiful dialog on top of an organization nobody owns is a
better-looking bug.** W5 is when customers notice the improvement; W1–W4 is when the product
becomes correct.

## Scope discipline

In:

- The two create flows and their four entry points.
- The grant model (three axes, one catalog).
- Deletion as a first-class, observable operation.
- Navigation derived from rights.
- The visual language for all of the above.

Out, and named so nobody re-litigates it:

- Billing, invoices, proration.
- Moving a workspace between organizations.
- SCIM.
- Per-workspace plans.
- A teams/groups object. EdgeQuake's grouping primitive is the workspace; a team layer would
  be a fourth axis with no enforcement behind it.

## How we know it worked

| Signal | Instrument | Target |
|--------|-----------|--------|
| Median time to first workspace on a new org | create-flow telemetry (start → 201) | < 45 s |
| Create attempts that end in 403 | API logs, `capability_required` on create routes | 0 |
| Organizations with no active owner | nightly `SELECT` | 0 |
| Purge receipts in `partial` > 24 h | alert | 0 |
| Support tickets tagged "access" | tracker | −60 % in two cycles |
| p95 of `POST /tenants` | latency histogram | < 400 ms |

The first two are the product's promise ([LAW-181-3](../01-first-principles.md),
[LAW-181-4](../01-first-principles.md)) expressed as numbers.

## The story I tell a customer

> "Creating an organization takes one field. It is yours the moment it exists. You then decide,
> per workspace, who administers it — the way you would in Vercel or GitHub. And when you delete
> something, it is actually deleted."

That is three sentences because the product should be explainable in three sentences.

## What I will refuse to approve

| Request | Refusal |
|---------|---------|
| "Add a 'skip for now' to the wizard" | The wizard *is* the skip path. There is nothing to skip. |
| "Show the create button disabled with a tooltip explaining your role" | Absent beats disabled for role; disabled is only for quota, which the user can fix. |
| "Let org admins create organizations" | An organization is the tenant boundary. Only the platform creates boundaries. |
| "Hide the quota until it matters" | The quota is the reason the dialog is short. It is part of the promise. |
| "Ship the dialogs first, the ownership fix later" | No. |
