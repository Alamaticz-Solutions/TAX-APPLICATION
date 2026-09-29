# PostgreSQL Local Notes

These notes are for local development only. Generated database package
execution should normally be run through:

```bash
scripts/appfw migrate
```

## Run PostgreSQL In Docker

```bash
docker volume create appfw-postgres-volume
docker run -d \
  --name appfw-postgres \
  -p 5432:5432 \
  -v appfw-postgres-volume:/var/lib/postgresql/data \
  -e POSTGRES_PASSWORD=postgres \
  postgres:16
```

Connect:

```bash
docker exec -it appfw-postgres psql -U postgres
```

## Optional pgAdmin

```bash
docker run -d \
  --name appfw-pgadmin \
  -p 5050:80 \
  -e PGADMIN_DEFAULT_EMAIL=dev@example.com \
  -e PGADMIN_DEFAULT_PASSWORD=devpass \
  dpage/pgadmin4
```

Open:

```text
http://localhost:5050/browser/
```

Use local-only passwords and do not commit real credentials.

## UUID Extension

Some generated PostgreSQL packages may require UUID support:

```sql
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
ALTER EXTENSION "uuid-ossp" SET SCHEMA public;
```
