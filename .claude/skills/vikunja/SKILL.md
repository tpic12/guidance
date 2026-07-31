---
name: vikunja
description: Manage TTRPG App Vikunja tickets via the vja CLI
---

# Vikunja Ticket Management

**Scope:** This project uses Vikunja project ID `7` (TTRPG App) exclusively. All commands below default to that project. Do not create or move tasks to other projects.

## Listing & Filtering

**List all open tasks in TTRPG App:**
```bash
vja ls -o 7
```

**List all tasks including completed ones:**
```bash
vja ls -o 7 --include-completed
```

**Filter by priority (1-5, where 1 is lowest):**
```bash
vja ls -o 7 -p "ge 3"        # priority >= 3
vja ls -o 7 -p "eq 1"        # priority = 1 (low)
```

**Filter by title (regex):**
```bash
vja ls -o 7 -i "multiclass"  # matches "multiclass" in title
```

**Filter by label (by ID or title):**
```bash
vja ls -o 7 -l "topic:character"  # tasks with this label
```

**Filter by due date:**
```bash
vja ls -o 7 -d "before 2026-08-15"
vja ls -o 7 -d "after 2026-07-29"
```

**Combine filters (all conditions must match):**
```bash
vja ls -o 7 -p "ge 2" -i "spell" -d "before 2026-08-15"
```

**Show verbose output with task counts:**
```bash
vja ls -o 7 -v
```

## Creating Tasks

**Create a basic task:**
```bash
vja add -o 7 "Brief task title"
```

**Create with description:**
```bash
vja add -o 7 "Task title" -n "Detailed description of what needs to be done"
```

**Create with priority (1-5):**
```bash
vja add -o 7 "High-priority task" -p 5
```

**Create with due date (supports natural language):**
```bash
vja add -o 7 "Task" -d "in 3 days"
vja add -o 7 "Task" -d "2026-08-15"
vja add -o 7 "Task" -d "next Friday"
```

**Create with labels:**
```bash
vja add -o 7 "Task" -l "kind:bug" -l "topic:ui"
```

**Create with multiple options:**
```bash
vja add -o 7 "Implement tabbed nav" \
  -n "Convert character wizard from linear to free tabbed navigation" \
  -p 4 \
  -d "in 5 days" \
  -l "kind:feature" \
  -l "area:character-builder"
```

**Show the resulting task after creation (for verification):**
```bash
vja add -o 7 "Task title" -v
```

## Viewing Task Details

**Show full details of a task:**
```bash
vja show 123           # shows task 123
vja show 123 456       # shows multiple tasks
```

**Open a task in the browser:**
```bash
vja open 123
```

## Editing & Moving Tasks

**Update a task's title:**
```bash
vja edit 123 -i "New title"
```

**Update description:**
```bash
vja edit 123 -n "New description"
```

**Append to description (without overwriting):**
```bash
vja edit 123 -a "Additional notes here"
```

**Change priority:**
```bash
vja edit 123 -p 3
```

**Change due date:**
```bash
vja edit 123 -d "in 7 days"
vja edit 123 -d "2026-08-20"
```

**Mark task as favorite/starred:**
```bash
vja edit 123 -f
vja edit 123 --no-favorite    # unstar
```

**Add or toggle a label:**
```bash
vja edit 123 -l "kind:blocked"
```

**Edit multiple tasks at once:**
```bash
vja edit 123 456 789 -p 2 -d "in 10 days"
```

**Show result after edit (for verification):**
```bash
vja edit 123 -i "New title" -v
```

## Marking Tasks Done / Toggling Completion

**Mark a task as done:**
```bash
vja toggle 123
```

**Mark as done using edit syntax:**
```bash
vja edit 123 -c true
vja edit 123 --completed true
```

**Mark as incomplete:**
```bash
vja edit 123 -c false
```

## Deleting Tasks

⚠️ **Deletes are permanent.** Confirm before proceeding.

**Delete a single task:**
```bash
vja delete 123
```

**Delete multiple tasks:**
```bash
vja delete 123 456 789
```

**Suppress confirmation prompt:**
```bash
vja delete 123 -q
```

## Cloning Tasks

**Clone an existing task (creates a duplicate in the same project):**
```bash
vja clone 123
```

## Workflow Examples

**List all open tasks and pick one to work on:**
```bash
vja ls -o 7
vja show 123       # see details
```

**Create a new ticket while working on a task:**
```bash
vja add -o 7 "Follow-up task discovered during work" \
  -n "Context from parent ticket #123" \
  -d "in 3 days" \
  -l "kind:subtask"
```

**Update a ticket with blockers found during implementation:**
```bash
vja edit 123 -a "Blocked on ticket #456: depends on schema migration"
vja edit 123 -p 5   # escalate priority if new blocker
```

**Mark a ticket done after PR is merged:**
```bash
vja edit 123 -a "Implemented in PR #xyz"
vja toggle 123
```

## Project ID Reference

- **Project 7** = "TTRPG App" (this project's tasks)
- Use `vja project ls` to see all available projects (for reference only; do not create tasks in other projects without explicit approval)

## Notes

- All `vja` commands require a working Vikunja server connection (credentials stored in `~/.vjacli/`)
- Due dates and times support natural language: `"in 3 days"`, `"next Friday"`, `"2026-08-15 at 18:00"`
- Labels must exist on the server unless you pass `--force-create` (use with caution)
- Filter operators: `eq` (equals), `ne` (not equals), `gt` (greater than), `lt` (less than), `ge` (greater than or equal), `le` (less than or equal), `before`, `after`, `contains`
- Bulk operations (editing/deleting multiple tasks) are permanent — always confirm the task list first with `vja ls` and review the command before executing
