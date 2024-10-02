use rand::prelude::*;
use std::collections::HashMap;

use serenity::all::Mentionable;

use crate::config;
use crate::config::GameState;
use crate::utils;
use crate::utils::broadcast_message;
use crate::utils::create_embed;
use crate::Context;
use crate::Error;

fn load_questions() -> Result<Vec<config::Question>, Error> {
    let mut path = std::path::PathBuf::from("./");
    path.push("blind_test");
    path.push("music.json");
    let file = std::fs::File::open(&path)?;
    let mut out: Vec<config::Question> = serde_json::from_reader(&file)?;
    out.iter_mut()
        .flat_map(|s| s.answer.iter_mut())
        .flat_map(|s| match s {
            config::Answer::MutlipleAnswer(s, len) => {
                *len = s.len();
                either::Left(s.iter_mut())
            }
            config::Answer::SingleAnswer(s) => either::Right(std::iter::once(s)),
        })
        .for_each(|s| {
            let taken = std::mem::take(s);
            *s = config::Question::normalize_string(&taken);
        });
    out.reverse();
    //out.shuffle(&mut rand::thread_rng());
    Ok(out)
}

/// Command to interact with the games
#[poise::command(
    slash_command,
    subcommands(
        "create_game",
        "end_game",
        "delete_game",
        "start_game",
        "team_cmd",
        "points_cmd"
    ),
    rename = "game",
    guild_only
)]
pub async fn game_cmd(_ctx: Context<'_>) -> Result<(), Error> {
    unreachable!()
}

/// Command to interact with the teams of a game
#[poise::command(
    slash_command,
    subcommands("add_team_game", "remove_team_game"),
    rename = "teams",
    guild_only
)]
pub async fn team_cmd(_ctx: Context<'_>) -> Result<(), Error> {
    unreachable!()
}

/// Get the leaderboard !
#[poise::command(
    slash_command,
    subcommands("points_list"),
    rename = "points",
    guild_only
)]
pub async fn points_cmd(_ctx: Context<'_>) -> Result<(), Error> {
    unreachable!()
}

/// Get the leaderboard !
#[poise::command(slash_command, rename = "list", guild_only)]
pub async fn points_list(
    ctx: Context<'_>,
    #[description = "also output a json for the leaderboard"]
    #[rename = "json"]
    print_json: bool,
) -> Result<(), Error> {
    let data = ctx.data().games.write().await;
    if !data.contains_key(&ctx.guild_id().unwrap()) {
        utils::send_error(ctx, "This guild doesn't has a g ame").await?;
        return Ok(());
    }
    for team in data
        .get(&ctx.guild_id().unwrap())
        .map(|c| c.teams.clone())
        .unwrap()
    {
        use std::fmt::Write;
        let mut msg = String::new();
        writeln!(
            &mut msg,
            "Team `{}`\ntotal {}\n",
            team.name, team.total_points
        )?;
        let mut pts: Vec<_> = team.leaderboard.iter().collect();
        pts.sort_by(|lhs, rhs| lhs.1.total_cmp(rhs.1));
        for (i, (user, p)) in pts.into_iter().take(20).enumerate() {
            writeln!(&mut msg, "{}) {} => {}pts\n", i + 1, user.mention(), p)?;
        }
        utils::send_reply(ctx, msg).await?;
        if print_json {
            let mut json = String::new();
            writeln!(&mut json, "```json")?;
            writeln!(&mut json, "{}", serde_json::to_string_pretty(&team)?)?;
            writeln!(&mut json, "```")?;
            utils::send_reply(ctx, json).await?;
        }
    }
    Ok(())
}

