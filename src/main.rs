#![forbid(unsafe_code)]

// dear reader:
//
// the following is my 2nd rust program, and it panics less than my first one,
// but is still very amateur level stuff and thrown together without much
// consideration for aesthetics.

use api::schema::{StoryState, StoryType};
use chrono::{DateTime, Local};
use clap::{Args, Parser, Subcommand};
use colored::*;
use itertools::Itertools;
use mdcat::push_tty;
use mdcat::terminal::TerminalProgram;
use reqwest::header::USER_AGENT;
use serde::Deserialize;
use serde_json::{Map, Number, Value};
use slugify::slugify;

use mdcat::Settings;
use pulldown_cmark::Options;
use syntect::parsing::SyntaxSet;
use tabled::object::Rows;
use tabled::style::{Style, VerticalLine};
use tabled::{Modify, Table, Tabled, Width};

use anyhow::{anyhow, Context, Result};
use reqwest::header;
use sha256::digest;
use terminal_link::Link;

use async_openai::types::{
    ChatCompletionRequestMessageArgs, CreateChatCompletionRequestArgs, Role,
};

use indoc::indoc;
use std::{
    env,
    fs::{self},
    path::{Path, PathBuf},
    process::Command,
};

pub mod api;

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
    View(ViewArgs),

    /// Checks out a git branch and changes the story's state to started
    #[clap(alias = "br")]
    Branch(BranchArgs),

    /// Print out suggested pull request title or body. Aliased as `pr`
    ///
    /// Pairs well with the github's "gh" cli https://cli.github.com/
    ///
    /// $ gh pr create --title "$(stories pr title)" --body "$(stories pr body)"
    /// $ gh pr edit --body "$(stories pr body)"
    #[clap(alias = "pr")]
    PullRequest(PrArgs),

    /// Stories assigned to you
    Mine(MineArgs),

    /// Currently authenticated user
    Whoami {},

    /// Recent things you have done on tracker
    Activity {},
    // future commands:
    // - cache clear (handle changes to tracker Me record)
}

#[derive(Debug, Deserialize)]
struct ProjectConfig {
    project_id: u64,
}

