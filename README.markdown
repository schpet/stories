# Stories

stories is a cli client for pivotal tracker for all the story delivery people out there.

drop a `stories.json` file in your project, and voila, you can quickly see your work, start stories, and generate summaries of recent work done.

some commands it offers:

```bash
# equivalent to tracker's "My work" tab
stories mine

# checks out a branch for a story, and marks it as started
stories branch 12345

# print a story to stdout, based on the git branch
stories view

# or, open the website
stories view --web

# you can pass an id instead of git branch
stories view 12345

# show a report of recent changes you've made to stories
stories activity

# show all of the commands
stories --help
```


## Installation

Homebrew (mac):

```sh
brew install schpet/tap/stories
```

Linux and mac binaries also available on [releases](https://github.com/schpet/stories/releases)

Cargo (from source):

```
# in this project's dir
cargo install --path .
```

throw `alias s=stories` in your ~/.zshrc ~/.bashrc for good measure.

## Setup

1. get your api token at https://www.pivotaltracker.com/profile#api
2. write this token into a file at ~/.config/stories/tracker_api_token.txt, e.g.
   ```bash
   mkdir -p ~/.config/stories
   echo $YOUR_TRACKER_API_TOKEN > ~/.config/stories/tracker_api_token.txt
   ```
3. drop a `stories.json` file in your project directory with a tracker project id, e.g.
   ```json
   { "project_id": 1234 }
   ```
## Github integration

Ensure that your pivotal tracker project is setup with the [github integration][tgh] which connects pull requests to tracker stories, and lets you deliver stories [via commit messages][tghc].

Additionally, stories' pull-request command, e.g. 

```bash
stories pr title --summarize
```

are intended to be used with github's [gh cli][gh], e.g.

```
gh pr create --title "$(stories pr title --summarize)" --body "$(stories pr body)" --web

# alias this for convenience:
gh alias set --shell prt "gh pr create --title \"\$(stories pr title --summarize)\" --body \"\$(stories pr body)\" --web"
```

you will also want to configure your repo's settings the following way:

- [ ] Allow merge commits _(optionally disable this)_
- [x] Allow squash merging
   - Default to pull request title and description
- [ ] Allow rebase merging  _(optionally disable this)_

this can be done through the github repo settings page, or via `gh api`:

```bash
gh api repos/{owner}/{repo} --method PATCH -f allow_squash_merge=true -f allow_merge_commit=false -f allow_rebase_merge=false -f squash_merge_commit_title=PR_TITLE -f squash_merge_commit_message=PR_BODY
```

[tgh]: https://www.pivotaltracker.com/help/articles/github_integration/
[tghc]: https://www.pivotaltracker.com/help/articles/github_integration/#using-the-github-integration-commits
[gh]: https://cli.github.com/


## Development

### Releasing a new version

```console
cargo release -x patch
cargo release -x minor
cargo release -x major
```
