use crate::commands::stats::user_card;
use crate::{Context, Error};
use chrono::Local;
use poise::CreateReply;
use serenity::builder::CreateAttachment;

/// Show a users stat card
#[poise::command(slash_command, guild_only, category = "stats", user_cooldown = 5)]
pub async fn stats(
    ctx: Context<'_>,
    user: poise::serenity_prelude::Member,
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

    let user_card_data = user_card(ctx, &user.user.id.to_string()).await;

    println!("{:?}ms", Local::now().timestamp_millis() - &start);
    match user_card_data {
        Some(data) => {
            message
                .edit(
                    ctx,
                    CreateReply::new()
                        .attachment(CreateAttachment::bytes(data, "me.png"))
                        .content(""),
                )
                .await?
        }
        None => {
            message
                .edit(
                    ctx,
                    CreateReply::new().content(format!("{} has no data.", user.display_name())),
                )
                .await?
        }
    }
    Ok(())
}