/// Show this help menu
#[poise::command(slash_command, rename = "new", guild_only)]
pub async fn create_game(ctx: Context<'_>) -> Result<(), Error> {
    let mut data = ctx.data().games.write().await;
    if data.contains_key(&ctx.guild_id().unwrap()) {
        utils::send_error(ctx, "This guild already has a game ongoing").await?;
        return Ok(());
    }
    let questions = load_questions()?;
    if questions.is_empty() {
        utils::send_error(ctx, "The questions list is empty").await?;
        return Ok(());
    }
    let game = config::GuildConfig {
        state: config::GameState::Configuring,
        teams: Vec::new(),
        admin_channel: ctx.channel_id(),
        questions,
    };
    data.insert(ctx.guild_id().ok_or("Not in a guild ???")?, game);
    utils::send_reply(ctx, "Created a game in the guild !").await?;

    Ok(())
}

/// End a game, even if there are remaining questions
#[poise::command(slash_command, rename = "delete", guild_only)]
pub async fn delete_game(ctx: Context<'_>) -> Result<(), Error> {
    let mut data = ctx.data().games.write().await;
    let game_state = data.get(&ctx.guild_id().unwrap()).map(|c| c.state);
    if game_state.is_none() {
        utils::send_error(ctx, "No game existed").await?;
        return Ok(());
    }
    if !matches!(game_state, Some(GameState::Ended)) {
        utils::send_error(ctx, "The game isn't finished").await?;
        return Ok(());
    }
    data.remove(&ctx.guild_id().unwrap());
    utils::send_reply(ctx, "Remove the game").await?;
    Ok(())
}

/// End a game, even if there are remaining questions
#[poise::command(slash_command, rename = "end", guild_only)]
pub async fn end_game(ctx: Context<'_>) -> Result<(), Error> {
    let mut data = ctx.data().games.write().await;
    let game_state = data.get(&ctx.guild_id().unwrap()).map(|c| c.state);
    if game_state.is_none() {
        utils::send_error(ctx, "No game exist").await?;
        return Ok(());
    }
    if matches!(game_state, Some(GameState::Ended)) {
        utils::send_error(ctx, "The game was already finished").await?;
        return Ok(());
    }
    data.get_mut(&ctx.guild_id().unwrap()).unwrap().state = GameState::Ended;
    utils::send_reply(ctx, "Ended The game").await?;
    Ok(())
}

