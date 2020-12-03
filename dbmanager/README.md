# DB Manager

This microservice is for migration/cleanup scripts for the postgresql database.

## Cleanup Scripts

These are located in the cleanup directory and they are ran every time you start the DB manager container.

## Migration Scripts

Any schema changes in the database are located here. They are idempotent so you can run these multiple times without messing with your db.