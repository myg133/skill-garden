# Admin Dashboard Design - AionHive

**Date:** 2026-04-22
**Status:** Approved

---

## 1. Concept & Vision

A minimalist admin dashboard for reviewing and monitoring Skills in the AionHive platform. The interface prioritizes the review queue as the primary workflow, with audit logs and statistics as supporting views. Clean, functional, no-nonsense — like a well-organized internal tool that gets out of the way.

**Style:** Minimalist/Utilitarian — similar to Linear/Basecamp. White backgrounds, clear typography, subtle borders, functional color usage only for status indicators.

---

## 2. Design Language

### Aesthetic Direction
- Minimal, clean, functional admin UI
- White/light gray backgrounds
- Subtle shadows and borders for depth
- Status colors: yellow (pending), green (published), red (rejected), gray (draft)

### Color Palette
```
Background:     #FFFFFF (primary), #F9FAFB (secondary)
Text:           #111827 (primary), #6B7280 (secondary)
Border:         #E5E7EB
Status Yellow:  #F59E0B (pending_review)
Status Green:   #10B981 (published)
Status Red:     #EF4444 (rejected)
Status Gray:    #6B7280 (draft)
Accent Blue:    #3B82F6 (primary actions)
```

### Typography
- Font: System font stack (-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif)
- Headings: 600 weight
- Body: 400 weight
- Size scale: 12px, 14px, 16px, 20px, 24px

### Spacing
- Base unit: 4px
- Component padding: 12px, 16px
- Section gaps: 24px, 32px

---

## 3. Layout & Structure

### Navigation
- Simple horizontal nav bar with logo/title on left
- Nav items: Review Queue, Audit Logs, Stats
- No sidebar — nav is minimal top bar

### Page Structure
```
┌─────────────────────────────────────────────────┐
│  AionHive Admin          [Review] [Audit] [Stats]│
├─────────────────────────────────────────────────┤
│                                                  │
│  [Page Content]                                  │
│                                                  │
└─────────────────────────────────────────────────┘
```

### Responsive Strategy
- Desktop-first (admin tools are typically used on desktop)
- Tables scroll horizontally on smaller screens
- Stack cards vertically on mobile

---

## 4. Pages

### 4.1 Review Queue (`/` or `/review`)

**Purpose:** Primary workflow — review pending skills

**Content:**
- Filter bar: Status dropdown (default: pending_review), Search input
- Skills table/list
- Pagination

**Table Columns:**
| Name | Agent | Tags | Created | Actions |

**Row Actions:**
- View Detail (click row)
- Approve button (green)
- Reject button (red)

**Empty State:** "No pending skills to review"

---

### 4.2 Skill Detail (`/skills/:id`)

**Purpose:** View skill details and take action

**Content:**
- Skill metadata: name, description, tags, version, agent_id, created_at
- Content preview (first 500 chars)
- Statistics card: install_count, evaluation_count, avg_success_rate, confidence
- Audit history for this skill (recent 5 log entries)
- Action panel: Approve / Reject form

**Reject Form:**
- Required: reason textarea
- Submit button

---

### 4.3 Audit Logs (`/audit`)

**Purpose:** Query operation history

**Filter Bar:**
- Action: dropdown (all, skill_create, skill_approve, skill_reject, etc.)
- Resource Type: dropdown (all, skill, agent)
- Agent ID: text input
- Date Range: from/to date inputs
- Search button, Reset button

**Table Columns:**
| Timestamp | Agent | Action | Resource | Details |

**Pagination:** 20 items per page

**Empty State:** "No audit logs match your filters"

---

### 4.4 Stats Dashboard (`/stats`)

**Purpose:** Overview of platform health

**Content:**
- Summary cards row:
  - Total Skills
  - Pending Review
  - Published
  - Total Evaluations

- Recent Activity:
  - Simple list of recent actions (last 10 audit entries)

- Top Skills (optional, if time permits):
  - List of skills sorted by install_count

---

## 5. Component Inventory

### `Badge`
- Props: status (pending_review | published | rejected | draft)
- Appearance: pill-shaped, colored background matching status

### `SkillRow`
- Props: skill object
- Displays: name, agent_id truncated, tags (max 3), created_at, action buttons
- Hover: subtle background highlight
- Click: navigate to detail

### `ReviewActions`
- Props: skill_id, onApproved callback, onRejected callback
- Buttons: Approve (green), Reject (red outline)
- Loading state during API call

### `RejectModal`
- Props: skill_id, onSubmit, onCancel
- Fields: reason textarea (required, min 10 chars)
- Submit disabled until valid

### `AuditTable`
- Props: logs array, loading state
- Columns: timestamp, agent, action, resource, details (truncated)
- Row hover highlight

### `StatCard`
- Props: title, value, subtitle (optional)
- Large number display, small label below

### `EmptyState`
- Props: message, icon (optional)
- Centered text with optional icon

### `LoadingSpinner`
- Simple CSS spinner for loading states

---

## 6. Technical Approach

### Stack
- **Framework:** Svelte + Vite
- **Styling:** Tailwind CSS
- **Routing:** svelte-routing (or svelte-spa-router)
- **HTTP:** native fetch API

### Project Structure
```
admin/
├── index.html
├── package.json
├── vite.config.js
├── tailwind.config.js
├── postcss.config.js
├── src/
│   ├── main.js
│   ├── App.svelte
│   ├── routes/
│   │   ├── Review.svelte
│   │   ├── SkillDetail.svelte
│   │   ├── AuditLogs.svelte
│   │   └── Stats.svelte
│   ├── components/
│   │   ├── Badge.svelte
│   │   ├── SkillRow.svelte
│   │   ├── ReviewActions.svelte
│   │   ├── RejectModal.svelte
│   │   ├── AuditTable.svelte
│   │   ├── StatCard.svelte
│   │   ├── EmptyState.svelte
│   │   └── LoadingSpinner.svelte
│   ├── lib/
│   │   └── api.js          # API client
│   └── stores/
│       └── app.js           # Svelte stores
```

### API Integration

Base URL: `http://localhost:8080/api`

| Endpoint | Method | Usage |
|----------|--------|-------|
| `/skills?status=pending_review&limit=20&offset=0` | GET | List pending skills |
| `/skills/:id` | GET | Skill detail |
| `/skills/:id/stats` | GET | Skill statistics |
| `/admin/audit-logs?action=...&agent_id=...&from=...&to=...&limit=20&offset=0` | GET | List audit logs |
| `/admin/skills/:id/approve` | POST | Approve skill |
| `/admin/skills/:id/reject` | POST | Reject skill |

### Authentication
- Admin endpoints expect JWT token in Authorization header
- Token stored in localStorage (simplest for MVP)
- Login page if no token (simple, can be enhanced later)

### Error Handling
- API errors show toast notification (red, top-right)
- Network errors show "Failed to connect to server"
- 401/403 shows "Unauthorized" message

---

## 7. MVP Scope

### Must Have
- Review Queue page with list and approve/reject actions
- Skill Detail page
- Basic Audit Logs page with filters
- Stats Dashboard with summary cards
- Responsive table layouts
- Toast notifications for feedback

### Nice to Have (Post-MVP)
- Login/auth flow
- Pagination on Review Queue
- Charts for activity trend
- Export audit logs to CSV
- Real-time updates via SSE

---

## 8. Implementation Notes

- Use Tailwind CDN via script tag for fastest setup (no PostCSS complexity)
- Svelte-routing for client-side routing
- No backend session management needed — token stored in localStorage
- All API calls go to port 8080 (assumes backend is running there)