/// Show this help menu
#[poise::command(slash_command, rename = "start", guild_only)]
pub async fn start_game(ctx: Context<'_>) -> Result<(), Error> {
    let mut data = ctx.data().games.write().await;

    match data.get(&ctx.guild_id().unwrap()).map(|c| c.state) {
        Some(GameState::Configuring) => {
            let channels = data
                .get(&ctx.guild_id().unwrap())
                .unwrap()
                .teams
                .iter()
                .map(|t| t.channel.clone())
                .chain([data.get(&ctx.guild_id().unwrap()).unwrap().admin_channel])
                .collect::<Vec<_>>();
            if channels.len() == 1 {
                utils::send_error(ctx, "No team has been added").await?;
            } else {
                {
                    data.get_mut(&ctx.guild_id().unwrap()).unwrap().state = GameState::Started;
                    broadcast_message(ctx, channels,

                         create_embed((0,0,0), "New game !",
r"We will soon start the blindtest !
If you know the name of the song, and the author/band that made it, send a message **in lowercase** here
One point will be given for the author/band name and one point for the song name
Be careful about mistakes in the response, and remember: One message for the song name, and one for the band name

ENJOY :D")).await?;
                    utils::send_reply(ctx, {
                        use std::fmt::Write;
                        let mut s = String::new();
                        writeln!(
                            &mut s,
                            "Here are the next answers for the question: `{}`\n",
                            data.get_mut(&ctx.guild_id().unwrap())
                                .unwrap()
                                .questions
                                .last()
                                .unwrap()
                                .name
                        )?;
                        for rep in data
                            .get_mut(&ctx.guild_id().unwrap())
                            .unwrap()
                            .questions
                            .last()
                            .unwrap()
                            .answer
                            .iter()
                        {
                            match rep {
                                config::Answer::SingleAnswer(srep) => {
                                    writeln!(&mut s, "-> `{srep}`")?;
                                }
                                config::Answer::MutlipleAnswer(rep_alias, _) => {
                                    if rep_alias.len() == 1 {
                                        writeln!(&mut s, "-> `{}`", rep_alias[0])?;
                                    } else {
                                        let mut iter = rep_alias.iter().peekable();
                                        if let Some(r) = iter.next() {
                                            writeln!(&mut s, "┌ -> `{r}`")?;
                                        }
                                        while let Some(r) = iter.next() {
                                            if iter.peek().is_none() {
                                                writeln!(&mut s, "└ -> `{r}`")?;
                                            } else {
                                                writeln!(&mut s, "│ -> `{r}`")?;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        writeln!(
                            &mut s,
                            "\nThere are {} remaining questions",
                            data.get_mut(&ctx.guild_id().unwrap())
                                .unwrap()
                                .questions
                                .len()
                                - 1
                        )?;
                        s
                    })
                    .await?;
                }
            }
        }
        None => {
            utils::send_error(ctx, "No Game exist in this guild").await?;
        }
        _ => {
            utils::send_error(ctx, "The game has already started, or it has ended").await?;
        }
    }
    Ok(())
}

/// Add a team to the current game
#[poise::command(slash_command, rename = "add", guild_only)]
pub async fn add_team_game(
    ctx: Context<'_>,
    #[description = "team name"] name: String,
    #[description = "team discord channel"] channel: serenity::all::ChannelId,
) -> Result<(), Error> {
    let mut data = ctx.data().games.write().await;
    if !data.contains_key(&ctx.guild_id().unwrap()) {
        utils::send_error(ctx, "No game exists").await?;
        return Ok(());
    }
    if data
        .get(&ctx.guild_id().unwrap())
        .map(|gconfig| gconfig.teams.iter().any(|e| e.channel == channel))
        .unwrap_or_default()
    {
        utils::send_error(ctx, "A team already exists with that channel !").await?;
        return Ok(());
    }
    if data
        .get(&ctx.guild_id().unwrap())
        .map(|gconfig| gconfig.teams.iter().any(|e| e.name == name))
        .unwrap_or_default()
    {
        utils::send_error(ctx, "A team already exists with that name !").await?;
        return Ok(());
    }
    let Some(channel_info) = ctx.http().get_channel(channel).await?.guild() else {
        utils::send_error(ctx, "The given channel isn't in the guild !").await?;
        return Ok(());
    };

    if !matches!(channel_info.kind, serenity::all::ChannelType::Text) {
        utils::send_error(ctx, "The channel given isn't a text channel").await?;
        return Ok(());
    }
    data.get_mut(&ctx.guild_id().unwrap())
        .unwrap()
        .teams
        .push(config::Team {
            name,
            channel,
            leaderboard: HashMap::new(),
            total_points: 0.0,
        });
    utils::send_reply(
        ctx,
        format!(
            "Created a new team that will respond in {}",
            channel.mention()
        ),
    )
    .await?;

    Ok(())
}

/// Remove a team from the current game
#[poise::command(slash_command, rename = "remove", guild_only)]
pub async fn remove_team_game(
    ctx: Context<'_>,
    #[description = "team name"] name: String,
) -> Result<(), Error> {
    let mut data = ctx.data().games.write().await;
    if !data.contains_key(&ctx.guild_id().unwrap()) {
        utils::send_error(ctx, "No game exists").await?;
        return Ok(());
    }
    if !data
        .get(&ctx.guild_id().unwrap())
        .map(|gconfig| gconfig.teams.iter().any(|e| e.name == name))
        .unwrap_or_default()
    {
        utils::send_error(ctx, "No team exists with this name !").await?;
        return Ok(());
    }
    let gconfig = data
        .get_mut(&ctx.guild_id().ok_or("Not in a guild ???")?)
        .unwrap();
    let Some(pos) = gconfig.teams.iter().position(|e| e.name == name) else {
        return Ok(());
    };
    gconfig.teams.swap_remove(pos);
    utils::send_reply(ctx, format!("Removed team named {name} !")).await?;
    Ok(())
}
