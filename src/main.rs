#![warn(clippy::str_to_string)]

mod blindtest;
mod config;
mod utils;

use ::serenity::all::{FullEvent, GuildId, Mentionable};
use config::Question;
use poise::{serenity_prelude as serenity, BoxFuture};
use std::{collections::HashMap, env::var, sync::Arc};
use utils::{broadcast_message, create_embed, send_admin_message};

// Types used by all command functions
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

// Custom user data passed to all command functions
pub struct Data {
    games: Arc<tokio::sync::RwLock<HashMap<GuildId, config::GuildConfig>>>,
}

/// Show this help menu
#[poise::command(slash_command)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> Result<(), Error> {
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration {
            extra_text_at_bottom: "This is an example bot made to showcase features of my custom Discord bot framework",
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    // This is our custom error handler
    // They are many errors that can occur, so we only handle the ones we want to customize
    // and forward the rest to the default handler
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            println!("Error in command `{}`: {:?}", ctx.command().name, error,);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                println!("Error while handling error: {}", e)
            }
        }
    }
}
fn handle_message<'a>(
    ctx: &'a serenity::all::Context,
    event: &'a FullEvent,
    _framework: poise::FrameworkContext<'a, Data, Error>,
    _data: &'a Data,
) -> BoxFuture<'a, Result<(), Error>> {
    Box::pin(async move { _handle_message(ctx, event, _framework, _data).await })
}

async fn _handle_message(
    ctx: &serenity::all::Context,
    event: &FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    _data: &Data,
) -> Result<(), Error> {
    match event {
        FullEvent::Message { new_message } => {
            if new_message.author.bot {
                return Ok(());
            }
            let Some(guild_id) = new_message.guild_id else {
                return Ok(());
            };
            let channel_id = new_message.channel_id;
            let author = &new_message.author;
            let data = _data.games.write().await;
            if !data
                .get(&guild_id)
                .map(|c| matches!(c.state, config::GameState::Started))
                .unwrap_or_default()
            {
                return Ok(());
            }

            let mut game =
                tokio::sync::RwLockWriteGuard::map(data, |s| s.get_mut(&guild_id).unwrap());
            let mut channels = game.teams.iter().map(|t| t.channel).collect::<Vec<_>>();
            if !channels.contains(&channel_id) {
                return Ok(());
            }
            channels.push(game.admin_channel);

            let Some(_) = game.questions.last_mut() else {
                game.state = config::GameState::Ended;
                return Ok(());
            };
            if let Some((pos, pts)) = game
                .questions
                .last_mut()
                .unwrap()
                .get_answer_pos(&new_message.content)
            {
                let normalized = config::Question::normalize_string(&new_message.content);
                let remove = match game.questions.last_mut().unwrap().answer.get_mut(pos) {
                    Some(config::Answer::MutlipleAnswer(i, _p)) => {
                        let p = i.iter().position(|s| *s == normalized);
                        p.and_then(|p| -> Option<()> {
                            i.swap_remove(p);
                            None
                        });
                        i.is_empty()
                    }
                    _ => true,
                };
                if remove {
                    game.questions.last_mut().unwrap().answer.swap_remove(pos);
                };
                let team = game
                    .teams
                    .iter_mut()
                    .find(|t| t.channel == channel_id)
                    .unwrap();
                *team.leaderboard.entry(author.id).or_default() += pts;
                team.total_points += pts;
                broadcast_message(
                    ctx,
                    channels.clone(),
                    create_embed(
                        (0, 255, 0),
                        "Answer found !",
                        format!(
                            "{} found an answer !\nIt was: `{}`\nThey now have {} points !",
                            new_message.author.id.mention(),
                            Question::normalize_string(&new_message.content_safe(ctx)),
                            team.total_points
                        ),
                    ),
                )
                .await?;
            }

            if game.questions.last_mut().unwrap().answer.is_empty() {
                game.questions.pop();
                let is_finished = game.questions.is_empty();

                broadcast_message(
                    ctx,
                    channels.clone(),
                    create_embed(
                        (0, 255, 0),
                        "All answer found",
                        if is_finished {
                            "The game is finished\n Hope you had fun !"
                        } else {
                            "All anser were found for the current questions !"
                        },
                    ),
                )
                .await?;
                if !is_finished {
                    send_admin_message(
                        &ctx,
                        game.admin_channel,
                        create_embed((0, 0, 255), "Next question !", {
                            use std::fmt::Write;
                            let mut s = String::new();
                            writeln!(
                                &mut s,
                                "Here are the next answers for the question: `{}`\n",
                                game.questions.last().unwrap().name
                            )?;
                            for rep in &game.questions.last().unwrap().answer {
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
                                game.questions.len() - 1
                            )?;
                            s
                        }),
                    )
                    .await?;
                } else {
                    send_admin_message(
                        &ctx,
                        game.admin_channel,
                        create_embed((0, 0, 255), "Game is finished !", "Hope it was fun!"),
                    )
                    .await?;
                }
            }

            Ok(())
        }
        _ => Ok(()),
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    // Framework kOptions contains allof poise's configuration option in one struct
    // Every option can be omitted to use its default value
    let options = poise::FrameworkOptions {
        commands: vec![help(), blindtest::game_cmd()],
        prefix_options: Default::default(),
        on_error: |error| Box::pin(on_error(error)),
        pre_command: |_ctx| Box::pin(async {}),
        post_command: |_ctx| Box::pin(async {}),
        command_check: Some(|ctx| {
            Box::pin(async move {
                let (guild, role) = {
                    let Some(guild) = ctx.guild() else {
                        return Ok(false);
                    };
                    let Some(role) = guild.role_by_name("Tutors") else {
                        eprintln!("There is no role named tutors !");
                        return Ok(false);
                    };
                    (guild.id, role.id)
                };
                Ok(ctx.author().has_role(ctx, guild, role).await?)
            })
        }),
        // Enforce command checks even for owners (enforced by default)
        // Set to true to bypass checks, which is useful for testing
        skip_checks_for_owners: false,
        event_handler: handle_message,
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                println!("Logged in as {}", _ready.user.name);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                println!("Registered commands");
                Ok(Data {
                    games: Default::default(),
                })
            })
        })
        .options(options)
        .build();

    let token = var("DISCORD_TOKEN")
        .expect("Missing `DISCORD_TOKEN` env var, see README for more information.");
    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap()
}
