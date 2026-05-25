# Phase 4: Admin API - Design Specification

**Date**: 2026-04-22
**Status**: Approved

## Overview

Phase 4 adds Admin API capabilities including Skills review workflow and audit logging to the AionHive platform.

## Architecture

```
API Handlers (extended)
    ↓
AppRouterState (contains audit_repo)
    ↓
AuditRepository (writes to audit_logs table)
```

## Skills Status Workflow

Skills use a status field for review workflow:

| Status | Description |
|--------|-------------|
| `draft` | Draft, being created |
| `pending_review` | Awaiting review |
| `published` | Published, installable |
| `rejected` | Rejected by reviewer |

Newly created Skills default to `pending_review`. After review, they become `published` or `rejected`.

## Database Changes

### Skills Table

Add `status` column to `skills` table:
```sql
ALTER TABLE skills ADD COLUMN status VARCHAR(20) DEFAULT 'pending_review';
```

### Audit Logs Table

The `audit_logs` table already exists from Phase 2. It stores:
- `id` (UUID)
- `agent_id` (String, nullable)
- `action` (String)
- `resource_type` (String)
- `resource_id` (String, nullable)
- `details` (JSONB)
- `timestamp` (DateTime)

## Audit Log Actions

| Action | Resource Type | Description |
|--------|---------------|-------------|
| `skill_created` | skill | New skill created |
| `skill_updated` | skill | Skill updated |
| `skill_deleted` | skill | Skill deleted |
| `skill_reviewed` | skill | Skill approved/rejected |
| `skill_installed` | skill | Skill installed |
| `evaluation_created` | evaluation | New evaluation |
| `agent_registered` | agent | New agent registered |
| `token_issued` | agent | JWT token issued |

## API Endpoints

### Audit Log Endpoints

#### GET /api/admin/audit (Admin only)
Query audit logs with filters.

**Query Parameters:**
- `agent_id` (optional) - Filter by agent
- `action` (optional) - Filter by action type
- `resource_type` (optional) - Filter by resource type
- `limit` (optional, default 50) - Max results
- `offset` (optional, default 0) - Pagination offset

**Response:** `200 OK`
```json
{
  "data": [
    {
      "id": "uuid",
      "agent_id": "agent-1",
      "action": "skill_created",
      "resource_type": "skill",
      "resource_id": "skill-name-1.0.0",
      "details": {},
      "timestamp": "2026-04-22T10:00:00Z"
    }
  ],
  "total": 100,
  "limit": 50,
  "offset": 0
}
```

#### GET /api/audit/my
Query current agent's own audit logs.

**Query Parameters:** Same as above (minus `agent_id`)

### Review Workflow Endpoints

#### POST /api/admin/skills/{id}/approve
Approve a pending skill.

**Request Body:** None required

**Response:** `200 OK`
```json
{
  "message": "Skill approved successfully",
  "skill_id": "skill-name-1.0.0"
}
```

#### POST /api/admin/skills/{id}/reject
Reject a pending skill.

**Request Body:**
```json
{
  "reason": "Optional rejection reason"
}
```

**Response:** `200 OK`
```json
{
  "message": "Skill rejected",
  "skill_id": "skill-name-1.0.0"
}
```

## Error Handling

| Scenario | Response |
|----------|----------|
| Unreviewed skill installed | `403 Forbidden` |
| Non-admin accesses `/api/admin/audit` | `401 Unauthorized` |
| Agent queries other's logs via `/api/audit/my` | Returns own logs only (safe) |
| Skill not found | `404 Not Found` |
| Invalid review action | `400 Bad Request` |

## Security

1. **Admin-only endpoints:** `/api/admin/*` require admin role
2. **Self-service audit:** Agents can only query their own logs via `/api/audit/my`
3. **Install protection:** Unpublished skills cannot be installed

## Implementation Notes

- Extend existing `handlers.rs` with new admin handlers
- Use `audit_repo.create()` to log all operations
- Add status check in `get_skill_handler` and install flow
- Admin role check via JWT claims

## Files to Modify

- `src/api/handlers.rs` - Add admin handlers
- `src/api/models.rs` - Add request/response models
- `src/api/routes.rs` - Add admin routes
- `src/main.rs` - Register admin routes
- `src/db/migrations/001_initial_schema.sql` - Add status column (if not already)

## Verification

- [ ] Admin can query all audit logs
- [ ] Agent can query own audit logs
- [ ] Admin can approve skills
- [ ] Admin can reject skills
- [ ] Unpublished skills return 403 on install
- [ ] All operations are logged