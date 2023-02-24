use chrono::{DateTime, Local};
use clap::{Parser, Subcommand};
use colored::*;
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};
use slugify::slugify;
use stories::print_markdown;
use tabled::merge::Merge;
use tabled::object::Columns;
use tabled::style::{Style, VerticalLine};
use tabled::{object::Rows, Modify, Table, Tabled, Width};

use anyhow::{anyhow, Context, Result};
use reqwest::header;
use sha256::digest;
use terminal_link::Link;

pub mod api;

use indoc::indoc;
use std::{
    env,
    fs::{self},
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Deserialize, Serialize, Debug)]
struct Story {
    name: String,
    id: u32,
    current_state: StoryState,
    story_type: StoryType,
    url: String,
    #[serde(default)]
    estimate: Option<u32>,
    labels: Vec<Label>,
}

#[derive(Serialize, Deserialize, Debug)]
enum StoryType {
    #[serde(rename = "bug")]
    Bug,
    #[serde(rename = "feature")]
    Feature,
    #[serde(rename = "chore")]
    Chore,
    #[serde(rename = "release")]
    Release,
}

#[derive(Serialize, Deserialize, Debug)]
enum StoryState {
    #[serde(rename = "accepted")]
    Accepted,
    #[serde(rename = "delivered")]
    Delivered,
    #[serde(rename = "finished")]
    Finished,
    #[serde(rename = "started")]
    Started,
    #[serde(rename = "rejected")]
    Rejected,
    #[serde(rename = "planned")]
    Planned,
    #[serde(rename = "unstarted")]
    Unstarted,
    #[serde(rename = "unscheduled")]
    Unscheduled,
}

#[derive(Deserialize, Serialize, Debug)]
struct StoryDetail {
    name: String,
    id: u32,
    current_state: StoryState,
    story_type: StoryType,
    url: String,
    #[serde(default)]
    estimate: Option<u32>,
    labels: Vec<Label>,
    description: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct Label {
    id: u64,
    project_id: u64,
    kind: String,
    name: String,
    created_at: String,
    updated_at: String,
}

#[derive(Tabled, Debug)]
struct StoryRow {
    #[tabled(rename = "Id")]
    id: String,
    #[tabled(rename = "⛬")]
    story_type: String,
    #[tabled(rename = " ☑ ")]
    current_state: ColoredString,
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "")]
    actions: String,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    // /// Sets a custom config file
    // #[arg(short, long, value_name = "FILE")]

    // config: Option<PathBuf>,
    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::ValueEnum, Clone)]
enum PrField {
    Body,
    Title,
}

#[derive(Subcommand)]
enum Commands {
    /// Displays the current story, also aliased as "show"
    #[clap(alias = "show")]
    View {
        /// Optionally provide a story id, otherwise find it in the current git branch
        story_id: Option<u64>,

        /// Open the story in a web browser
        #[arg(short, long)]
        web: bool,
    },

    /// Checks out a git branch and changes the story's state to started
    Branch {
        story_id: u64,

        /// Optionally provide a different branch name prefix, defaults to story name
        #[arg(short, long)]
        name: Option<String>,

        #[arg(short, long)]
        estimate: Option<u8>,
    },

    /// Print out suggested pull request title or body
    Pr {
        #[arg(value_enum)]
        field: PrField,

        /// Optionally provide a story id, otherwise find it in the current git branch
        story_id: Option<u64>,
    },

    /// Stories assigned to you
    Mine {
        /// Print json response
        #[arg(short, long)]
        json: bool,
    },

    /// Currently authenticated user
    Whoami {},

    /// Recent things you have done on tracker
    Activity {},
    // ideas:
    // standup
    // - lists stories recently completed
    // - recent commits to main branch

    // cache clear
}

#[derive(Debug, Deserialize)]
struct ProjectConfig {
    project_id: u64,
}

