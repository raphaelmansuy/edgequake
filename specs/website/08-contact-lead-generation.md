# SPEC-WEBSITE-08: Contact & Lead Generation

> **Status**: DRAFT  
> **Created**: 2026-03-21  
> **Parent**: [00-overview.md](./00-overview.md)  
> **Related**: [03-page-specifications.md](./03-page-specifications.md) · [04-technical-architecture.md](./04-technical-architecture.md) · [05-seo-strategy.md](./05-seo-strategy.md)

---

## 1. Contact Form Specification

### 1.1 Form Fields

<<<<<<< HEAD
| Field | Type | Required | Validation | Placeholder |
|-------|------|----------|-----------|-------------|
| First Name | `text` | Yes | 2-50 chars, letters/hyphens/spaces | "First name" |
| Last Name | `text` | Yes | 2-50 chars, letters/hyphens/spaces | "Last name" |
| Business Email | `email` | Yes | RFC 5322 email format | "you@company.com" |
| Company | `text` | No | 2-100 chars | "Company name" |
| Inquiry Type | `select` | Yes | Must select one | "Select inquiry type..." |
| Message | `textarea` | Yes | 10-2000 chars | "Tell us about your project..." |

### 1.2 Inquiry Types

| Value | Label |
|-------|-------|
| `enterprise-support` | Enterprise Support |
| `partnership` | Partnership |
| `consulting` | Architecture Consulting |
| `custom-integration` | Custom Integration |
| `training` | Training & Onboarding |
| `other` | Other |
=======
| Field          | Type       | Required | Validation                         | Placeholder                     |
| -------------- | ---------- | -------- | ---------------------------------- | ------------------------------- |
| First Name     | `text`     | Yes      | 2-50 chars, letters/hyphens/spaces | "First name"                    |
| Last Name      | `text`     | Yes      | 2-50 chars, letters/hyphens/spaces | "Last name"                     |
| Business Email | `email`    | Yes      | RFC 5322 email format              | "you@company.com"               |
| Company        | `text`     | No       | 2-100 chars                        | "Company name"                  |
| Inquiry Type   | `select`   | Yes      | Must select one                    | "Select inquiry type..."        |
| Message        | `textarea` | Yes      | 10-2000 chars                      | "Tell us about your project..." |

### 1.2 Inquiry Types

| Value                | Label                   |
| -------------------- | ----------------------- |
| `enterprise-support` | Enterprise Support      |
| `partnership`        | Partnership             |
| `consulting`         | Architecture Consulting |
| `custom-integration` | Custom Integration      |
| `training`           | Training & Onboarding   |
| `other`              | Other                   |
>>>>>>> origin/edgequake-main

### 1.3 Form Layout (Desktop)

```
┌──────────────────────────────────────────────────────┐
│  Tell us about your project                          │
│                                                      │
│  ┌─────────────────┐  ┌─────────────────┐            │
│  │ First Name *    │  │ Last Name *     │            │
│  │ [____________]  │  │ [____________]  │            │
│  └─────────────────┘  └─────────────────┘            │
│                                                      │
│  ┌──────────────────────────────────────┐            │
│  │ Business Email *                     │            │
│  │ [________________________________]  │            │
│  └──────────────────────────────────────┘            │
│                                                      │
│  ┌──────────────────────────────────────┐            │
│  │ Company                              │            │
│  │ [________________________________]  │            │
│  └──────────────────────────────────────┘            │
│                                                      │
│  ┌──────────────────────────────────────┐            │
│  │ Inquiry Type *                       │            │
│  │ [▼ Select inquiry type...]          │            │
│  └──────────────────────────────────────┘            │
│                                                      │
│  ┌──────────────────────────────────────┐            │
│  │ Message *                            │            │
│  │ [                                  ] │            │
│  │ [                                  ] │            │
│  │ [                                  ] │            │
│  │ [________________________________] │            │
│  └──────────────────────────────────────┘            │
│                                                      │
│  By submitting, you agree to our privacy             │
│  policy. We respect your data.                       │
│                                                      │
│  [Submit →]                                          │
│                                                      │
└──────────────────────────────────────────────────────┘
```

