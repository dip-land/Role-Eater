use crate::Bot;
use serenity::model::voice::VoiceState;
use serenity::prelude::*;

pub async fn voice_state_update(
    _bot: &Bot,
    _ctx: Context,
    _old: Option<VoiceState>,
    _new: VoiceState,
) {
}
