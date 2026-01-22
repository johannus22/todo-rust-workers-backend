# todo-rust-workers-backend

## Ory Keto

The app includes an Ory Keto Read API client (`src/db/keto.rs`) for permission checks. Keto's DB runs inside Docker; the worker talks to Keto's HTTP Read API (no direct DB access).

- **`KETO_READ_URL`**: base URL of the Keto Read API (e.g. `http://localhost:4467` for local Docker, or `http://keto:4467` if the worker runs in the same compose). In production, set via `wrangler secret put KETO_READ_URL`.
- **Endpoints used**: `check` (`/relation-tuples/check`), `expand` (`/relation-tuples/expand`), `list_relation_tuples` (`/relation-tuples`).

## Keto Configuration

Keto needs to know about namespaces before you can create/query relation tuples. Define them in a config file:

Create `keto.yml`:

```yaml
namespaces:
  - name: todos
    relations:
      - name: owner
```

Or using OPL (Ory Permission Language):

```opl
class Todo implements Namespace {
  related owners as User
}
```

Example `docker-compose` for Keto (PostgreSQL + Keto):

```yaml
services:
  keto-db:
    image: postgres:15-alpine
    environment:
      POSTGRES_USER: keto
      POSTGRES_DB: keto
      POSTGRES_PASSWORD: secret
    volumes: ["keto-db-data:/var/lib/postgresql/data"]

  keto:
    image: oryd/keto:v0.15.0
    command: serve
    ports:
      - "4466:4466"  # read API
      - "4467:4467"  # write API
    volumes:
      - ./keto.yml:/etc/keto.yml  # mount config file
    environment:
      DSN: postgres://keto:secret@keto-db:5432/keto?sslmode=disable
      SERVE_READ_API_HOST: 0.0.0.0
      SERVE_READ_API_PORT: 4466
      SERVE_WRITE_API_HOST: 0.0.0.0
      SERVE_WRITE_API_PORT: 4467
      LOG_LEVEL: debug
    command: ["serve", "--config", "/etc/keto.yml"]
    depends_on: [keto-db]

volumes:
  keto-db-data: {}
```

**Alternative: Define namespaces via environment**

If you prefer not to mount a file:

```yaml
environment:
  DSN: postgres://keto:secret@keto-db:5432/keto?sslmode=disable
  SERVE_READ_API_HOST: 0.0.0.0
  SERVE_READ_API_PORT: 4466
  SERVE_WRITE_API_HOST: 0.0.0.0
  SERVE_WRITE_API_PORT: 4467
  NAMESPACES: |
    - name: todos
      relations:
        - name: owner
```

With `wrangler dev`, use `KETO_READ_URL=http://localhost:4466` and `KETO_WRITE_URL=http://localhost:4467` so the worker can reach Keto on the host.