fn print_result(result: Result<String, anyhow::Error>) {
    match result {
        Ok(value) => {
            println!("{}", value);
            std::process::exit(0);
        }
        Err(err) => {
            eprintln!("{}\n", "uh oh!".red().bold());
            let message = format!("{:?}", err);
            eprintln!("{}", message.red());
            std::process::exit(1);
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::View { story_id, web }) => print_result(view(*story_id, *web).await),
        Some(Commands::Mine { json }) => {
            print_result(mine(*json).await);
        }
        Some(Commands::Whoami {}) => {
            print_result(whoami().await);
        }
        Some(Commands::Branch {
            story_id,
            name,
            estimate,
        }) => {
            print_result(branch(*story_id, name, *estimate).await);
        }
        Some(Commands::Pr {
            field: _,
            story_id: _,
        }) => {
            todo!()
        }

        Some(Commands::Activity {}) => {
            print_result(activity().await);
        }

        None => {}
    }

    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Me {
    id: u64,
    name: String,
    initials: String,
    username: String,
    email: String,
}

pub async fn whoami() -> anyhow::Result<String> {
    let data = tracker_me().await?;
    Ok(format!("you: {}", data.email))
}

pub async fn tracker_api_client() -> anyhow::Result<reqwest::Client> {
    static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

    let token = read_api_token()?;

    let mut headers = header::HeaderMap::new();
    let mut api_token_value = header::HeaderValue::from_str(&token)?;
    api_token_value.set_sensitive(true);

    headers.insert("X-TrackerToken", api_token_value);

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .default_headers(headers)
        .build()?;

    Ok(client)
}

pub async fn branch(
    story_id: u64,
    name: &Option<String>,
    estimate: Option<u8>,
) -> anyhow::Result<String> {
    let client = tracker_api_client().await?;
    let project_id = read_project_id()?;

    let story_url = format!(
        "https://www.pivotaltracker.com/services/v5/projects/{}/stories/{}",
        project_id, story_id
    );

    let data: StoryDetail = client.get(&story_url).send().await?.json().await?;
    // let me = tracker_me().await?;

    // let story_type = match &data.story_type {
    //     StoryType::Bug => "bug",
    //     StoryType::Feature => "feat",
    //     StoryType::Chore => "chore",
    //     StoryType::Release => "release",
    // };

    let name_formatted = match &name {
        Some(val) => format!("-{}", val),
        None => data.name,
    };

    let branch_name = format!("{}-{}", slugify!(&name_formatted, max_length = 30), data.id,);

    let git_result = Command::new("git")
        .arg("switch")
        .arg("-c")
        .arg(&branch_name)
        .spawn()?
        .wait();

    // nicer way of doing this?
    if git_result.is_err() {
        return Err(git_result.unwrap_err().into());
    }

    let mut map = Map::new();
    map.insert(
        "current_state".to_string(),
        Value::String("started".to_string()),
    );

    if estimate.is_some() {
        map.insert(
            "estimate".to_string(),
            Value::Number(Number::from(estimate.unwrap())),
        );
    }

    let response = client.put(&story_url).json(&map).send().await?;

    let status = response.status();
    let response_text = response.text().await?;

    if !status.is_success() {
        return Err(anyhow!(format!("{}\n\n{}", status, response_text)));
    };

    // let data: &StoryDetail = &response.json().await?;
    let data = serde_json::from_str::<StoryDetail>(&response_text)?;

    // TODO: if a feature is not pointed, there will be a serialization error here

    Ok(format!(
        "updated story {} and checked out branch '{}'",
        data.id, branch_name
    ))
}

async fn tracker_me() -> anyhow::Result<Me> {
    let token = read_api_token()?;
    let client = tracker_api_client().await?;

    let dir = Path::new(&config_dir()?).join("cache");
    let cache_key = format!("tracker::me::{}", digest(&*token));

    let cached = cacache::read(&dir, &cache_key).await;

    match &cached {
        Ok(cached) => Ok(serde_json::from_slice(cached)?),
        Err(_) => {
            let data: Me = client
                .get("https://www.pivotaltracker.com/services/v5/me")
                .send()
                .await?
                .json()
                .await?;

            let bytes = serde_json::to_vec(&data)?;

            cacache::write(&dir, &cache_key, bytes).await?;

            Ok(data)
        }
    }
}

#[derive(Tabled, Debug)]
struct ActivityRow {
    #[tabled(rename = "Date")]
    date: String,
    #[tabled(rename = "Story")]
    name: String,
    #[tabled(rename = "Changes")]
    highlights: String,
}

async fn activity() -> anyhow::Result<String> {
    let client = tracker_api_client().await?;
    let project_id = read_project_id()?;

    let activities: Vec<api::schema::Activity> = client
        .get("https://www.pivotaltracker.com/services/v5/my/activity")
        .send()
        .await?
        .json()
        .await?;

    let mut rows: Vec<ActivityRow> = Vec::new();

    activities
        .into_iter()
        .filter(|a| a.project.id == project_id)
        .filter(|a| a.kind == "story_update_activity")
        .group_by(|a| {
            let datetime_utc = DateTime::parse_from_rfc3339(&a.occurred_at).unwrap();
            datetime_utc.with_timezone(&Local).date_naive()
        })
        .into_iter()
        .for_each(|(date, activities_by_date)| {
            activities_by_date
                .sorted_by(|a, b| {
                    a.primary_resources[0]
                        .id
                        .partial_cmp(&b.primary_resources[0].id)
                        .unwrap()
                })
                .group_by(|a| {
                    format!(
                        "{} {}",
                        a.primary_resources[0].id, a.primary_resources[0].name
                    )
                })
                .into_iter()
                .for_each(|(story_label, activities_for_story)| {
                    let highlights = activities_for_story
                        .sorted_by(|a, b| a.occurred_at.partial_cmp(&b.occurred_at).unwrap())
                        .map(|a| match a.highlight.as_str() {
                            "delivered" => "delivered".green().to_string(),
                            "finished" => "finished".cyan().to_string(),
                            "started" => "started".blue().to_string(),
                            other => other.to_string(),
                        })
                        .collect::<Vec<String>>()
                        .join(", ");

                    rows.push(ActivityRow {
                        date: date.format("%a %b %d").to_string(),
                        name: story_label,
                        highlights,
                    });
                });
        });

    let mut table = Table::new(&rows);

    let (terminal_size::Width(width), terminal_size::Height(_height)) =
        terminal_size::terminal_size().unwrap();

    // table.with(Modify::new(Columns::first()).with(Width::increase(15)).with(Width::wrap(15)));
    table.with(Modify::new(Columns::last()).with(Width::wrap(30).keep_words()));

    table
        .with(Merge::horizontal())
        .with(Merge::vertical())
        .with(Style::modern())
        .with(Width::wrap(width as usize).keep_words())
        .with(Width::increase(width as usize));

    Ok(table.to_string())
}

pub async fn view(story_id: Option<u64>, web: bool) -> anyhow::Result<String> {
    let branch_id = match story_id {
        Some(id) => id.to_string(),
        None => read_branch_id()?,
    };

    if web {
        let url = format!("https://www.pivotaltracker.com/story/show/{}", branch_id);
        webbrowser::open(&url)?;
        return Ok(format!("opened {}", url));
    }

    let client = tracker_api_client().await?;

    let project_id = read_project_id()?;

    let url = format!(
        "https://www.pivotaltracker.com/services/v5/projects/{}/stories/{}",
        project_id, branch_id
    );

    let data: StoryDetail = client.get(url).send().await?.json().await?;

    let doc = format!("# {}\n\n{}", data.name, data.description);

    print_markdown(&doc)?;

    Ok("".to_string())
}

fn format_current_state(state: &StoryState) -> ColoredString {
    match state {
        StoryState::Planned => "---".black(),
        StoryState::Unscheduled => "---".black(),
        StoryState::Unstarted => "···".black(),
        StoryState::Started => "☐☐☐".blue(),
        StoryState::Finished => "☑☐☐".cyan(),
        StoryState::Delivered => "☑☑☐".green(),
        StoryState::Accepted => "☑☑☑".green(),
        StoryState::Rejected => "☑☑☒".red(),
    }
}

pub async fn mine(json: bool) -> anyhow::Result<String> {
    let client = tracker_api_client().await?;
    let project_id = read_project_id()?;
    let me = tracker_me().await?;

    // https://www.pivotaltracker.com/help/api/rest/v5#Stories
    let url = format!(
        "https://www.pivotaltracker.com/services/v5/projects/{}/stories?filter=mywork:{}",
        project_id, me.id
    );

    let response = client.get(url).send().await?;

    if json {
        return Ok(response.text().await?);
    }

    let data: Vec<Story> = response.json().await?;

    let rows: Vec<StoryRow> = data
        .into_iter()
        .map(|entry| {
            let link = Link::new("[↗]", &entry.url).to_string();

            let story_type = match entry.story_type {
                StoryType::Feature => "⭐️".to_string(),
                StoryType::Bug => "🐞".to_string(),
                StoryType::Chore => "🧹".to_string(),
                StoryType::Release => "🏁".to_string(),
            };

            // todo clean this up, use something cool, e.g. ➊➁➍
            // https://en.wikipedia.org/wiki/List_of_Unicode_characters
            let estimate = match entry.estimate.as_ref() {
                Some(value) => format!(" · {}", value.to_string().yellow()),
                None => match entry.story_type {
                    StoryType::Feature => " · ⚠".to_string(),
                    _ => "".to_string(),
                },
            };

            let labels = entry
                .labels
                .into_iter()
                .map(|label| format!("{}", label.name.black().italic()))
                .collect::<Vec<String>>()
                .join(", ");

            let name = match labels.len() {
                0 => format!("{} {}", entry.name, estimate),
                _ => format!("{} · {}{}", entry.name, labels, estimate),
            };

            StoryRow {
                id: entry.id.to_string(),
                story_type,
                current_state: format_current_state(&entry.current_state),
                name,
                actions: link,
            }
        })
        .collect();

    let (terminal_size::Width(term_width), terminal_size::Height(_height)) =
        terminal_size::terminal_size().unwrap();
    let width = term_width as usize; // todo why casting?
    let name_wrap = match width > 80 {
        true => 44 + (width - 80),
        false => 44,
    };

    let style = Style::modern()
        .off_vertical()
        .verticals([VerticalLine::new(3, Style::modern().get_vertical())]);

    let mut table = Table::new(&rows);
    table
        .with(style)
        .with(Modify::new(Rows::new(1..)).with(Width::wrap(name_wrap).keep_words()));

    Ok(table.to_string())
}

pub fn read_project_id() -> anyhow::Result<u64> {
    let contents = fs::read_to_string("stories.json").with_context(|| {
        indoc! {r#"
            didn't find a stories.json in this directory.

            add one with something like this:

                $ echo '{"project_id":123456}' > stories.json

            to find a project id, visit the project in the tracker website and look at the url
            https://www.pivotaltracker.com/dashboard
        "#}
    })?;

    let config: ProjectConfig =
        serde_json::from_str(&contents).context("stories.json isn't right")?;
    Ok(config.project_id)
}

fn config_dir() -> anyhow::Result<PathBuf> {
    let home = env::var("HOME").context("no $HOME env var defined? wacky")?;
    let path = Path::new(&home).join(".config/stories");
    Ok(path)
}

pub fn read_api_token() -> anyhow::Result<String> {
    let path = Path::new(&config_dir()?).join("tracker_api_token.txt");

    let path_string = path.as_os_str().to_str().unwrap();

    let token_file_contents = fs::read_to_string(&path).context(format!(
        indoc! {"
            didn't find the credentials config file at {}

            1. visit https://www.pivotaltracker.com/profile#api
            2. note that token!
            3. run something like this to dump it in

                $ echo $YOUR_API_TOKEN > {}
        "},
        path_string, path_string
    ))?;

    Ok(token_file_contents.trim().to_string())
}

pub fn read_branch_id() -> anyhow::Result<String> {
    // possible enhancement: drag in a git crate to do this, and can likely
    // a) get this from any dir in the git repo
    // b) possibly avoid shelling out to git to create new branches

    let head_contents = fs::read_to_string(".git/head")
        .context("failed to read .git/head, are you in the root of a git repo?")?;

    let branch =
        branch_name(&head_contents).ok_or_else(|| anyhow!("no branch name found in .git/head"))?;

    let id = extract_id(&branch).ok_or_else(|| {
        anyhow!(format!(
            indoc! {r#"
                the current git branch doesn't appear to have an id in it.

                    current branch:
                        {}

                    what we're looking for:
                        12345-some-feature

                run the following to checkout a relevant branch for a story

                    $ stories start 12345
            "#},
            branch
        ))
    })?;

    Ok(id)
}

pub fn branch_name(head_contents: &str) -> Option<String> {
    Some(
        head_contents
            .strip_prefix("ref: refs/heads/")?
            .trim()
            .to_string(),
    )
}

pub fn extract_id(branch_name: &str) -> Option<String> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(?P<story_id>\d+)").unwrap();
    }
    RE.captures(branch_name)
        .and_then(|cap| cap.name("story_id").map(|bid| bid.as_str().to_string()))
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_branch_name() {
        assert_eq!(
            branch_name("ref: refs/heads/main"),
            Some("main".to_string())
        );
    }

    #[test]
    fn test_extract_id() {
        assert_eq!(extract_id("123-yep"), Some("123".to_string()));
        assert_eq!(extract_id("123-456-yep"), Some("123".to_string()));
        assert_eq!(extract_id("foobar"), None);
    }
}
