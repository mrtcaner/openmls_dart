# Updater GitHub App

**Last verified:** 2026-07-23

The scheduled OpenMLS and Copier template update workflows authenticate as a
dedicated GitHub App. Do not replace this with a personal access token or grant
write access to the workflows' default `GITHUB_TOKEN`.

GitHub App tokens are short-lived and repository-scoped. They also let
`peter-evans/create-pull-request` create signed bot commits whose pull-request
events run the normal CI workflows.

## One-time setup

1. Register a GitHub App owned by the repository owner.
2. Disable webhooks; these workflows do not receive App events.
3. Grant these repository permissions:
   - **Contents:** Read and write
   - **Pull requests:** Read and write
   - **Metadata:** Read-only (automatically granted)
4. Install the App on **Only select repositories** and select
   `mrtcaner/openmls_dart`.
5. Copy the numeric App ID into the repository Actions variable `APP_ID`.
6. Generate one private key for the App. Store the complete downloaded PEM,
   including its header and footer, as the repository Actions secret
   `APP_PRIVATE_KEY`, then remove the downloaded copy from ordinary local
   storage.
7. Manually run both updater workflows and confirm their token-generation and
   check steps pass.

The **Workflows: Read and write** permission is not required by the current
notification/update behavior. Add it only if a future updater is deliberately
changed to commit files under `.github/workflows/`.

## Expected no-update result

As of 2026-07-23:

- Copier template: current and latest `v3.0.3`
- OpenMLS: current and latest `openmls-v0.8.1`

Both workflows should finish successfully without creating a pull request when
those versions remain current.

## Failure behavior

The workflows fail before token generation with a direct configuration error
when `APP_ID` or `APP_PRIVATE_KEY` is absent. The OpenMLS checker accepts only
the upstream `openmls-vX.Y.Z` tag form, with an optional semantic prerelease
suffix. Manual workflow inputs are validated in the shell before they are
passed through the Makefile and then validated again by the Dart checker.
Checker exit code 1 means an update is available and continues into PR
creation; configuration, parsing, or network errors use exit code 2 and fail
the workflow instead of being silently ignored.

This double validation is intentional: fetched tags must not become unsafe
GitHub output or branch-name data, and manual inputs must not reach Make's
shell-expanded `ARGS` value before validation.
