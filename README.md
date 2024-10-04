# Releaser

A tool to create releases for your Node.js projects.
It works like a charm with monorepos!

## Usage

```bash
$ releaser [environment]
```

## Usage with Github Actions

1. Create a PAT token as described above.
2. Add the PAT token as a secret in your repository.
3. Create a new workflow file in your repository.

```yaml
on:
  workflow_dispatch:

permissions:
  contents: write
  pull-requests: write
  issues: write

name: releaser-production
jobs:
  release:
    name: Create the release
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Git config
        run: git config --global user.email "github-actions[bot]@users.noreply.github.com" && git config --global user.name "github-actions[bot]"

      - name: Create branch
        run: git checkout -b production-release

      - name: Releaser
        run: ./releaser/releaser-linux production

      - name: Push
        run: |
          git push -f origin production-release
          PR_EXISTS=$(gh pr list --head production-release --base main --json number --jq length)
          if [ "$PR_EXISTS" -eq "0" ]; then
            gh pr create --title "Staging Release" --body-file ./pull_request_content.md --base main --head production-release
          else
            echo "PR already exists. Updating..."
            PR_NUMBER=$(gh pr list --head production-release --base main --json number --jq '.[0].number')
            gh pr edit $PR_NUMBER --body-file ./pull_request_content.md
          fi
        env:
          GITHUB_TOKEN: ${{ secrets.PAT_TOKEN }}
```

If you want to create a tag when you merge your pull request, you can create a new workflow file in your repository.

```yaml
on:
  pull_request:
    types: [closed]
    branches:
      - main
  workflow_dispatch:

jobs:
  create-production-tag-on-merge:
    name: Create tag on main after merge
    runs-on: ubuntu-latest
    if: github.event.workflow_dispatch == true || github.event.pull_request.merged == true && github.event.pull_request.base.ref == 'main' && github.event.pull_request.head.ref == 'production-release'
    steps:
      - name: Checkout main
        uses: actions/checkout@v4
        with:
          fetch-depth: 0
          ref: main

      - name: Create tag with releaser
        run: ./releaser/releaser-linux production --tag

      - name: Push tags
        run: |
          git config user.name github-actions
          git config user.email github-actions@github.com
          git push origin --tags
```

## Building

For the platform you are working on:

```bash
$ cargo build --release
```

For linux:
you need to install `cross` first:

```bash
$ cargo install cross
```

Then run:

```bash
$ cross build --release --target x86_64-unknown-linux-gnu
```

## Create a Github PAT Token

When creating a Personal Access Token (PAT) for use in GitHub Actions, you should carefully consider the permissions needed for your specific workflow. Based on your current workflow requirements, here are the recommended permissions for your PAT:

1. repo (Full control of private repositories)

   - This includes access to code, commit statuses, pull requests, and repository hooks.

2. workflow (Update GitHub Action workflows)

   - This allows the token to modify and trigger workflows.

3. write:packages (Write packages to GitHub Package Registry)

   - If your workflow involves publishing packages.

4. read:org (Read org and team membership, read org projects)

   - This might be necessary if your repository is part of an organization.

5. gist (Create gists)
   - Only if your workflow creates gists.

Here's a step-by-step guide to create a PAT with these permissions:

1. Go to your GitHub account settings.
2. Click on "Developer settings" in the left sidebar.
3. Click on "Personal access tokens" and then "Tokens (classic)".
4. Click "Generate new token" and select "Generate new token (classic)".
5. Give your token a descriptive name, e.g., "GitHub Actions Staging Release".
6. Set an expiration date (consider security implications when setting this).
7. Select the following scopes:
   - repo
   - workflow
   - write:packages
   - read:org
   - gist (if needed)
8. Click "Generate token" at the bottom of the page.
9. Copy the token immediately (you won't be able to see it again).

After generating the token:

1. Go to your repository settings.
2. Click on "Secrets and variables" then "Actions".
3. Click "New repository secret".
4. Name it (e.g., `PAT_TOKEN`) and paste your token as the value.
5. Click "Add secret".

Remember, this token has significant permissions, so keep it secure and don't share it. Also, consider using the principle of least privilege and only grant the permissions that are absolutely necessary for your workflow.

## License

Releaser is licensed under the [MIT License](LICENSE).
