# Public beta release

This runbook prepares the public npm package. Publishing, pushing a branch, or
changing registry settings always requires explicit owner action.

## Before the first release

1. Restore `main` on `github.com/BaydoganMirac/patchfleet` and require the CI
   job before merge. Do not force-push over an unknown remote history.
2. Verify that `patchfleet` is still unclaimed on npm.
3. Run the Node 22 macOS, Windows, and Linux jobs plus Linux Node 24.
4. From a clean review commit run:

   ```sh
   npm ci
   npm test
   npm run build
   npm run test:package
   npm publish --dry-run --access public --tag beta
   ```

Review the dry-run file list and reject any credential, environment file,
machine path, test fixture, private Cloud document, or unexpected build output.

## First-package bootstrap

npm requires a package to exist before a trusted publisher can be attached.
Because `patchfleet` is currently unclaimed, the owner must create its first
beta version interactively with npm account 2FA after a separate release
approval:

```sh
npm publish --access public --tag beta
```

Do not put an OTP or registry token in the command, shell history, repository,
or GitHub secret. Confirm the owner and package contents on npm immediately.

## Enable trusted publishing

After the package exists:

1. Create a protected GitHub environment named `npm` with required owner
   approval.
2. In the npm package's trusted-publisher settings select GitHub Actions and
   enter owner `BaydoganMirac`, repository `patchfleet`, workflow
   `publish.yml`, environment `npm`, and allow `npm publish`.
3. Require 2FA and disallow traditional publishing tokens after the OIDC path
   succeeds.
4. Bump to a new version, merge the reviewed commit to `main`, and manually run
   **Publish npm beta**. The workflow publishes with OIDC and automatic
   provenance; it contains no npm token.

The workflow pins npm 11.5.1 because that is the minimum CLI version for npm
trusted publishing. It runs only from `main`, uses a GitHub-hosted runner, and
requires the protected `npm` environment.

## Verify and recover

After publication:

```sh
npm view patchfleet version dist-tags repository --json
npx --yes patchfleet@beta doctor
```

If the artifact is wrong, stop promotion and deprecate the affected version
with an actionable message. Prefer a corrected version over unpublishing: npm
versions are immutable and cannot be reused. Local Patchfleet remains usable
from the reviewed tarball while registry or Cloud release work is paused.
