"""
Serves as the entrypoint of the database container, running migrations as
necessary and automatically running new migrations as they appear.
"""

__author__ = "Architus"
__version__ = "0.1.0"

import os
import sys
import shlex
import subprocess
import time

BASE_DIR = os.path.dirname(os.path.realpath(__file__))
MIGRATIONS_DIR = os.path.join(BASE_DIR, "migrations")
CURRENT_MIGRATIONS_DIR = os.path.join(BASE_DIR, "current_migration")
CURRENT_MIGRATION_FILE = "current.out"


def main():
    print("Running db migration manager")
    current_migration = get_current_migration()
    to_run = get_new_migrations(current_migration)
    if to_run:
        print(f"Running migrations: {to_run}")
        run_migrations(to_run)
    else:
        print("No migrations to run")


def run_migrations(sql_names):
    """
    Runs all migration SQL files in the order given
    """
    for file_name in sql_names:
        exitcode = execute_sql_script(os.path.join(MIGRATIONS_DIR, file_name))
        if exitcode == 2:
            # retry once if connection to server fails
            print("failed to connect to postgres server. Retrying in 20 Seconds...")
            time.sleep(20)
            exitcode = execute_sql_script(os.path.join(MIGRATIONS_DIR, file_name))

        if exitcode != 0:
            raise Exception(f"oh no, {file_name} failed to execute")

    current_path = os.path.join(CURRENT_MIGRATIONS_DIR, CURRENT_MIGRATION_FILE)
    last_migration = sql_names[-1]
    with open(current_path, "w") as file:
        print(f"Saving last-run migration '{last_migration}' to disk at {current_path}")
        file.write(last_migration)


def execute_sql_script(file):
    """
    Runes a sql script on the postgres container
    """
    process = subprocess.run(["psql", "-h", "postgres", "-U", "autbot", "-a", "-v", "ON_ERROR_STOP=1", "-f", file],
                          stdout=sys.stdout,
                          stderr=sys.stderr)
    return process.returncode;


def get_new_migrations(current):
    """
    Gets all new migration scripts to run from the given one
    """
    migrations = get_all_files(MIGRATIONS_DIR, suffix=".sql")
    migrations.sort()

    if current is None:
        return migrations

    try:
        idx = migrations.index(current) + 1
        return migrations[idx:]
    except ValueError:
        # If the current migration was not in the list, run every migration again
        print("An error occurred while detecting the last-run migration. Running all migrations again")
        return migrations


def get_current_migration():
    """
    Attempts to get the current migration from the mounted volume
    """

    all_files = get_all_files(CURRENT_MIGRATIONS_DIR, suffix=CURRENT_MIGRATION_FILE)
    if all_files:
        filename = os.path.join(CURRENT_MIGRATIONS_DIR, all_files[0])
        with open(filename, 'r') as file:
            return file.read().replace('\n', '').strip()
    return None


def get_all_files(dirname, suffix=None):
    """
    Gets all files with the given suffix in the directory
    """

    return [file for file in os.listdir(dirname)
            if suffix is None or file.endswith(suffix)]


if __name__ == "__main__":
    main()
