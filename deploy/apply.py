#!/usr/bin/env python3
import argparse
import os
import glob
import sys
import subprocess
import shlex
from os import path
from typing import Optional, List, Set, Dict


# Directories that won't be searched for kubernetes manifest files
EXCLUDED_DIRS = {".git", ".github", "tilt_modules", "lib"}
SEARCH_ROOT = path.abspath(os.path.join(__file__, "..", ".."))


def main(tag: Optional[str],
         prefix: Optional[str],
         target_dirs: Optional[Set[str]],
         dry_run: bool,
         kubectl_args: Optional[str]) -> int:
    print("Architus deployment script")

    kubectl_arg_list = []
    if kubectl_args:
        kubectl_arg_list = shlex.split(kubectl_args, posix=True)
        print(f"Additional kubectl arguments: {repr(kubectl_arg_list)}")

    print("Searching for Kubernetes manifest files")
    all_manifest_files = find_manifest_files(target_dirs)

    if dry_run:
        print("Dry run -> identified manifest files:")
        for manifest_file in all_manifest_files:
            print(f" - {manifest_file}")
        return 0

    print("Applying all Kubernetes manifest files")
    for manifest_file_path in all_manifest_files:
        print(f"=> {manifest_file_path}")

        manifest_file_contents = ""
        with open(manifest_file_path, "r") as manifest_file:
            manifest_file_contents = manifest_file.read()

        # Render our the variables
        manifest_file_contents = render_variables(manifest_file_contents, {
            "prefix": prefix if prefix else "",
            "tag": tag if tag else "latest",
        })
        p = subprocess.run(["kubectl", "apply", "-f", "-"] + kubectl_arg_list,
                           stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                           input=manifest_file_contents, encoding='ascii')
        for output_line in p.stdout.splitlines():
            print(f" - {output_line}")


def render_variables(contents: str, variables: Dict[str, str]) -> str:
    for name, value in variables.items():
        contents = contents.replace("{{" + name + "}}", value)
    return contents


def find_manifest_files(target_dirs: Optional[Set[str]]) -> List[str]:
    all_files = []
    for subdir in next(os.walk(SEARCH_ROOT))[1]:
        subdir_basename = path.basename(subdir)
        subdir_joined = path.join(SEARCH_ROOT, subdir)
        if subdir_basename in EXCLUDED_DIRS:
            continue
        if target_dirs and subdir_basename not in target_dirs:
            continue

        # Search the subdirectory for **/kube/*.yml and **/kube/prod/**/*.yml
        all_files.extend(glob.glob(path.join(subdir_joined, '**/kube/*.yml'), recursive=True))
        all_files.extend(glob.glob(path.join(subdir_joined, '**/kube/*.yaml'), recursive=True))
        all_files.extend(glob.glob(path.join(subdir_joined, '**/kube/prod/**/*.yml'), recursive=True))
        all_files.extend(glob.glob(path.join(subdir_joined, '**/kube/prod/**/*.yaml'), recursive=True))
    return all_files


def bootstrap():
    parser = argparse.ArgumentParser()
    parser.add_argument('--tag', type=str, help='tag to use for all deployment images')
    parser.add_argument('--prefix', type=str, help='prefix for all deployment images')
    parser.add_argument('--dir', type=str, action='append',
                        help='parent directory to search for **/kube/*.yml and **/kube/prod/**/*.yml')
    parser.add_argument('--dry-run', action="store_true", default=False, help='whether to run kubectl or not')
    parser.add_argument('--kubectl-args', type=str, help='additional kubectl arguments')
    args = parser.parse_args()
    code = main(tag=args.tag if args.tag and len(args.tag.strip()) else None,
                prefix=args.prefix if args.prefix and len(args.prefix.strip()) else None,
                target_dirs=set(args.dir) if args.dir else None,
                dry_run=args.dry_run,
                kubectl_args=args.kubectl_args if args.kubectl_args and len(args.kubectl_args.strip()) else None)
    sys.exit(code)


if __name__ == "__main__":
    bootstrap()
