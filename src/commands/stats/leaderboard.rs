use crate::commands::stats::guild_leaderboard_card;
use crate::{Context, Error};
use chrono::Local;
use poise::CreateReply;
use serenity::builder::CreateAttachment;

/// Show the servers leaderboard
#[poise::command(slash_command, guild_only, category = "stats", user_cooldown = 5)]
pub async fn leaderboard(
    ctx: Context<'_>,
    #[description = "Hide command output"]
    #[flag]
    hide: bool,
) -> Result<(), Error> {
    let message = ctx
        .send(
            CreateReply::new()
                .content("Loading <a:loading:1547289616208367726>")
                .ephemeral(hide),
        )
        .await?;
    let start = Local::now().timestamp_millis();

    let guild_leaderboard_card_data = guild_leaderboard_card(ctx).await;

    println!("{:?}ms", Local::now().timestamp_millis() - &start);
    match guild_leaderboard_card_data {
        Some(data) => {
            message
                .edit(
                    ctx,
                    CreateReply::new()
                        .attachment(CreateAttachment::bytes(data, "leaderboard.png"))
                        .content(""),
                )
                .await?
        }
        None => {
            message
                .edit(ctx, CreateReply::new().content("User has no data."))
                .await?
        }
    }
    Ok(())
}
