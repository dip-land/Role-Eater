use serenity::async_trait;
use serenity::model::voice::VoiceState;
use serenity::prelude::*;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn voice_state_update(&self, _ctx: Context, old: Option<VoiceState>, new: VoiceState) {

    }
}