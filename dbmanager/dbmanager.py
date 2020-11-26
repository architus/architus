"""
Serves as the entrypoint of the database container, running migrations as
necessary and automatically running new migrations as they appear.
"""

__author__ = "Architus"
__version__ = "0.1.0"

import os
import sys
import subprocess
import time

BASE_DIR = os.path.dirname(os.path.realpath(__file__))
MIGRATIONS_DIR = os.path.join(BASE_DIR, "migrations")
CLEANUP_DIR = os.path.join(BASE_DIR, "cleanup")
CURRENT_MIGRATIONS_DIR = os.path.join(BASE_DIR, "current_migration")
CURRENT_MIGRATION_FILE = "current.out"
PSQL_USER = os.getenv('db_user')


def main():
    configure_psql_env_vars()
    print("Running db migration manager")
    current_migration = get_current_migration()
    new_migrations = get_new_migrations(current_migration)
    if new_migrations:
        print(f"Running migrations: {list(map(os.path.basename, new_migrations))}")
        run_migration_scripts(new_migrations)
    else:
        print("No migrations to run")

    cleanup_scripts = get_cleanup_scripts()
    print(f"Running Cleanup Scripts: {[os.path.basename(script) for script in cleanup_scripts]}")
    run_scripts(cleanup_scripts)


def run_scripts(file_paths):
    """
    Runs all of the SQL scripts given their file paths
    """
    for file_name in file_paths:
        exitcode = execute_sql_script(file_name)
        if exitcode == 2:
            # retry once if connection to server fails
            print("failed to connect to postgres server. Retrying in 20 Seconds...")
            time.sleep(20)
            exitcode = execute_sql_script(os.path.join(MIGRATIONS_DIR, file_name))

        if exitcode != 0:
            raise Exception(f"oh no, {file_name} failed to execute")


def run_migration_scripts(migration_files):
    """
    Runs all migration SQL files in the order given
    """
    run_scripts(migration_files)

    current_path = os.path.join(CURRENT_MIGRATIONS_DIR, CURRENT_MIGRATION_FILE)
    last_migration = os.path.basename(migration_files[-1])
    with open(current_path, "w") as file:
        print(f"Saving last-run migration '{last_migration}' to disk at {current_path}")
        file.write(last_migration)


def execute_sql_script(file):
    """
    Runes a sql script on the postgres container
    """
    process = subprocess.run(["psql", "-h", "postgres", "-U", PSQL_USER, "-a", "-v", "ON_ERROR_STOP=1", "-f", file],
                             stdout=sys.stdout,
                             stderr=sys.stderr)
    return process.returncode


def get_cleanup_scripts():
    """
    Gets all of the cleanup scripts to run.
    """
    cleanup_scripts = get_all_files(CLEANUP_DIR, suffix=".sql")
    return [os.path.join(CLEANUP_DIR, f) for f in cleanup_scripts]


def get_new_migrations(current):
    """
    Gets all new migration scripts to run from the given one
    """
    base_migrations = get_all_files(MIGRATIONS_DIR, suffix=".sql")
    base_migrations.sort()
    full_path_migrations = [os.path.join(MIGRATIONS_DIR, f) for f in base_migrations]

    if current is None:
        return full_path_migrations

    try:
        idx = base_migrations.index(current) + 1
        return full_path_migrations[idx:]
    except ValueError:
        # If the current migration was not in the list, run every migration again
        print("An error occurred while detecting the last-run migration. Running all migrations again")
        return full_path_migrations


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


def configure_psql_env_vars():
    """
    Maps necessary postgres credential enviromental variables to allow for scripting.
    """
    os.environ['PGPASSWORD'] = os.getenv('db_pass')


if __name__ == "__main__":
    main()
