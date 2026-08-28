use crate::client_events::Bot;
use ::serenity::all::GuildId;
use ::serenity::gateway::ActivityData;
use dotenvy::{dotenv, var};
use poise::serenity_prelude as serenity;
use sea_orm::Database;
use serenity::prelude::*;
use std::{sync::Arc, time::Duration};

pub mod client_events;
pub mod commands;
pub mod db;
pub mod interactions;

pub struct Data {}
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
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

#[poise::command(slash_command, subcommands("child1", "child2"))]
pub async fn parent(ctx: Context<'_>, arg: String) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command)]
pub async fn child1(ctx: Context<'_>, arg: String) -> Result<(), Error> {
    Ok(())
}
#[poise::command(slash_command)]
pub async fn child2(ctx: Context<'_>, arg: String) -> Result<(), Error> {
    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILD_PRESENCES;

    let database_connection = Database::connect(var("DATABASE_URL").unwrap())
        .await
        .unwrap();

    let bot = Bot {
        version: env!("CARGO_PKG_VERSION"),
        database: database_connection,
    };

    let commands = vec![parent()];

    let poise_options = poise::FrameworkOptions {
        commands,
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("~".into()),
            edit_tracker: Some(Arc::new(poise::EditTracker::for_timespan(
                Duration::from_secs(3600),
            ))),
            additional_prefixes: vec![
                poise::Prefix::Literal("hey bot,"),
                poise::Prefix::Literal("hey bot"),
            ],
            ..Default::default()
        },
        on_error: |error| Box::pin(on_error(error)),
        pre_command: |ctx| {
            Box::pin(async move {
                println!("Executing command {}...", ctx.command().qualified_name);
            })
        },
        post_command: |ctx| {
            Box::pin(async move {
                println!("Executed command {}!", ctx.command().qualified_name);
            })
        },
        command_check: Some(|ctx| {
            Box::pin(async move {
                if ctx.author().id == 123456789 {
                    return Ok(false);
                }
                Ok(true)
            })
        }),
        skip_checks_for_owners: false,
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                println!("Logged in as {}", _ready.user.name);
                poise::builtins::register_in_guild(
                    ctx,
                    &framework.options().commands,
                    // Test Server ID 1182260148501225552
                    GuildId::new(1182260148501225552),
                )
                .await?;
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        })
        .options(poise_options)
        .build();

    let mut client = Client::builder(var("TOKEN").unwrap(), intents)
        .event_handler(bot)
        .framework(framework)
        .activity(ActivityData {
            name: format!("Version {}", env!("CARGO_PKG_VERSION")),
            kind: serenity::ActivityType::Custom,
            state: Some(format!("Version Beta_{}", env!("CARGO_PKG_VERSION"))),
            url: None,
        })
        .await
        .expect("Err creating client");

    if let Err(err) = client.start().await {
        println!("Client error: {err:?}");
    }
}
