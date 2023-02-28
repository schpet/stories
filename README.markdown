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
   echo $YOUR_TRACKER_API_TOKEN > ~/.config/stories/tracker_api_token.txt
   ```
3. drop a `stories.json` file in your project directory with a tracker project id, e.g. 
   ```json
   {"project_id":1234}
   ```