### 1.4 Form States

<<<<<<< HEAD
| State | UI |
|-------|-----|
| **Idle** | Empty form with placeholders |
| **Validation Error** | Red border on invalid fields, error message below field |
| **Submitting** | Button shows spinner, form inputs disabled |
| **Success** | Form replaced with "Thank you! We'll respond within 2 business days." + check icon |
| **Error** | Toast notification: "Something went wrong. Please email contact@elitizon.com directly." |
=======
| State                | UI                                                                                      |
| -------------------- | --------------------------------------------------------------------------------------- |
| **Idle**             | Empty form with placeholders                                                            |
| **Validation Error** | Red border on invalid fields, error message below field                                 |
| **Submitting**       | Button shows spinner, form inputs disabled                                              |
| **Success**          | Form replaced with "Thank you! We'll respond within 2 business days." + check icon      |
| **Error**            | Toast notification: "Something went wrong. Please email contact@elitizon.com directly." |
>>>>>>> origin/edgequake-main

---

## 2. Form Backend (Formspree)

### 2.1 Why Formspree

<<<<<<< HEAD
| Criterion | Formspree | Resend | Custom API |
|-----------|----------|--------|-----------|
| Static site compatible | Yes | Yes (needs edge function) | No |
| No server code | Yes | No | No |
| Free tier | 50 submissions/mo | 100 emails/day | N/A |
| Spam protection | reCAPTCHA, honeypot | Manual | Manual |
| Setup complexity | Minimal | Moderate | High |
=======
| Criterion              | Formspree           | Resend                    | Custom API |
| ---------------------- | ------------------- | ------------------------- | ---------- |
| Static site compatible | Yes                 | Yes (needs edge function) | No         |
| No server code         | Yes                 | No                        | No         |
| Free tier              | 50 submissions/mo   | 100 emails/day            | N/A        |
| Spam protection        | reCAPTCHA, honeypot | Manual                    | Manual     |
| Setup complexity       | Minimal             | Moderate                  | High       |
>>>>>>> origin/edgequake-main

**Decision:** Formspree for v1 (zero server code). Migrate to Resend if volume exceeds 50/mo.

### 2.2 Integration

```typescript
// src/components/contact/contact-form.tsx
"use client";

import { useActionState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Select } from "@/components/ui/select";

const FORMSPREE_URL = `https://formspree.io/f/${process.env.NEXT_PUBLIC_FORMSPREE_ID}`;

export function ContactForm() {
  // Client-side form with fetch to Formspree
  // Includes client validation before submit
  // Shows success/error state after response
}
```

### 2.3 Formspree Configuration

```yaml
# formspree.json (optional, for advanced config)
forms:
  contact:
    email: contact@elitizon.com
    subject: "EdgeQuake Enterprise Inquiry: {{inquiry_type}}"
<<<<<<< HEAD
    redirect: false     # Handle in-page
=======
    redirect: false # Handle in-page
>>>>>>> origin/edgequake-main
    fields:
      - name: first_name
        required: true
      - name: last_name
        required: true
      - name: email
        type: email
        required: true
      - name: company
      - name: inquiry_type
        required: true
      - name: message
        required: true
```

### 2.4 Email Routing

```
Form submit
  │
  ▼
Formspree server
  │
  ├── Spam filter (reCAPTCHA score + honeypot)
  │
  ├── Forward to: contact@elitizon.com
  │     Subject: "EdgeQuake Enterprise Inquiry: {inquiry_type}"
  │     Reply-To: {user_email}
  │
  └── Auto-response to user (optional, Formspree feature)
        Subject: "Thank you for contacting EdgeQuake"
        Body: "We received your inquiry and will respond within
               2 business days. — The EdgeQuake Team"
