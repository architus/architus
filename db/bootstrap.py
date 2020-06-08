"""
Serves as the entrypoint of the database container, running migrations as
necessary and automatically running new migrations as they appear.
"""

__author__ = "Architus"
__version__ = "0.1.0"

import os
import sys
import shlex
import argparse
import subprocess

POSTGRES_USER = "postgres"
BASE_DIR = os.path.dirname(os.path.realpath(__file__))
MIGRATIONS_DIR = os.path.join(BASE_DIR, "./migrations")
CURRENT_MIGRATIONS_DIR = os.path.join(BASE_DIR, "./current_migration")
CURRENT_MIGRATION_FILE = "current.out"
ORIGINAL_ENTRYPOINT = "/usr/local/bin/docker-entrypoint.sh"
INJECTED_ENTRYPOINT = os.path.join(BASE_DIR, "./entrypoint.sh")


def main():
    print("Running db migration manager")
    current_migration = get_current_migration()
    to_run = get_new_migrations(current_migration)
    if to_run:
        print(f"Running migrations: [{', '.join(to_run)}]")
        run_migrations(to_run)
    else:
        print("No migrations to run")


def run_migrations(sql_names):
    """
    Runs all migration SQL files in the order given,
    starting a local PostgreSQL server to accomplish this.
    """

    # Commands can fail, and if so, they should fail fast
    print("Starting temporary Postgres server to run migrations")
    run_entrypoint_script(["docker_temp_server_start", "postgres"])
    run_entrypoint_script(["docker_process_init_files", *[os.path.join(MIGRATIONS_DIR, file)
                                                          for file in sql_names]])
    print("Stopping temporary Postgres server")
    run_entrypoint_script(["docker_temp_server_stop"])

    current_path = os.path.join(CURRENT_MIGRATIONS_DIR, CURRENT_MIGRATION_FILE)
    last_migration = sql_names[-1]
    with open(current_path, "w") as file:
        print(f"Saving last-run migration '{last_migration}' to disk at {current_path}")
        file.write(last_migration)


def run_entrypoint_script(args):
    """
    Runs a function of the original entrypoint script

    See
    https://github.com/docker-library/postgres/blob/682ff83c5c83f1b6f2b02caf7aa3e17a491b403a/13/docker-entrypoint.sh
    """

    files = " ".join([shlex.quote(a) for a in args])
    command = f"source {ORIGINAL_ENTRYPOINT} && {files} && echo \"$?\""""
    return subprocess.run(["bash", "-c", command],
                          stdout=sys.stdout,
                          stderr=sys.stderr)


def get_new_migrations(current):
    """
    Gets all new migration scripts to run from the given one
    """
    migrations = get_all_files(MIGRATIONS_DIR, suffix=".sql")
    migrations.sort()

    if current == None:
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
            if suffix == None or file.endswith(suffix)]


def inject(lines):
    """
    Inject the command after initial database setup is performed into the normal
    postgres entrypoint script

    See
    https://github.com/docker-library/postgres/blob/682ff83c5c83f1b6f2b02caf7aa3e17a491b403a/13/docker-entrypoint.sh
    """

    injection_point = '''exec "$@"'''
    injection = f"python3 -u {os.path.realpath(__file__)} --migrate"
    final_lines = []
    ever_found = False
    for line in lines:
        if injection_point in line:
            ever_found = True
            without_newline = line.rstrip()
            # Use the indent of the injection point to add space to the beginning
            indent = without_newline.replace(injection_point, '')
            final_lines.append(indent + injection)
            final_lines.append(without_newline)
            pass
        else:
            final_lines.append(line.rstrip())

    # If the injection point was never found, migrations won't ever be run
    # In that case, it's better to fail-fast
    if not ever_found:
        raise Exception(f"Injection point {injection_point} not found in postgres bash script. This may require an update to the injection search fragment.")

    return final_lines


def bootstrap():
    """
    Parses command line arguments and injects this script into the original
    entrypoint unless running in migration mode (--migrate)
    """

    parser = argparse.ArgumentParser()
    parser.add_argument('--migrate', action='store_true')
    args = parser.parse_args()
    if args.migrate:
        main()
    else:
        # Inject a call to our script halfway through the original entrypoint
        # that performs the actual migration
        print("Injecting migration script call into Postgres entrypoint script")
        script_contents = ""
        with open(ORIGINAL_ENTRYPOINT, "r") as file:
            contents = file.readlines()
            new_lines = inject(contents)
            script_contents = "\n".join(new_lines)
        with open(INJECTED_ENTRYPOINT, "w") as file:
            file.write(script_contents)
            
        subprocess.run(["chmod", "+x", INJECTED_ENTRYPOINT])
        subprocess.run(["/usr/bin/env", "bash", INJECTED_ENTRYPOINT, "postgres"],
                            stdout=sys.stdout,
                            stderr=sys.stderr)


if __name__ == "__main__":
    bootstrap()
