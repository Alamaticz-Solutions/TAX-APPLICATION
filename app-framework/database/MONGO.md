# MongoDB Local Notes

These notes are for local development only. Generated database package
execution should normally be run through:

```bash
scripts/appfw migrate
```

## Document Model

The framework maps schemas and entity types to MongoDB databases and
collections:

```text
MongoDB instance
`-- database
    `-- collections
```

## Run MongoDB In Docker

```bash
docker volume create appfw-mongo-volume
docker run -d \
  --name appfw-mongo \
  -p 27017:27017 \
  -v appfw-mongo-volume:/data/db \
  -e MONGO_INITDB_ROOT_USERNAME=root \
  -e MONGO_INITDB_ROOT_PASSWORD=localpassword \
  mongo:7
```

Connect:

```bash
docker exec -it appfw-mongo mongosh \
  -u root \
  -p localpassword \
  --authenticationDatabase admin
```

Use local-only credentials and do not commit real passwords.