```

---

## 3. Spam Protection

### 3.1 Honeypot Field

```html
<!-- Hidden field — bots fill it, humans don't -->
<input
  type="text"
  name="_gotcha"
  style="display: none"
<<<<<<< HEAD
  tabIndex={-1}
  autoComplete="off"
=======
  tabindex="{-1}"
  autocomplete="off"
>>>>>>> origin/edgequake-main
/>
```

### 3.2 Additional Defenses

<<<<<<< HEAD
| Defense | Method |
|---------|--------|
| Honeypot | Hidden `_gotcha` field (Formspree built-in) |
| Rate limit | Formspree limits per IP |
| Client validation | Required fields + format checks before fetch |
=======
| Defense                 | Method                                        |
| ----------------------- | --------------------------------------------- |
| Honeypot                | Hidden `_gotcha` field (Formspree built-in)   |
| Rate limit              | Formspree limits per IP                       |
| Client validation       | Required fields + format checks before fetch  |
>>>>>>> origin/edgequake-main
| No `action` URL in HTML | URL constructed in JS only (harder to scrape) |

---

## 4. Lead Qualification

### 4.1 Lead Scoring (Internal Use)

Leads forwarded to `contact@elitizon.com` are scored informally by:

<<<<<<< HEAD
| Signal | Score | Indicator |
|--------|-------|-----------|
| Business email domain (not gmail/yahoo) | +2 | Likely enterprise |
| Company name provided | +1 | Engaged prospect |
| Inquiry type = "Enterprise Support" or "Consulting" | +2 | High intent |
| Message length > 100 chars | +1 | Detailed need |
| Message mentions team size / budget / timeline | +3 | Qualified |

### 4.2 Response SLA

| Tier | Response Time | Criteria |
|------|---------------|----------|
| Hot lead (score ≥ 5) | 24 hours | Enterprise email + detailed message |
| Warm lead (score 3-4) | 48 hours | Business email + clear need |
| General inquiry (score < 3) | 5 business days | All others |
=======
| Signal                                              | Score | Indicator         |
| --------------------------------------------------- | ----- | ----------------- |
| Business email domain (not gmail/yahoo)             | +2    | Likely enterprise |
| Company name provided                               | +1    | Engaged prospect  |
| Inquiry type = "Enterprise Support" or "Consulting" | +2    | High intent       |
| Message length > 100 chars                          | +1    | Detailed need     |
| Message mentions team size / budget / timeline      | +3    | Qualified         |

### 4.2 Response SLA

| Tier                        | Response Time   | Criteria                            |
| --------------------------- | --------------- | ----------------------------------- |
| Hot lead (score ≥ 5)        | 24 hours        | Enterprise email + detailed message |
| Warm lead (score 3-4)       | 48 hours        | Business email + clear need         |
| General inquiry (score < 3) | 5 business days | All others                          |
>>>>>>> origin/edgequake-main

---

## 5. Enterprise CTA Placements

CTAs throughout the site funnel users to the contact page:

<<<<<<< HEAD
| Location | CTA Text | Link |
|----------|---------|------|
| Homepage hero | "Get Started" | `/docs/getting-started` |
| Homepage enterprise section | "Contact Us →" | `/contact` |
| Enterprise page bottom | "Contact Us →" | `/contact` |
| Docs sidebar footer | "Need help? Contact us" | `/contact` |
| 404 page | "Reach out to us" | `/contact` |
| Footer company column | "Contact" | `/contact` |
=======
| Location                    | CTA Text                | Link                    |
| --------------------------- | ----------------------- | ----------------------- |
| Homepage hero               | "Get Started"           | `/docs/getting-started` |
| Homepage enterprise section | "Contact Us →"          | `/contact`              |
| Enterprise page bottom      | "Contact Us →"          | `/contact`              |
| Docs sidebar footer         | "Need help? Contact us" | `/contact`              |
| 404 page                    | "Reach out to us"       | `/contact`              |
| Footer company column       | "Contact"               | `/contact`              |
>>>>>>> origin/edgequake-main

### CTA Analytics Events

Each CTA click fires a Plausible custom event:

```typescript
// Track CTA clicks
function trackCTA(source: string) {
  if (typeof window !== "undefined" && window.plausible) {
    window.plausible("cta_click", { props: { source } });
  }
}
```

<<<<<<< HEAD
| Event Property (`source`) | CTA Location |
|---------------------------|-------------|
| `hero_get_started` | Homepage hero primary button |
| `hero_live_demo` | Homepage hero secondary button |
| `enterprise_banner` | Homepage enterprise section |
| `enterprise_page` | Enterprise page CTA |
| `docs_sidebar` | Docs sidebar footer |
| `footer_contact` | Footer link |
=======
| Event Property (`source`) | CTA Location                   |
| ------------------------- | ------------------------------ |
| `hero_get_started`        | Homepage hero primary button   |
| `hero_live_demo`          | Homepage hero secondary button |
| `enterprise_banner`       | Homepage enterprise section    |
| `enterprise_page`         | Enterprise page CTA            |
| `docs_sidebar`            | Docs sidebar footer            |
| `footer_contact`          | Footer link                    |
>>>>>>> origin/edgequake-main

---

## 6. Privacy & Compliance

### 6.1 Data Handling

<<<<<<< HEAD
| Aspect | Policy |
|--------|--------|
| Data collected | Name, email, company, inquiry type, message |
| Storage | Formspree servers (processed, not stored long-term) + email inbox |
| Third parties | Formspree only |
| Cookies | None from contact form |
| Analytics | Plausible (no cookies, no personal data) |
| GDPR | No EU personal data processing beyond email delivery |
| Consent | Form submission = consent to email response |
=======
| Aspect         | Policy                                                            |
| -------------- | ----------------------------------------------------------------- |
| Data collected | Name, email, company, inquiry type, message                       |
| Storage        | Formspree servers (processed, not stored long-term) + email inbox |
| Third parties  | Formspree only                                                    |
| Cookies        | None from contact form                                            |
| Analytics      | Plausible (no cookies, no personal data)                          |
| GDPR           | No EU personal data processing beyond email delivery              |
| Consent        | Form submission = consent to email response                       |
>>>>>>> origin/edgequake-main

### 6.2 Privacy Copy

Displayed below the submit button:

> "By submitting this form, you consent to Elitizon contacting you about EdgeQuake enterprise solutions. We do not share your data with third parties. See our [privacy policy](/privacy)."

---

## 7. Future Enhancements (v2)

<<<<<<< HEAD
| Enhancement | Description | Trigger |
|-------------|------------|---------|
| Resend integration | Replace Formspree for higher volume | > 50 submissions/month |
| CRM integration | Auto-create leads in HubSpot/Notion | > 20 leads/month |
| Calendar booking | Embed Calendly for direct scheduling | Repeated back-and-forth emails |
| Live chat | Crisp or Intercom widget | High traffic + support demand |
| A/B testing | Form copy variations | Sufficient conversion data |

---

*Previous: [07-component-library.md](./07-component-library.md) · Next: [09-implementation-roadmap.md](./09-implementation-roadmap.md)*
=======
| Enhancement        | Description                          | Trigger                        |
| ------------------ | ------------------------------------ | ------------------------------ |
| Resend integration | Replace Formspree for higher volume  | > 50 submissions/month         |
| CRM integration    | Auto-create leads in HubSpot/Notion  | > 20 leads/month               |
| Calendar booking   | Embed Calendly for direct scheduling | Repeated back-and-forth emails |
| Live chat          | Crisp or Intercom widget             | High traffic + support demand  |
| A/B testing        | Form copy variations                 | Sufficient conversion data     |

---

_Previous: [07-component-library.md](./07-component-library.md) · Next: [09-implementation-roadmap.md](./09-implementation-roadmap.md)_
>>>>>>> origin/edgequake-main
