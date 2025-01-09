# Releaser

A tool to create releases for your Node.js projects.
It works like a charm with monorepos!

Inspired by [release-please](https://github.com/googleapis/release-please), Releaser is a CLI tool that automates the process of creating releases for your Node.js projects. It works with monorepos and follows the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) specification.

Advantages of Releaser over other release automation tools:

- **Speed**: Releaser is optimized for speed, allowing you to create releases quickly and efficiently. It need just a few seconds locally to run the github action! By comparison, other release automation tools can take several minutes to complete the release process.
- **Monorepo support**: Releaser is designed to work with monorepos, allowing you to manage releases for multiple packages within a single repository.
- **Conventional Commits support**: Releaser adheres to the Conventional Commits specification, ensuring that your release notes are structured and easy to understand.

## Features

- Create releases for your Node.js projects.
- Supports monorepos.
- Follows the Conventional Commits specification.
- Automatically creates Git tags for the new versions.
- Automatically updates the CHANGELOG.md file.
- Automatically updates the package.json file.
- Automatically updates the version in extra files.
- Optionally creates a pull request with the release notes.
- Optionally creates a tag for the new versions and deploys the new version to a specified environment.

> **Note:** While Releaser currently supports Node.js projects, support for other languages could be added in the future through custom adapters. Stay tuned for updates!

## Setup

```bash
$ npm install -g releaser-cli
```

Create a `releaser-manifest.json` file in the root of your project.

```json
[
  {
    "path": "packages/api", // path to the package
    "extraFiles": ["packages/api/index.js"] // optional: extra files to be updated. You need to comment // x-releaser-version on the lines you want to update
    "dependencies": ["shared"] // optional: name of the packages that this package depends on
  },
  {
    "path": "packages/shared"
  }
]
```

See the [example](./releaser-manifest.json) for a complete example.

## Usage

```bash
$ releaser [environment]
```

Options:
--tag: Create a tag for the new versions
--dry-run: Dry run mode. No changes will be made.

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
        run: npx -y releaser-cli production

      - name: Push
        run: |
          git push -f origin production-release
          PR_EXISTS=$(gh pr list --head production-release --base main --json number --jq length)
          if [ "$PR_EXISTS" -eq "0" ]; then
            gh pr create --title "Production Release" --body-file ./pull_request_content.md --base main --head production-release
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
    inputs:
      environment:
        description: "Environment to create tag for (staging or production)"
        required: true
        default: "staging"
        type: choice
        options:
          - staging
          - production

permissions:
  contents: write

name: releaser-tag
jobs:
  create-tag:
    name: Create tag on main after merge or manual trigger
    runs-on: ubuntu-latest
    if: |
      (github.event_name == 'workflow_dispatch') ||
      (github.event_name == 'pull_request' &&
       github.event.pull_request.merged == true &&
       github.event.pull_request.base.ref == 'main' &&
       (github.event.pull_request.head.ref == 'staging-release' ||
        github.event.pull_request.head.ref == 'production-release'))
    outputs:
      tag_created: ${{ steps.push_tags.outputs.tag_created }}
      api_tag_created: ${{ steps.push_tags.outputs.api_tag_created }}
      environment: ${{ steps.determine_env.outputs.ENVIRONMENT }}
    steps:
      - name: Checkout main
        uses: actions/checkout@v4
        with:
          fetch-depth: 0
          ref: main

      - name: Determine environment
        id: determine_env
        run: |
          if [[ "${{ github.event_name }}" == "workflow_dispatch" ]]; then
            echo "ENVIRONMENT=${{ github.event.inputs.environment }}" >> $GITHUB_ENV
          elif [[ "${{ github.event.pull_request.head.ref }}" == "staging-release" ]]; then
            echo "ENVIRONMENT=staging" >> $GITHUB_ENV
          else
            echo "ENVIRONMENT=production" >> $GITHUB_ENV
          fi

          echo "ENVIRONMENT=${{ env.ENVIRONMENT }}" >> $GITHUB_OUTPUT

      - name: Git config
        run: |
          git config user.name github-actions
          git config user.email github-actions@github.com

      - name: Create tag with releaser
        run: npx -y releaser-cli ${{ env.ENVIRONMENT }} --tag

      - name: Push tags
        id: push_tags
        env:
          GITHUB_TOKEN: ${{ secrets.PAT_TOKEN }}
        run: |
          if git push origin --tags; then
            echo "tag_created=true" >> $GITHUB_OUTPUT
            if grep -q "api" tags_to_create.txt; then
              echo "api_tag_created=true" >> $GITHUB_OUTPUT
            else
              echo "api_tag_created=false" >> $GITHUB_OUTPUT
            fi
          else
            echo "tag_created=false" >> $GITHUB_OUTPUT
          fi

  deploy-api-staging:
    needs: create-tag
    if: needs.create-tag.outputs.api_tag_created == 'true' && needs.create-tag.outputs.environment == 'staging'
    uses: ./.github/workflows/YOUR_STAGING_WORKFLOW.yml
    secrets: inherit

  deploy-api-production:
    needs: create-tag
    if: needs.create-tag.outputs.api_tag_created == 'true' && needs.create-tag.outputs.environment == 'production'
    uses: ./.github/workflows/YOUR_PRODUCTION_WORKFLOW.yml
    secrets: inherit
```

## Building

### Automatic build

The `build.sh` script will build the releaser binary for all platforms.

### Manual build

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

## Configuring a Github Personal Access Token

The following link can be used as a shortcut to create your token: https://github.com/settings/tokens/new?description=Releaser&scopes=repo,workflows

Otherwise, you can manually create it:

Go to https://github.com/settings/tokens

For Classic Token:

1. Click "Generate new token"
2. Give your token a descriptive name
3. Select the following scopes:
   - `repo` (Full control of private repositories)
   - `workflow` (if you need to trigger workflows)
4. Click "Generate token".
5. Copy the token

For Fine Grained Token:

1. Click "Generate new token"
2. Give your token a descriptive name
3. Select the following scopes:
   - contents
   - pull-requests
   - issues
4. Click "Generate token".
5. Copy the token

After generating the token:

1. Go to your repository settings.
2. Click on "Secrets and variables" then "Actions".
3. Click "New repository secret".
4. Name it (e.g., `PAT_TOKEN`) and paste your token as the value.
5. Click "Add secret".

Remember, this token has significant permissions, so keep it secure and don't share it. Also, consider using the principle of least privilege and only grant the permissions that are absolutely necessary for your workflow.

## License

Releaser is licensed under the [MIT License](LICENSE).
