---
kind: routine
description: Run database migrations and local DB tooling (migrate, rollback, seed, studio, reset). Use
  when the user asks to migrate, seed, or open the DB.
uses:
- usage: '[[run-usage]]'
  when:
  - migrate the database
  - run migrations
  - seed the db
  - roll back a migration
  tags:
  - db
  - migrate
  - prisma
  - sql
---

## Inputs

Requires on PATH: `prisma`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects db-write; danger medium.

## Do

Recipe family over a DB CLI (`prisma` — swap for `drizzle`, `sqlx`, etc.; interface stays identical).
Default is `migrate`; call others by name: `rollback`, `seed`, `studio`, `reset`. `reset` is destructive (dev only).

```just
# apply pending migrations (default)
migrate:
  npx prisma migrate deploy

# roll back the last migration batch
rollback:
  npx prisma migrate resolve --rolled-back

# seed with dev fixtures
seed:
  npx prisma db seed

# open a local DB GUI
studio:
  npx prisma studio

# wipe and re-create the dev DB (destructive — dev only)
reset:
  npx prisma migrate reset --force
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `migrate` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
