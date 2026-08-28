use ::serenity::all::GuildId;
use ::serenity::model::user::User;
use poise::serenity_prelude as serenity;
use sea_orm::DatabaseConnection;
use serenity::all::GuildMemberUpdateEvent;
use serenity::async_trait;
use serenity::model::{
    application::Interaction,
    channel::Message,
    gateway::{Presence, Ready},
    guild::{Guild, Member, PartialGuild},
    voice::VoiceState,
};
use serenity::prelude::*;

pub mod guild_create;
pub mod guild_member_removal;
pub mod guild_member_update;
pub mod guild_update;
pub mod interaction_create;
pub mod message;
pub mod presence_update;
pub mod ready;
pub mod voice_state_update;

pub struct Bot {
    pub version: &'static str,
    pub database: DatabaseConnection,
}

#[async_trait]
impl EventHandler for Bot {
    async fn guild_create(&self, ctx: serenity::Context, guild: Guild, is_new: Option<bool>) {
        guild_create::guild_create(self, ctx, guild, is_new).await
    }
    async fn guild_member_removal(
        &self,
        ctx: serenity::Context,
        guild_id: GuildId,
        user: User,
        member_data_if_available: Option<Member>,
    ) {
        guild_member_removal::guild_member_removal(
            self,
            ctx,
            guild_id,
            user,
            member_data_if_available,
        )
        .await
    }
    async fn guild_update(
        &self,
        ctx: serenity::Context,
        old_data: Option<Guild>,
        new_data: PartialGuild,
    ) {
        guild_update::guild_update(self, ctx, old_data, new_data).await
    }
    async fn message(&self, ctx: serenity::Context, msg: Message) {
        message::message(self, ctx, msg).await
    }
    async fn presence_update(&self, ctx: serenity::Context, new_data: Presence) {
        presence_update::presence_update(self, ctx, new_data).await
    }
    async fn ready(&self, ctx: serenity::Context, data_about_bot: Ready) {
        ready::ready(self, ctx, data_about_bot).await
    }
    async fn guild_member_update(
        &self,
        ctx: serenity::Context,
        old_if_available: Option<Member>,
        new: Option<Member>,
        event: GuildMemberUpdateEvent,
    ) {
        guild_member_update::guild_member_update(self, ctx, old_if_available, new, event).await
    }
    async fn voice_state_update(
        &self,
        ctx: serenity::Context,
        old: Option<VoiceState>,
        new: VoiceState,
    ) {
        voice_state_update::voice_state_update(self, ctx, old, new).await
    }
    async fn interaction_create(&self, ctx: serenity::Context, interaction: Interaction) {
        interaction_create::interaction_create(self, ctx, interaction).await
    }
}
