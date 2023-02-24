# Stories

stories is a cli client for pivotal tracker for all the story delivery people out there.

drop a `stories.json` file in your project, and voila, you can quickly see your work, start stories, and generate summaries of recent work done.

some commands it offers:

```bash
# equivalent to tracker's "My work" tab
stories mine

# checks out a branch for a story, and marks it as started
stories start 12345

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

Homebrew:

```sh
brew install --cask ...todo...
```

From source:

```
git clone <this repo>
cd stories
cargo build release
```

throw `alias s=stories` in your ~/.zshrc ~/.bashrc for good measure.