fn print_result(result: Result<(), anyhow::Error>) {
    match result {
        Ok(_) => {
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
        Some(Commands::View(view_args)) => {
            print_result(view(view_args).await);
        }
        Some(Commands::Mine(mine_args)) => {
            print_result(mine(mine_args).await);
        }
        Some(Commands::Whoami {}) => {
            print_result(whoami().await);
        }
        Some(Commands::Branch(branch_args)) => {
            print_result(branch(branch_args).await);
        }
        Some(Commands::PullRequest(pr_args)) => {
            print_result(pull_request(pr_args).await);
        }
        Some(Commands::Activity {}) => {
            print_result(activity().await);
        }

        None => {}
    }

    Ok(())
}

#[derive(Args)]
pub struct PrArgs {
    #[arg(value_enum)]
    field: PrField,

    /// Optionally provide a story id, otherwise find it in the current git branch
    story_id: Option<u64>,

    #[arg(short, long)]
    /// Automatically summarize the field with chatgpt
    summarize: bool,
}

pub async fn pull_request(pr_args: &PrArgs) -> anyhow::Result<()> {
    let story_id = match pr_args.story_id {
        Some(id) => id,
        None => read_branch_id()?,
    };

    let project_id = read_project_id()?;
    let story_url = format!(
        "https://www.pivotaltracker.com/services/v5/projects/{}/stories/{}",
        project_id, story_id
    );

    let client = tracker_api_client().await?;
    let story: api::schema::StoryDetail = client.get(&story_url).send().await?.json().await?;

    let conventional_commit_type = match &story.story_type {
        StoryType::Bug => "fix",
        StoryType::Feature => "feat",
        StoryType::Chore => "chore",
        StoryType::Release => "chore",
    };

    match pr_args.field {
        PrField::Body => {
            let message = format!(
                indoc! {r#"
                    {}

                    --------

                    Tracker: [delivers #{}]"#},
                story.url, story.id,
            );
            println!("{}", message);
            Ok(())
        }
        PrField::Title => {
            if pr_args.summarize {
                let client = open_ai_client()?;
                let action = match &story.story_type {
                    StoryType::Bug => "fixes",
                    StoryType::Feature => "implements",
                    _ => "addresses",
                };

                let prompt = format!(
                    "Write a git commit subject for a change that {} this:",
                    action
                );
                let full_prompt = format!("{}\n{}\n\n{}", prompt, story.name, story.description);

                let request = CreateChatCompletionRequestArgs::default()
                    .model("gpt-3.5-turbo")
                    .messages([
                        ChatCompletionRequestMessageArgs::default()
                            .role(Role::Assistant)
                            .content( "You summarize tasks into git commit messages. You follow the git conventional commit specification by prefixing features and bugs with feat: and bug: respectively, and omit the optional scope.")
                            .build()?,
                        ChatCompletionRequestMessageArgs::default()
                            .role(Role::User)
                            .content(full_prompt)
                            .build()?,
                    ])
                    .max_tokens(40_u16)
                    .build()?;

                let response = client.chat().create(request).await?;

                let first_choice = response
                    .choices
                    .first()
                    .ok_or_else(|| anyhow!("didn't get a choice back from openai"))?;

                println!("{}", first_choice.message.content);
            } else {
                println!("{}: {}", conventional_commit_type, story.name);
            }
            Ok(())
        }
    }
}

fn open_ai_client() -> anyhow::Result<async_openai::Client> {
    let api_key = read_open_ai_secret_key()?;
    Ok(async_openai::Client::new().with_api_key(api_key))
}

fn read_open_ai_secret_key() -> anyhow::Result<String> {
    let path = Path::new(&config_dir()?).join("open_ai_secret_key.txt");

    let path_string = path.as_os_str().to_str().unwrap();

    let token_file_contents = fs::read_to_string(&path).context(format!(
        indoc! {"
            didn't find the credentials config file at {}

            1. visit https://platform.openai.com/account/api-keys
            2. create a new secret
            3. run something like this to dump it in

                $ echo $YOUR_API_TOKEN > {}
        "},
        path_string, path_string
    ))?;

    Ok(token_file_contents.trim().to_string())
}

pub async fn whoami() -> anyhow::Result<()> {
    let data = tracker_me().await?;
    println!("{:?}", data);
    Ok(())
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

#[derive(Args)]
pub struct BranchArgs {
    story_id: u64,

    /// Optionally provide a different branch name prefix, defaults to story name
    #[arg(short, long)]
    name: Option<String>,

    #[arg(short, long)]
    estimate: Option<u8>,
}

pub async fn branch(branch_args: &BranchArgs) -> anyhow::Result<()> {
    let client = tracker_api_client().await?;
    let project_id = read_project_id()?;

    let story_url = format!(
        "https://www.pivotaltracker.com/services/v5/projects/{}/stories/{}",
        project_id, branch_args.story_id
    );

    let data: api::schema::StoryDetail = client.get(&story_url).send().await?.json().await?;
    // let me = tracker_me().await?;

    // let story_type = match &data.story_type {
    //     StoryType::Bug => "bug",
    //     StoryType::Feature => "feat",
    //     StoryType::Chore => "chore",
    //     StoryType::Release => "release",
    // };

    let name_formatted = match &branch_args.name {
        Some(val) => format!("-{}", val),
        None => data.name,
    };

    let branch_name = format!("{}-{}", slugify!(&name_formatted, max_length = 40), data.id);

    let git_result = Command::new("git")
        .arg("switch")
        .arg("-c")
        .arg(&branch_name)
        .output();

    let git_message = match git_result {
        Ok(result) => {
            if result.status.success() {
                format!("checked out branch successfully: {}", branch_name)
            } else {
                format!(
                    "🔥 creating git branch failed, does it already exist? try\n\n\tgit switch {}",
                    branch_name
                )
            }
        }
        Err(err) => {
            // this right?
            return Err(err.into());
        }
    };

    let mut map = Map::new();
    map.insert(
        "current_state".to_string(),
        Value::String("started".to_string()),
    );

    if branch_args.estimate.is_some() {
        map.insert(
            "estimate".to_string(),
            Value::Number(Number::from(branch_args.estimate.unwrap())),
        );
    }

    let response = client.put(&story_url).json(&map).send().await?;

    let status = response.status();
    let response_text = response.text().await?;

    if !status.is_success() {
        return Err(anyhow!(format!("{}\n\n{}", status, response_text)));
    };

    let data = serde_json::from_str::<api::schema::StoryDetail>(&response_text)?;

    // TODO: if a feature is not pointed, there will be a serialization error here

    println!("{}\nupdated story {}", git_message, data.id);

    Ok(())
}

async fn tracker_me() -> anyhow::Result<api::schema::Me> {
    let token = read_api_token()?;
    let client = tracker_api_client().await?;

    let dir = Path::new(&config_dir()?).join("cache");
    let cache_key = format!("tracker::me::{}", digest(&*token));

    let cached = cacache::read(&dir, &cache_key).await;

    match &cached {
        Ok(cached) => Ok(serde_json::from_slice(cached)?),
        Err(_) => {
            let data: api::schema::Me = client
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

async fn activity() -> anyhow::Result<()> {
    #[derive(Tabled, Debug)]
    struct ActivityRow {
        #[tabled(rename = "Story")]
        name: String,
        #[tabled(rename = "Changes")]
        highlights: String,
    }

    let client = tracker_api_client().await?;
    let project_id = read_project_id()?;

    let activities: Vec<api::schema::Activity> = client
        .get("https://www.pivotaltracker.com/services/v5/my/activity")
        .send()
        .await?
        .json()
        .await?;

    activities
        .into_iter()
        .rev()
        .filter(|a| a.project.id == project_id)
        .filter(|a| a.kind == "story_update_activity")
        .filter(|a| {
            matches!(
                a.highlight.as_str(),
                "delivered" | "started" | "finished" | "rejected" | "accepted"
            )
        })
        .group_by(|a| {
            let datetime_utc = DateTime::parse_from_rfc3339(&a.occurred_at).unwrap();
            datetime_utc.with_timezone(&Local).date_naive()
        })
        .into_iter()
        .for_each(|(date, activities_by_date)| {
            println!("{}\n----------\n", date.format("%a %b %d"));
            activities_by_date
                .sorted_by(|a, b| {
                    a.primary_resources[0]
                        .id
                        .partial_cmp(&b.primary_resources[0].id)
                        .unwrap()
                })
                .group_by(|a| a.primary_resources[0].name.to_string())
                .into_iter()
                .for_each(|(story_label, activities_for_story)| {
                    let highlights = activities_for_story
                        .sorted_by(|a, b| a.occurred_at.partial_cmp(&b.occurred_at).unwrap())
                        .map(|a| match a.highlight.as_str() {
                            "delivered" => "delivered".green().to_string(),
                            "finished" => "finished".cyan().to_string(),
                            other => other.to_string(),
                        })
                        .collect::<Vec<String>>()
                        .join(", ");

                    println!("{}\n  └── {}", story_label.trim(), highlights.italic());
                });

            println!();
        });

    Ok(())
}

#[derive(Args)]
pub struct ViewArgs {
    /// Optionally provide a story id, otherwise find it in the current git branch
    story_id: Option<u64>,

    /// Open the story in a web browser
    #[arg(short, long)]
    web: bool,

    /// Print json response
    #[arg(short, long)]
    json: bool,
}

pub async fn view(view_args: &ViewArgs) -> anyhow::Result<()> {
    let branch_id = match view_args.story_id {
        Some(id) => id,
        None => read_branch_id()?,
    };

    if view_args.web {
        let url = format!("https://www.pivotaltracker.com/story/show/{}", branch_id);
        webbrowser::open(&url)?;
        println!("opened {}", url);
        return Ok(());
    }

    let client = tracker_api_client().await?;

    let project_id = read_project_id()?;

    let url = format!(
        "https://www.pivotaltracker.com/services/v5/projects/{}/stories/{}",
        project_id, branch_id
    );

    let response = client.get(url).send().await?;

    if view_args.json {
        println!("{}", response.text().await?);
        return Ok(());
    }

    let data: api::schema::MaybeStoryDetail = response.json().await?;

    match data {
        api::schema::MaybeStoryDetail::StoryDetail(sd) => {
            let view_on_web = format!("View this story on Tracker: {}", sd.url);
            let doc = format!("# {}\n\n{}\n\n\n{}", sd.name, sd.description, view_on_web.truecolor(200, 200, 200));
            print_markdown(&doc)?;
        }
        api::schema::MaybeStoryDetail::ApiError(why) => {
            return Err(anyhow::anyhow!(format!(
                "tracker api error: {}\n\n{}",
                why.code, why.error
            )))
        }
    }

    Ok(())
}

#[derive(Args)]
pub struct MineArgs {
    /// Print json response
    #[arg(short, long)]
    json: bool,
}

pub async fn mine(mine_args: &MineArgs) -> anyhow::Result<()> {
    let client = tracker_api_client().await?;
    let project_id = read_project_id()?;
    let me = tracker_me().await?;

    // https://www.pivotaltracker.com/help/api/rest/v5#Stories
    let url = format!(
        "https://www.pivotaltracker.com/services/v5/projects/{}/stories?filter=mywork:{}",
        project_id, me.id
    );

    let response = client.get(url).send().await?;

    if mine_args.json {
        println!("{}", response.text().await?);
        return Ok(());
    }

    let data: Vec<api::schema::Story> = response.json().await?;

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

    let size = terminal_size::terminal_size();

    let term_width: usize = match size {
        Some(size_v) => {
            let (terminal_size::Width(w), _) = size_v;
            w.into()
        }
        None => 120,
    };

    let name_wrap: usize = match term_width > 80 {
        true => 44 + (term_width - 80),
        false => 44,
    };

    let style = Style::modern()
        .off_vertical()
        .verticals([VerticalLine::new(3, Style::modern().get_vertical())]);

    let mut table = Table::new(&rows);
    table
        .with(style)
        .with(Modify::new(Rows::new(1..)).with(Width::wrap(name_wrap).keep_words()));

    println!("{}", table);
    Ok(())
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

pub fn read_branch_id() -> anyhow::Result<u64> {
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
                        some-feature-12345

                run the following to create a relevant branch for a story, and set the story's
                status to "started"

                    $ stories branch 12345

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

pub fn extract_id(branch_name: &str) -> Option<u64> {
    // lazy_static! {
    //     static ref RE: Regex = Regex::new(r"(?P<story_id>\d+)").unwrap();
    // }
    // RE.captures(branch_name)
    //     .and_then(|cap| cap.name("story_id").map(|bid| bid.as_str().to_string()))

    branch_name
        .split(|c: char| !c.is_numeric())
        .filter_map(|s| s.parse::<u64>().ok())
        .last()
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
        assert_eq!(extract_id("yep-123"), Some(123));
        assert_eq!(extract_id("yep-24-123"), Some(123));
        assert_eq!(extract_id("123-456-yep"), Some(456));
        assert_eq!(extract_id("foobar"), None);
    }
}

fn print_markdown(text: &str) -> anyhow::Result<()> {
    let parser = pulldown_cmark::Parser::new_ext(
        text,
        Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH,
    );

    let settings: Settings = mdcat::Settings {
        terminal_capabilities: TerminalProgram::Ansi.capabilities(),
        terminal_size: mdcat::terminal::TerminalSize::default(),
        resource_access: mdcat::ResourceAccess::LocalOnly,
        syntax_set: SyntaxSet::load_defaults_newlines(),
    };
    let env = mdcat::Environment::for_local_directory(&std::env::current_dir().unwrap()).unwrap();

    let stdout = std::io::stdout();
    let mut output = stdout.lock();

    push_tty(&settings, &env, &mut output, parser).or_else(|error| {
        if error.kind() == std::io::ErrorKind::BrokenPipe {
            Ok(())
        } else {
            Err(anyhow!("Cannot render markdown to stdout: {:?}", error))
        }
    })
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
