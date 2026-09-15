pub mod leaderboard;
pub mod me;
pub mod stat_image;
pub mod stats_command;

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use numfmt::{Formatter, Scales};
use sea_orm::{ColumnTrait, DbErr, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use skia_rs::canvas::{Canvas, ClipOp, Surface};
use skia_rs::codec::{Image, ImageEncoder, PngEncoder};
use skia_rs::core::{Color, Point, Rect};
use skia_rs::paint::{Paint, Style};
use skia_rs::path::PathBuilder;
use skia_rs::text::{Font, Typeface};
use std::cmp::Ordering;
use std::sync::Arc;

use crate::Context;
use crate::db::{global::*, users::*};

const DEFAULT_TTF: &[u8] = include_bytes!("../../../fonts/segoeui.ttf");
const BOLD_TTF: &[u8] = include_bytes!("../../../fonts/segoeuib.ttf");
const ITALIC_TTF: &[u8] = include_bytes!("../../../fonts/segoeuii.ttf");

const DATE_FORMAT: &str = "%b %d, %Y";

fn default_font(size: f32) -> Font {
    let tf = Typeface::from_data(DEFAULT_TTF.to_vec()).expect("Failed to load default typeface");
    Font::new(Arc::new(tf), size)
}

fn bold_font(size: f32) -> Font {
    let tf = Typeface::from_data(BOLD_TTF.to_vec()).expect("Failed to load bold typeface");
    Font::new(Arc::new(tf), size)
}

fn italic_font(size: f32) -> Font {
    let tf = Typeface::from_data(ITALIC_TTF.to_vec()).expect("Failed to load italic typeface");
    Font::new(Arc::new(tf), size)
}

pub struct UserPositions {
    pub total: Vec<String>,
    pub voice: Vec<String>,
    pub message: Vec<String>,
}
async fn get_user_positions(ctx: Context<'_>) -> UserPositions {
    let mut users: Vec<user_data::Model> = user_data::Entity::find()
        .filter(user_data::Column::GuildId.eq(&ctx.guild_id().unwrap().to_string()))
        .filter(user_data::Column::UserLeft.eq(false))
        .order_by_desc(user_data::Column::Total)
        .all(&ctx.data().database)
        .await
        .unwrap();

    let total: Vec<String> = users.clone().into_iter().map(|user| user.user_id).collect();

    users.sort_by(|a, b| cmp_f64(&b.voice_time, &a.voice_time));
    let voice: Vec<String> = users.clone().into_iter().map(|user| user.user_id).collect();

    users.sort_by(|a, b| cmp_i64(&b.message_count, &a.message_count));
    let message: Vec<String> = users.clone().into_iter().map(|user| user.user_id).collect();

    UserPositions {
        total,
        voice,
        message,
    }
}

#[derive(Debug, Clone)]
pub struct UserActivityHistory {
    pub day: UserActivityHistoryData,
    pub week: Vec<UserActivityHistoryData>,
    pub total: Vec<(NaiveDate, UserActivityHistoryData)>,
}
#[derive(Debug, Clone)]
pub struct UserActivityHistoryData {
    pub message_count: i64,
    pub voice_time: f64,
    pub game_time: f64,
}
fn get_or_insert_day(
    all_data: &mut Vec<(NaiveDate, UserActivityHistoryData)>,
    date: NaiveDate,
) -> &mut UserActivityHistoryData {
    if let Some(index) = all_data.iter().position(|(day, _)| *day == date) {
        return &mut all_data[index].1;
    }

    all_data.push((
        date,
        UserActivityHistoryData {
            message_count: 0,
            voice_time: 0.0,
            game_time: 0.0,
        },
    ));

    &mut all_data.last_mut().unwrap().1
}
async fn get_user_activity_history(ctx: Context<'_>, user_id: &String) -> UserActivityHistory {
    let forty_five_days_ago = (Local::now() - Duration::days(45))
        .with_timezone(&Local)
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap();

    let mut all_data: Vec<(NaiveDate, UserActivityHistoryData)> = Vec::new();

    for history in voice_message_history::Entity::find()
        .filter(voice_message_history::Column::GuildId.eq(&ctx.guild_id().unwrap().to_string()))
        .filter(voice_message_history::Column::UserId.eq(user_id))
        .filter(voice_message_history::Column::Date.gte(forty_five_days_ago))
        .order_by_asc(voice_message_history::Column::Date)
        .all(&ctx.data().database)
        .await
        .unwrap()
    {
        let value = get_or_insert_day(&mut all_data, history.date);

        value.message_count += history.message_count;
        value.voice_time += history.voice_time;
    }

    for history in activity_time_history::Entity::find()
        .filter(activity_time_history::Column::UserId.eq(user_id))
        .filter(activity_time_history::Column::Date.gte(forty_five_days_ago))
        .order_by_asc(activity_time_history::Column::Date)
        .all(&ctx.data().database)
        .await
        .unwrap()
    {
        let value = get_or_insert_day(&mut all_data, history.date);

        value.game_time += history.time_played;
    }

    let now: DateTime<Local> = Local::now();
    let past = now - Duration::days(45);
    let mut date_step = past;

    while date_step <= now {
        get_or_insert_day(&mut all_data, date_step.date_naive());
        date_step += Duration::days(1);
    }

    all_data.sort_by_key(|(date, _)| *date);

    let now: DateTime<Local> = Local::now();
    let past = now - Duration::days(7);
    let mut date_step = past;
    let mut week: Vec<UserActivityHistoryData> = vec![];

    while date_step <= now {
        let value = all_data
            .iter()
            .find(|(date, _)| *date == date_step.date_naive())
            .map(|(_, value)| value);

        week.push(value.cloned().unwrap_or(UserActivityHistoryData {
            message_count: 0,
            voice_time: 0.0,
            game_time: 0.0,
        }));

        date_step += Duration::days(1);
    }

    let day = all_data
        .iter()
        .find(|(date, _)| *date == Local::now().date_naive())
        .map(|(_, value)| value.clone())
        .unwrap_or(UserActivityHistoryData {
            message_count: 0,
            voice_time: 0.0,
            game_time: 0.0,
        });

    UserActivityHistory {
        day,
        week,
        total: all_data,
    }
}

pub struct UserCurrentActivities {
    pub last_played_activity: Option<String>,
    pub last_played_activity_time: Option<f64>,
    pub current_activity: Option<String>,
    pub current_activity_start_time: Option<NaiveDateTime>,
}
async fn get_user_current_activity(
    ctx: Context<'_>,
    user_id: &String,
) -> Result<UserCurrentActivities, DbErr> {
    let user = activity_user_data::Entity::find_by_id(user_id)
        .one(&ctx.data().database)
        .await?
        .unwrap();

    let last_played_activity = match activity_data::Entity::find()
        .filter(activity_data::Column::Id.eq(&user.last_played_activity.unwrap_or(0)))
        .one(&ctx.data().database)
        .await?
    {
        Some(model) => Some(model.name),
        None => None,
    };

    let current_activity = match activity_data::Entity::find()
        .filter(activity_data::Column::Id.eq(&user.current_activity.unwrap_or(0)))
        .one(&ctx.data().database)
        .await?
    {
        Some(model) => Some(model.name),
        None => None,
    };

    Ok(UserCurrentActivities {
        last_played_activity: last_played_activity,
        last_played_activity_time: user.last_played_time_played,
        current_activity: current_activity,
        current_activity_start_time: user.current_activity_start_time,
    })
}

async fn user_card(ctx: Context<'_>, user_id: &String) -> Option<Vec<u8>> {
    let guild_data = guild_data::Entity::find()
        .filter(guild_data::Column::GuildId.eq(&ctx.guild_id().unwrap().to_string()))
        .one(&ctx.data().database)
        .await
        .unwrap();
    let user_data = user_data::Entity::find()
        .filter(user_data::Column::GuildId.eq(&ctx.guild_id().unwrap().to_string()))
        .filter(user_data::Column::UserId.eq(user_id))
        .one(&ctx.data().database)
        .await
        .unwrap();

    if user_data.is_none() {
        return None;
    }

    let mut num_format = Formatter::new()
        .precision(numfmt::Precision::Decimals(2))
        .separator(',')
        .unwrap();

    let positions = get_user_positions(ctx.clone()).await;

    let activity_data = get_user_activity_history(ctx.clone(), user_id).await;
    let activity_current_data = match get_user_current_activity(ctx.clone(), user_id).await {
        Ok(data) => data,
        Err(err) => {
            println!("{:?}", err);
            return None;
        }
    };
    let activity_history = activity_history::Entity::find()
        .filter(activity_history::Column::UserId.eq(user_id))
        .order_by_desc(activity_history::Column::TimePlayed)
        .all(&ctx.data().database)
        .await
        .unwrap();

    let most_played_activity_name = match activity_history.len() > 0 {
        true => {
            &activity_data::Entity::find()
                .filter(activity_data::Column::Id.eq(activity_history[0].activity))
                .one(&ctx.data().database)
                .await
                .unwrap()
                .unwrap()
                .name
        }
        false => "No Data",
    };
    let most_played_activity_time = match activity_history.len() > 0 {
        true => {
            let mut value = num_format
                .fmt2((activity_history[0].time_played / 60.0) / 60.0)
                .to_owned()
                + " hrs";
            if (activity_history[0].time_played / 60.0) / 60.0 < 1.0 {
                value = num_format
                    .fmt2(activity_history[0].time_played / 60.0)
                    .to_owned()
                    + " mins";
            }
            value
        }
        false => "".to_string(),
    };
    let last_played_activity_name = match activity_current_data.last_played_activity {
        Some(name) => name,
        None => "No Data".to_string(),
    };
    let last_played_activity_time = match activity_current_data.last_played_activity_time {
        Some(time) => {
            let mut value = num_format.fmt2(time / 60.0 / 60.0).to_owned() + " hrs";
            if time / 60.0 / 60.0 < 1.0 {
                value = num_format.fmt2(time / 60.0).to_owned() + " mins";
            }
            value
        }
        None => "".to_string(),
    };

    let number_scale = Scales::new(1000, vec!["", "K", "M", "B", "T", "q", "Q", "s", "S"]).unwrap();
    let mut num_format = Formatter::new()
        .scales(number_scale)
        .precision(numfmt::Precision::Decimals(2));

    let mut surface = Surface::new_raster_n32_premul(1280, 706).expect("Failed to create surface");
    let mut canvas = surface.canvas();

    let primary_text_color = Color::from_rgb(217, 219, 227);
    let secondary_text_color = Color::from_rgb(137, 144, 164);
    let primary_background_color = Color::from_rgb(15, 18, 26);
    let secondary_background_color = Color::from_rgb(25, 28, 36);
    let tertiary_background_color = Color::from_rgb(33, 36, 45);
    let quaternary_background_color = Color::from_rgb(45, 49, 60);
    let graph_message_color = Color::from_rgb(54, 217, 235);
    let graph_voice_color = Color::from_rgb(255, 99, 107);
    let graph_game_color = Color::from_rgb(99, 255, 125);

    // theme black
    // let primary_text_color = Color::from_rgb(217, 219, 227);
    // let secondary_text_color = Color::from_rgb(137, 144, 164);
    // let primary_background_color = Color::from_rgb(0, 0, 0);
    // let secondary_background_color = Color::from_rgb(10, 10, 10);
    // let tertiary_background_color = Color::from_rgb(20, 20, 20);
    // let quaternary_background_color = Color::from_rgb(30, 30, 30);
    // let graph_message_color = Color::from_rgb(54, 217, 235);
    // let graph_voice_color = Color::from_rgb(255, 99, 107);
    // let graph_game_color = Color::from_rgb(99, 255, 125);

    let width = 1280.0;
    let height = 706.0;
    let padding = 16.0;
    let outer_border_radius = 20.0;
    let inner_border_radius = outer_border_radius - (padding / 2.0);
    let full_inner_border_radius = inner_border_radius / 2.0;
    let header_height = 90.0;
    let header_font_size = (header_height - padding) / 2.0;
    let sub_font_size = 26.0;
    let header_font = bold_font(header_font_size);
    let sub_font = default_font(sub_font_size);

    let default_avatar_font_size = header_height;
    let default_avatar_font = bold_font(default_avatar_font_size);

    let date_primary_font_size = 28.0;
    let date_secondary_font_size = 24.0;
    let date_primary_font = default_font(date_primary_font_size);
    let date_secondary_font = bold_font(date_secondary_font_size);

    let section_header_font_size = 30.0;
    let sub_section_header_font_size = 28.0;
    let sub_section_font_size = 26.0;
    let sub_section_small_font_size = 22.0;
    let section_header_font = bold_font(section_header_font_size);
    let sub_section_header_font = bold_font(sub_section_header_font_size);
    let sub_section_font = default_font(sub_section_font_size);
    let sub_section_small_font = italic_font(sub_section_small_font_size);

    let middle_area_y = header_height + (padding * 2.0);
    let middle_area_height = 250.0;
    let bottom_area_y = middle_area_y + middle_area_height + padding;
    let bottom_area_height = height - (padding * 4.0) - header_height - middle_area_height;

    let middle_sections = 3.0;
    let middle_section_width = (width - (padding * (middle_sections + 1.0))) / middle_sections;
    let middle_section_a_x = padding;
    let middle_section_b_x = (padding * 2.0) + middle_section_width;
    let middle_section_c_x = middle_section_b_x + padding + middle_section_width;
    let middle_section_header_height = 60.0;

    let middle_sub_sections = 3.0;
    let middle_sub_section_width = middle_section_width - (padding * 2.0);
    let middle_sub_section_height =
        ((middle_area_height - middle_section_header_height) / middle_sub_sections) - padding;
    let middle_sub_section_a_y = middle_area_y + middle_section_header_height;
    let middle_sub_section_b_y = middle_sub_section_a_y + middle_sub_section_height + padding;
    let middle_sub_section_c_y = middle_sub_section_b_y + middle_sub_section_height + padding;

    let bottom_sections = 2.0;
    let game_section_width = (middle_section_width * 2.0) + padding;
    let chart_section_width = (width - (padding * (bottom_sections + 1.0))) - game_section_width;
    let game_section_x = padding;
    let chart_section_x = (padding * 2.0) + chart_section_width;
    let bottom_section_header_height = 60.0;

    let game_sub_sections = 2.0;
    let game_sub_section_width = chart_section_width - (padding * 2.0);
    let game_sub_section_height =
        ((bottom_area_height - bottom_section_header_height) / game_sub_sections) - padding;
    let game_sub_section_a_y = bottom_area_y + bottom_section_header_height;
    let game_sub_section_b_y = game_sub_section_a_y + game_sub_section_height + padding;
    let game_sub_section_header_height = sub_section_header_font_size + padding;

    let chart_width = game_section_width - (padding * 2.0);
    let chart_height = (bottom_area_height - bottom_section_header_height) - padding;
    let chart_x = chart_section_x + padding;
    let chart_y = bottom_area_y + bottom_section_header_height;

    let mut primary_text_paint = Paint::new();
    primary_text_paint.set_anti_alias(true);
    primary_text_paint.set_color(primary_text_color.to_color4f());
    primary_text_paint.set_style(Style::Fill);

    let mut secondary_text_paint = Paint::new();
    secondary_text_paint.set_anti_alias(true);
    secondary_text_paint.set_color(secondary_text_color.to_color4f());
    secondary_text_paint.set_style(Style::Fill);

    let mut primary_background_paint = Paint::new();
    primary_background_paint.set_anti_alias(true);
    primary_background_paint.set_color(primary_background_color.to_color4f());
    primary_background_paint.set_style(Style::Fill);

    let mut secondary_background_paint = Paint::new();
    secondary_background_paint.set_anti_alias(true);
    secondary_background_paint.set_color(secondary_background_color.to_color4f());
    secondary_background_paint.set_style(Style::Fill);

    let mut tertiary_background_paint = Paint::new();
    tertiary_background_paint.set_anti_alias(true);
    tertiary_background_paint.set_color(tertiary_background_color.to_color4f());
    tertiary_background_paint.set_style(Style::Fill);

    let mut quaternary_background_paint = Paint::new();
    quaternary_background_paint.set_anti_alias(true);
    quaternary_background_paint.set_color(quaternary_background_color.to_color4f());
    quaternary_background_paint.set_style(Style::Fill);

    let rect = Rect::from_xywh(0.0, 0.0, width, height);
    canvas.draw_round_rect(
        &rect,
        outer_border_radius,
        outer_border_radius,
        &primary_background_paint,
    );

    let rect = Rect::from_xywh(padding, padding, header_height, header_height);
    canvas.draw_round_rect(
        &rect,
        inner_border_radius,
        inner_border_radius,
        &secondary_background_paint,
    );
    if let Some(avatar_url) = user_data.clone().unwrap().avatar {
        if let Some(avatar) = load_image_from_url(&avatar_url).await {
            canvas.save();
            let mut builder = PathBuilder::new();
            builder.add_round_rect(&rect, inner_border_radius, inner_border_radius);
            canvas.clip_path(&builder.build(), ClipOp::Intersect, true);
            canvas.draw_image_rect(&avatar, None, &rect, Some(&secondary_background_paint));
            canvas.restore();
        } else {
            draw_username_letter(
                &mut canvas,
                &user_data.clone().unwrap().username,
                padding,
                padding,
                header_height,
                header_height,
                &default_avatar_font,
                &primary_text_paint,
            );
        }
    } else {
        draw_username_letter(
            &mut canvas,
            &user_data.clone().unwrap().username,
            padding,
            padding,
            header_height,
            header_height,
            &default_avatar_font,
            &primary_text_paint,
        );
    }

    fn draw_username_letter(
        canvas: &mut Canvas<'_>,
        username: &str,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        font: &Font,
        color: &Paint,
    ) {
        let char = username
            .chars()
            .next()
            .unwrap()
            .to_ascii_uppercase()
            .to_string();
        let char_width = font.measure_text(&char);
        let metrics = font.metrics();
        let baseline_y = (y / 1.5) + (height / 2.0) - ((metrics.ascent + metrics.descent) / 2.0);

        canvas.draw_string(
            &char,
            x + (width / 2.0) - (char_width / 2.0),
            baseline_y,
            font,
            color,
        );
    }

    //middle area sections
    let rect = Rect::from_xywh(
        middle_section_a_x,
        middle_area_y,
        middle_section_width,
        middle_area_height,
    );
    canvas.draw_round_rect(
        &rect,
        inner_border_radius,
        inner_border_radius,
        &tertiary_background_paint,
    );
    let rect = Rect::from_xywh(
        middle_section_b_x,
        middle_area_y,
        middle_section_width,
        middle_area_height,
    );
    canvas.draw_round_rect(
        &rect,
        inner_border_radius,
        inner_border_radius,
        &tertiary_background_paint,
    );
    let rect = Rect::from_xywh(
        middle_section_c_x,
        middle_area_y,
        middle_section_width,
        middle_area_height,
    );
    canvas.draw_round_rect(
        &rect,
        inner_border_radius,
        inner_border_radius,
        &tertiary_background_paint,
    );

    //bottom area sections
    let rect = Rect::from_xywh(
        game_section_x,
        bottom_area_y,
        chart_section_width,
        bottom_area_height,
    );
    canvas.draw_round_rect(
        &rect,
        inner_border_radius,
        inner_border_radius,
        &tertiary_background_paint,
    );
    let rect = Rect::from_xywh(
        chart_section_x,
        bottom_area_y,
        game_section_width,
        bottom_area_height,
    );
    canvas.draw_round_rect(
        &rect,
        inner_border_radius,
        inner_border_radius,
        &tertiary_background_paint,
    );

    //middle sub sections
    let rect = Rect::from_xywh(
        middle_section_a_x + padding,
        middle_sub_section_a_y,
        middle_sub_section_width,
        middle_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        full_inner_border_radius,
        full_inner_border_radius,
        &secondary_background_paint,
    );
    let rect = Rect::from_xywh(
        middle_section_a_x + padding,
        middle_sub_section_b_y,
        middle_sub_section_width,
        middle_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        full_inner_border_radius,
        full_inner_border_radius,
        &secondary_background_paint,
    );
    let rect = Rect::from_xywh(
        middle_section_a_x + padding,
        middle_sub_section_c_y,
        middle_sub_section_width,
        middle_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        full_inner_border_radius,
        full_inner_border_radius,
        &secondary_background_paint,
    );

    let rect = Rect::from_xywh(
        middle_section_b_x + padding,
        middle_sub_section_a_y,
        middle_sub_section_width,
        middle_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        full_inner_border_radius,
        full_inner_border_radius,
        &secondary_background_paint,
    );
    let rect = Rect::from_xywh(
        middle_section_b_x + padding,
        middle_sub_section_b_y,
        middle_sub_section_width,
        middle_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        full_inner_border_radius,
        full_inner_border_radius,
        &secondary_background_paint,
    );
    let rect = Rect::from_xywh(
        middle_section_b_x + padding,
        middle_sub_section_c_y,
        middle_sub_section_width,
        middle_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        full_inner_border_radius,
        full_inner_border_radius,
        &secondary_background_paint,
    );

    let rect = Rect::from_xywh(
        middle_section_c_x + padding,
        middle_sub_section_a_y,
        middle_sub_section_width,
        middle_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        full_inner_border_radius,
        full_inner_border_radius,
        &secondary_background_paint,
    );
    let rect = Rect::from_xywh(
        middle_section_c_x + padding,
        middle_sub_section_b_y,
        middle_sub_section_width,
        middle_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        full_inner_border_radius,
        full_inner_border_radius,
        &secondary_background_paint,
    );
    let rect = Rect::from_xywh(
        middle_section_c_x + padding,
        middle_sub_section_c_y,
        middle_sub_section_width,
        middle_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        full_inner_border_radius,
        full_inner_border_radius,
        &secondary_background_paint,
    );

    //bottom sub sections
    let rect = Rect::from_xywh(
        game_section_x + padding,
        game_sub_section_a_y,
        game_sub_section_width,
        game_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        full_inner_border_radius,
        full_inner_border_radius,
        &secondary_background_paint,
    );
    let rect = Rect::from_xywh(
        game_section_x + padding,
        game_sub_section_b_y,
        game_sub_section_width,
        game_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        full_inner_border_radius,
        full_inner_border_radius,
        &secondary_background_paint,
    );

    let rect = Rect::from_xywh(
        game_section_x + padding,
        game_sub_section_a_y,
        game_sub_section_width,
        game_sub_section_header_height,
    );
    canvas.draw_round_rect(
        &rect,
        full_inner_border_radius,
        full_inner_border_radius,
        &primary_background_paint,
    );
    let rect = Rect::from_xywh(
        game_section_x + padding,
        game_sub_section_b_y,
        game_sub_section_width,
        game_sub_section_header_height,
    );
    canvas.draw_round_rect(
        &rect,
        full_inner_border_radius,
        full_inner_border_radius,
        &primary_background_paint,
    );

    let rect = Rect::from_xywh(chart_x, chart_y, chart_width, chart_height);
    canvas.draw_round_rect(
        &rect,
        full_inner_border_radius,
        full_inner_border_radius,
        &primary_background_paint,
    );

    // card header
    let mut user_display_name = user_data.clone().unwrap().nickname.unwrap_or(
        user_data.clone().unwrap().display_name.unwrap_or(
            user_data
                .clone()
                .unwrap()
                .global_name
                .unwrap_or(user_data.clone().unwrap().username),
        ),
    );
    let mut user_username = user_data.clone().unwrap().username;
    let server_name = &guild_data.clone().unwrap().name;

    let mut display_name_max_length = 32;
    if user_username.chars().count() > 4 {
        display_name_max_length = 31 - (user_username.chars().count() / 2);
    }

    if user_display_name.chars().count() > display_name_max_length {
        let mut string = user_display_name.clone();
        string.truncate(display_name_max_length - 3);
        user_display_name = string.clone() + &"...";
    }
    if user_username.chars().count() > 24 {
        let mut string = user_username.clone();
        string.truncate(21);
        user_username = string.clone() + &"...";
    }

    let display_name_width = header_font.measure_text(user_display_name.as_str());
    canvas.draw_string(
        user_display_name.as_str(),
        header_height + (padding * 2.0),
        padding + header_font_size,
        &header_font,
        &primary_text_paint,
    );
    canvas.draw_string(
        user_username.as_str(),
        header_height + (padding * 3.0) + display_name_width,
        padding + header_font_size,
        &sub_font,
        &secondary_text_paint,
    );
    canvas.draw_string(
        server_name,
        header_height + (padding * 2.0),
        (padding * 2.0) + header_font_size + sub_font_size,
        &sub_font,
        &secondary_text_paint,
    );

    //card dates
    let start_epoch: DateTime<Utc> = Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).unwrap();
    let user_creation_date = match user_data.clone().unwrap().creation_date {
        Some(date) => date.with_timezone(&Utc),
        None => start_epoch,
    };
    let user_creation_date = match user_creation_date.year() == 1970 {
        true => "No Data",
        false => &user_creation_date.format(DATE_FORMAT).to_string(),
    };

    let user_join_date = match user_data.clone().unwrap().join_date {
        Some(date) => date.with_timezone(&Utc),
        None => start_epoch,
    };
    let user_join_date = match user_join_date.year() == 1970 {
        true => "No Data",
        false => &user_join_date.format(DATE_FORMAT).to_string(),
    };

    let creation_date_fixed_width = date_secondary_font.measure_text("Created On");
    let creation_date_width = date_primary_font.measure_text(user_creation_date);
    let join_date_fixed_width = date_secondary_font.measure_text("Joined On");
    let join_date_width = date_primary_font.measure_text(user_join_date);

    let rect = Rect::from_xywh(
        width - (padding * 3.0) - join_date_width,
        header_height - padding - date_primary_font_size,
        join_date_width + (padding * 2.0),
        date_primary_font_size + (padding * 2.0),
    );
    canvas.draw_round_rect(
        &rect,
        inner_border_radius,
        inner_border_radius,
        &tertiary_background_paint,
    );
    let rect = Rect::from_xywh(
        width - (padding * 6.0) - join_date_width - creation_date_width,
        header_height - padding - date_primary_font_size,
        creation_date_width + (padding * 2.0),
        date_primary_font_size + (padding * 2.0),
    );
    canvas.draw_round_rect(
        &rect,
        inner_border_radius,
        inner_border_radius,
        &tertiary_background_paint,
    );

    let rect = Rect::from_xywh(
        width - (padding * 3.0) - join_date_width + (padding / 2.0),
        padding,
        join_date_fixed_width + padding,
        date_primary_font_size + (padding / 2.0),
    );
    canvas.draw_round_rect(
        &rect,
        inner_border_radius,
        inner_border_radius,
        &quaternary_background_paint,
    );
    let rect = Rect::from_xywh(
        width - (padding * 6.0) - join_date_width - creation_date_width + (padding / 2.0),
        padding,
        creation_date_fixed_width + padding,
        date_primary_font_size + (padding / 2.0),
    );
    canvas.draw_round_rect(
        &rect,
        inner_border_radius,
        inner_border_radius,
        &quaternary_background_paint,
    );

    canvas.draw_string(
        "Created On",
        width - (padding * 6.0) - join_date_width - creation_date_width + padding,
        (padding * 1.2) + date_secondary_font_size,
        &date_secondary_font,
        &primary_text_paint,
    );
    canvas.draw_string(
        "Joined On",
        width - (padding * 3.0) - join_date_width + padding,
        (padding * 1.2) + date_secondary_font_size,
        &date_secondary_font,
        &primary_text_paint,
    );

    canvas.draw_string(
        user_creation_date,
        width - (padding * 6.0) - join_date_width - creation_date_width + padding,
        header_height - (padding * 2.0) + date_primary_font_size,
        &date_primary_font,
        &primary_text_paint,
    );
    canvas.draw_string(
        user_join_date,
        width - (padding * 3.0) - join_date_width + padding,
        header_height - (padding * 2.0) + date_primary_font_size,
        &date_primary_font,
        &primary_text_paint,
    );

    // middle section
    let rank_total = positions.total.iter().position(|x| x == user_id).unwrap() + 1;
    let rank_message = positions.message.iter().position(|x| x == user_id).unwrap() + 1;
    let rank_voice = positions.voice.iter().position(|x| x == user_id).unwrap() + 1;

    let message_today = activity_data.day.message_count;
    let message_week: i64 = activity_data.week.iter().map(|v| v.message_count).sum();
    let message_total = user_data.clone().unwrap().message_count;

    let voice_today = activity_data.day.voice_time;
    let voice_week: f64 = activity_data.week.iter().map(|v| v.voice_time).sum();
    let voice_total = user_data.clone().unwrap().voice_time;

    let rank_total_formatted = ("#".to_string() + num_format.fmt2(rank_total)).replace(".0", "");
    let rank_message_formatted =
        ("#".to_string() + num_format.fmt2(rank_message)).replace(".0", "");
    let rank_voice_formatted = ("#".to_string() + num_format.fmt2(rank_voice)).replace(".0", "");

    let message_today_formatted = num_format.fmt2(message_today).to_string().replace(".0", "");
    let message_week_formatted = num_format.fmt2(message_week).to_string().replace(".0", "");
    let message_total_formatted = num_format.fmt2(message_total).to_string().replace(".0", "");

    let voice_today_formatted = num_format.fmt2(voice_today).to_string();
    let voice_week_formatted = num_format.fmt2(voice_week).to_string();
    let voice_total_formatted = num_format.fmt2(voice_total).to_string();

    let server_ranks_ssh_total_width = sub_section_header_font.measure_text("Total");
    let server_ranks_ssh_message_width =
        sub_section_header_font.measure_text("Message") + (padding * 2.0);
    let server_ranks_ssh_voice_width = sub_section_header_font.measure_text("Voice");

    let message_voice_ssh_today_width = sub_section_header_font.measure_text("Today");
    let message_voice_ssh_week_width = sub_section_header_font.measure_text("Week");
    let message_voice_ssh_total_width =
        sub_section_header_font.measure_text("All Time") + (padding * 2.0);

    canvas.draw_string(
        "Server Ranks",
        middle_section_a_x + padding,
        middle_area_y + (padding / 1.5) + section_header_font_size,
        &section_header_font,
        &primary_text_paint,
    );
    canvas.draw_string(
        "Messages",
        middle_section_b_x + padding,
        middle_area_y + (padding / 1.5) + section_header_font_size,
        &section_header_font,
        &primary_text_paint,
    );
    canvas.draw_string(
        "Voice Activity",
        middle_section_c_x + padding,
        middle_area_y + (padding / 1.5) + section_header_font_size,
        &section_header_font,
        &primary_text_paint,
    );

    let rect = Rect::from_xywh(
        middle_section_a_x + padding,
        middle_sub_section_a_y,
        server_ranks_ssh_message_width,
        middle_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        full_inner_border_radius,
        full_inner_border_radius,
        &primary_background_paint,
    );
    let rect = Rect::from_xywh(
        middle_section_a_x + padding,
        middle_sub_section_b_y,
        server_ranks_ssh_message_width,
        middle_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        full_inner_border_radius,
        full_inner_border_radius,
        &primary_background_paint,
    );
    let rect = Rect::from_xywh(
        middle_section_a_x + padding,
        middle_sub_section_c_y,
        server_ranks_ssh_message_width,
        middle_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        full_inner_border_radius,
        full_inner_border_radius,
        &primary_background_paint,
    );

    let rect = Rect::from_xywh(
        middle_section_b_x + padding,
        middle_sub_section_a_y,
        message_voice_ssh_total_width,
        middle_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        full_inner_border_radius,
        full_inner_border_radius,
        &primary_background_paint,
    );
    let rect = Rect::from_xywh(
        middle_section_b_x + padding,
        middle_sub_section_b_y,
        message_voice_ssh_total_width,
        middle_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        full_inner_border_radius,
        full_inner_border_radius,
        &primary_background_paint,
    );
    let rect = Rect::from_xywh(
        middle_section_b_x + padding,
        middle_sub_section_c_y,
        message_voice_ssh_total_width,
        middle_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        full_inner_border_radius,
        full_inner_border_radius,
        &primary_background_paint,
    );

    let rect = Rect::from_xywh(
        middle_section_c_x + padding,
        middle_sub_section_a_y,
        message_voice_ssh_total_width,
        middle_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        full_inner_border_radius,
        full_inner_border_radius,
        &primary_background_paint,
    );
    let rect = Rect::from_xywh(
        middle_section_c_x + padding,
        middle_sub_section_b_y,
        message_voice_ssh_total_width,
        middle_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        full_inner_border_radius,
        full_inner_border_radius,
        &primary_background_paint,
    );
    let rect = Rect::from_xywh(
        middle_section_c_x + padding,
        middle_sub_section_c_y,
        message_voice_ssh_total_width,
        middle_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        full_inner_border_radius,
        full_inner_border_radius,
        &primary_background_paint,
    );

    canvas.draw_string(
        "Total",
        middle_section_a_x + padding + (server_ranks_ssh_message_width / 2.0)
            - (server_ranks_ssh_total_width / 2.0),
        middle_sub_section_a_y + (padding / 2.2) + sub_section_header_font_size,
        &sub_section_header_font,
        &primary_text_paint,
    );
    canvas.draw_string(
        "Message",
        middle_section_a_x + (padding * 2.0),
        middle_sub_section_b_y + (padding / 2.2) + sub_section_header_font_size,
        &sub_section_header_font,
        &primary_text_paint,
    );
    canvas.draw_string(
        "Voice",
        middle_section_a_x + padding + (server_ranks_ssh_message_width / 2.0)
            - (server_ranks_ssh_voice_width / 2.0),
        middle_sub_section_c_y + (padding / 2.2) + sub_section_header_font_size,
        &sub_section_header_font,
        &primary_text_paint,
    );

    canvas.draw_string(
        "Today",
        middle_section_b_x + padding + (message_voice_ssh_total_width / 2.0)
            - (message_voice_ssh_today_width / 2.0),
        middle_sub_section_a_y + (padding / 2.2) + sub_section_header_font_size,
        &sub_section_header_font,
        &primary_text_paint,
    );
    canvas.draw_string(
        "Week",
        middle_section_b_x + padding + (message_voice_ssh_total_width / 2.0)
            - (message_voice_ssh_week_width / 2.0),
        middle_sub_section_b_y + (padding / 2.2) + sub_section_header_font_size,
        &sub_section_header_font,
        &primary_text_paint,
    );
    canvas.draw_string(
        "All Time",
        middle_section_b_x + (padding * 2.0),
        middle_sub_section_c_y + (padding / 2.2) + sub_section_header_font_size,
        &sub_section_header_font,
        &primary_text_paint,
    );

    canvas.draw_string(
        "Today",
        middle_section_c_x + padding + (message_voice_ssh_total_width / 2.0)
            - (message_voice_ssh_today_width / 2.0),
        middle_sub_section_a_y + (padding / 2.2) + sub_section_header_font_size,
        &sub_section_header_font,
        &primary_text_paint,
    );
    canvas.draw_string(
        "Week",
        middle_section_c_x + padding + (message_voice_ssh_total_width / 2.0)
            - (message_voice_ssh_week_width / 2.0),
        middle_sub_section_b_y + (padding / 2.2) + sub_section_header_font_size,
        &sub_section_header_font,
        &primary_text_paint,
    );
    canvas.draw_string(
        "All Time",
        middle_section_c_x + (padding * 2.0),
        middle_sub_section_c_y + (padding / 2.2) + sub_section_header_font_size,
        &sub_section_header_font,
        &primary_text_paint,
    );

    let rank_total_width = sub_section_font.measure_text(&rank_total_formatted);
    let rank_message_width = sub_section_font.measure_text(&rank_message_formatted);
    let rank_voice_width = sub_section_font.measure_text(&rank_voice_formatted);

    canvas.draw_string(
        &rank_total_formatted,
        middle_section_a_x
            + server_ranks_ssh_message_width
            + padding
            + ((middle_sub_section_width - server_ranks_ssh_message_width) / 2.0)
            - (rank_total_width / 2.0),
        middle_sub_section_a_y + (padding / 2.2) + sub_section_font_size,
        &sub_section_font,
        &primary_text_paint,
    );
    canvas.draw_string(
        &rank_message_formatted,
        middle_section_a_x
            + server_ranks_ssh_message_width
            + padding
            + ((middle_sub_section_width - server_ranks_ssh_message_width) / 2.0)
            - (rank_message_width / 2.0),
        middle_sub_section_b_y + (padding / 2.2) + sub_section_font_size,
        &sub_section_font,
        &primary_text_paint,
    );
    canvas.draw_string(
        &rank_voice_formatted,
        middle_section_a_x
            + server_ranks_ssh_message_width
            + padding
            + ((middle_sub_section_width - server_ranks_ssh_message_width) / 2.0)
            - (rank_voice_width / 2.0),
        middle_sub_section_c_y + (padding / 2.2) + sub_section_font_size,
        &sub_section_font,
        &primary_text_paint,
    );

    let message_today_width = sub_section_font.measure_text(&message_today_formatted);
    let message_week_width = sub_section_font.measure_text(&message_week_formatted);
    let message_total_width = sub_section_font.measure_text(&message_total_formatted);

    canvas.draw_string(
        &message_today_formatted,
        middle_section_b_x + message_voice_ssh_total_width + (padding * 2.0),
        middle_sub_section_a_y + (padding / 2.2) + sub_section_font_size,
        &sub_section_font,
        &primary_text_paint,
    );
    canvas.draw_string(
        &message_week_formatted,
        middle_section_b_x + message_voice_ssh_total_width + (padding * 2.0),
        middle_sub_section_b_y + (padding / 2.2) + sub_section_font_size,
        &sub_section_font,
        &primary_text_paint,
    );
    canvas.draw_string(
        &message_total_formatted,
        middle_section_b_x + message_voice_ssh_total_width + (padding * 2.0),
        middle_sub_section_c_y + (padding / 2.2) + sub_section_font_size,
        &sub_section_font,
        &primary_text_paint,
    );

    canvas.draw_string(
        "msgs",
        middle_section_b_x + message_voice_ssh_total_width + (padding * 2.2) + message_today_width,
        middle_sub_section_a_y + sub_section_small_font_size + (sub_section_font_size / 2.0) - 2.0,
        &sub_section_small_font,
        &secondary_text_paint,
    );
    canvas.draw_string(
        "msgs",
        middle_section_b_x + message_voice_ssh_total_width + (padding * 2.2) + message_week_width,
        middle_sub_section_b_y + sub_section_small_font_size + (sub_section_font_size / 2.0) - 2.0,
        &sub_section_small_font,
        &secondary_text_paint,
    );
    canvas.draw_string(
        "msgs",
        middle_section_b_x + message_voice_ssh_total_width + (padding * 2.2) + message_total_width,
        middle_sub_section_c_y + sub_section_small_font_size + (sub_section_font_size / 2.0) - 2.0,
        &sub_section_small_font,
        &secondary_text_paint,
    );

    let voice_today_width = sub_section_font.measure_text(&voice_today_formatted);
    let voice_week_width = sub_section_font.measure_text(&voice_week_formatted);
    let voice_total_width = sub_section_font.measure_text(&voice_total_formatted);

    canvas.draw_string(
        &voice_today_formatted,
        middle_section_c_x + message_voice_ssh_total_width + (padding * 2.0),
        middle_sub_section_a_y + (padding / 2.2) + sub_section_font_size,
        &sub_section_font,
        &primary_text_paint,
    );
    canvas.draw_string(
        &voice_week_formatted,
        middle_section_c_x + message_voice_ssh_total_width + (padding * 2.0),
        middle_sub_section_b_y + (padding / 2.2) + sub_section_font_size,
        &sub_section_font,
        &primary_text_paint,
    );
    canvas.draw_string(
        &voice_total_formatted,
        middle_section_c_x + message_voice_ssh_total_width + (padding * 2.0),
        middle_sub_section_c_y + (padding / 2.2) + sub_section_font_size,
        &sub_section_font,
        &primary_text_paint,
    );

    canvas.draw_string(
        "mins",
        middle_section_c_x + message_voice_ssh_total_width + (padding * 2.2) + voice_today_width,
        middle_sub_section_a_y + sub_section_small_font_size + (sub_section_font_size / 2.0) - 2.0,
        &sub_section_small_font,
        &secondary_text_paint,
    );
    canvas.draw_string(
        "mins",
        middle_section_c_x + message_voice_ssh_total_width + (padding * 2.2) + voice_week_width,
        middle_sub_section_b_y + sub_section_small_font_size + (sub_section_font_size / 2.0) - 2.0,
        &sub_section_small_font,
        &secondary_text_paint,
    );
    canvas.draw_string(
        "mins",
        middle_section_c_x + message_voice_ssh_total_width + (padding * 2.2) + voice_total_width,
        middle_sub_section_c_y + sub_section_small_font_size + (sub_section_font_size / 2.0) - 2.0,
        &sub_section_small_font,
        &secondary_text_paint,
    );

    // game section
    canvas.draw_string(
        "Game Stats",
        game_section_x + padding,
        bottom_area_y + (padding / 1.5) + section_header_font_size,
        &section_header_font,
        &primary_text_paint,
    );
    canvas.draw_string(
        "Last Played",
        game_section_x + (padding * 1.5),
        game_sub_section_a_y - (padding / 4.0)
            + (game_sub_section_header_height / 2.0)
            + (sub_section_header_font_size / 2.0),
        &sub_section_header_font,
        &primary_text_paint,
    );
    canvas.draw_string(
        "Most Played",
        game_section_x + (padding * 1.5),
        game_sub_section_b_y - (padding / 4.0)
            + (game_sub_section_header_height / 2.0)
            + (sub_section_header_font_size / 2.0),
        &sub_section_header_font,
        &primary_text_paint,
    );
    let actvitiy_name_length = 25;
    let mut last_played_activity_name = last_played_activity_name;
    let mut most_played_activity_name = most_played_activity_name.to_string();
    if last_played_activity_name.chars().count() > actvitiy_name_length {
        let mut string = last_played_activity_name.clone();
        string.truncate(actvitiy_name_length - 3);
        last_played_activity_name = string.clone() + &"...";
    }
    if most_played_activity_name.chars().count() > actvitiy_name_length {
        let mut string = most_played_activity_name.clone();
        string.truncate(actvitiy_name_length - 3);
        most_played_activity_name = string.clone() + &"...";
    }
    canvas.draw_string(
        &last_played_activity_name,
        game_section_x + (padding * 1.5),
        game_sub_section_a_y + game_sub_section_header_height - (padding / 2.0)
            + (game_sub_section_header_height / 2.0)
            + (sub_section_font_size / 2.0),
        &sub_section_font,
        &primary_text_paint,
    );
    canvas.draw_string(
        &most_played_activity_name,
        game_section_x + (padding * 1.5),
        game_sub_section_b_y + game_sub_section_header_height - (padding / 2.0)
            + (game_sub_section_header_height / 2.0)
            + (sub_section_font_size / 2.0),
        &sub_section_font,
        &primary_text_paint,
    );
    canvas.draw_string(
        &last_played_activity_time,
        game_section_x + (padding * 1.5),
        game_sub_section_a_y + game_sub_section_height - (padding / 2.0),
        &sub_section_small_font,
        &secondary_text_paint,
    );
    canvas.draw_string(
        &most_played_activity_time,
        game_section_x + (padding * 1.5),
        game_sub_section_b_y + game_sub_section_height - (padding / 2.0),
        &sub_section_small_font,
        &secondary_text_paint,
    );

    // chart section
    canvas.draw_string(
        "Activity Last 45 Days",
        chart_section_x + padding,
        bottom_area_y + (padding / 1.5) + section_header_font_size,
        &section_header_font,
        &primary_text_paint,
    );

    // chart chips
    let chip_font = bold_font(20.0);
    let chip_sub_font = italic_font(16.0);
    let mut chip_text_paint = Paint::new();
    chip_text_paint.set_anti_alias(true);
    chip_text_paint.set_color(graph_game_color.to_color4f());
    chip_text_paint.set_style(Style::Fill);
    let hour_sub_text_size = chip_font.measure_text(&"(HOURS)");
    let count_sub_text_size = chip_font.measure_text(&"(COUNT)");
    let game_chip_text_size = chip_font.measure_text(&"Game");
    let game_chip_x = chart_x + chart_width - game_chip_text_size - hour_sub_text_size;
    let rect = Rect::from_xywh(
        game_chip_x,
        bottom_area_y + padding,
        game_chip_text_size + hour_sub_text_size,
        chip_font.size() + (padding / 2.0),
    );
    canvas.draw_round_rect(
        &rect,
        full_inner_border_radius,
        full_inner_border_radius,
        &primary_background_paint,
    );
    canvas.draw_string(
        "Game",
        (padding / 2.0) + game_chip_x + 1.0,
        bottom_area_y + padding + chip_font.size(),
        &chip_font,
        &chip_text_paint,
    );
    canvas.draw_string(
        "(HOURS)",
        (padding / 2.0) + game_chip_x + game_chip_text_size + 5.0,
        bottom_area_y + padding + chip_font.size(),
        &chip_sub_font,
        &secondary_text_paint,
    );
    chip_text_paint.set_color(graph_voice_color.to_color4f());
    let voice_chip_text_size = chip_font.measure_text(&"Voice");
    let voice_chip_x = game_chip_x - voice_chip_text_size - hour_sub_text_size - (padding / 2.0);
    let rect = Rect::from_xywh(
        voice_chip_x,
        bottom_area_y + padding,
        voice_chip_text_size + hour_sub_text_size,
        chip_font.size() + (padding / 2.0),
    );
    canvas.draw_round_rect(
        &rect,
        full_inner_border_radius,
        full_inner_border_radius,
        &primary_background_paint,
    );
    canvas.draw_string(
        "Voice",
        (padding / 2.0) + voice_chip_x + 1.0,
        bottom_area_y + padding + chip_font.size(),
        &chip_font,
        &chip_text_paint,
    );
    canvas.draw_string(
        "(HOURS)",
        (padding / 2.0) + voice_chip_x + voice_chip_text_size + 5.0,
        bottom_area_y + padding + chip_font.size(),
        &chip_sub_font,
        &secondary_text_paint,
    );
    chip_text_paint.set_color(graph_message_color.to_color4f());
    let message_chip_text_size = chip_font.measure_text(&"Message");
    let message_chip_x =
        voice_chip_x - message_chip_text_size - hour_sub_text_size - (padding / 2.0);
    let rect = Rect::from_xywh(
        message_chip_x,
        bottom_area_y + padding,
        message_chip_text_size + count_sub_text_size,
        chip_font.size() + (padding / 2.0),
    );
    canvas.draw_round_rect(
        &rect,
        full_inner_border_radius,
        full_inner_border_radius,
        &primary_background_paint,
    );
    canvas.draw_string(
        "Message",
        (padding / 2.0) + message_chip_x + 1.0,
        bottom_area_y + padding + chip_font.size(),
        &chip_font,
        &chip_text_paint,
    );
    canvas.draw_string(
        "(COUNT)",
        (padding / 2.0) + message_chip_x + message_chip_text_size + 5.0,
        bottom_area_y + padding + chip_font.size(),
        &chip_sub_font,
        &secondary_text_paint,
    );

    // chart
    let mut message_history: Vec<f64> = Vec::new();
    let mut voice_history: Vec<f64> = Vec::new();
    let mut game_history: Vec<f64> = Vec::new();

    for (_, history) in &activity_data.total {
        message_history.push(history.message_count as f64);
        voice_history.push(history.voice_time / 60.0);
        game_history.push(history.game_time / 60.0 / 60.0);
    }

    draw_grid(
        &mut canvas,
        chart_x,
        chart_y,
        chart_width,
        chart_height,
        full_inner_border_radius,
        secondary_background_color,
        45,
        12,
    );

    //// draw in reverse order so messages are on top
    generate_graph(
        &mut canvas,
        chart_x,
        chart_y,
        chart_width,
        chart_height,
        full_inner_border_radius,
        graph_game_color,
        &game_history,
    );
    generate_graph(
        &mut canvas,
        chart_x,
        chart_y,
        chart_width,
        chart_height,
        full_inner_border_radius,
        graph_voice_color,
        &voice_history,
    );
    generate_graph(
        &mut canvas,
        chart_x,
        chart_y,
        chart_width,
        chart_height,
        full_inner_border_radius,
        graph_message_color,
        &message_history,
    );

    generate_graph_labels(
        &mut canvas,
        chart_x,
        chart_y,
        chart_width,
        chart_height,
        full_inner_border_radius,
        primary_text_color,
        vec![graph_message_color, graph_voice_color, graph_game_color],
        vec![message_history, voice_history, game_history],
    );

    // prepare output
    let width = surface.width();
    let img_info = skia_rs::codec::ImageInfo::new(
        width,
        surface.height(),
        skia_rs::core::ColorType::Rgba8888,
        skia_rs::core::AlphaType::Premul,
    );

    let data =
        skia_rs::codec::Image::from_raster_data(&img_info, surface.pixels(), width as usize * 4)
            .unwrap();
    let encoder = PngEncoder::new();
    Some(encoder.encode_bytes(&data).unwrap())
}

fn cmp_f64(a: &f64, b: &f64) -> Ordering {
    if a < b {
        return Ordering::Less;
    } else if a > b {
        return Ordering::Greater;
    }
    Ordering::Equal
}

fn cmp_i64(a: &i64, b: &i64) -> Ordering {
    if a < b {
        return Ordering::Less;
    } else if a > b {
        return Ordering::Greater;
    }
    Ordering::Equal
}

async fn load_image_from_url(url: &str) -> Option<Image> {
    let start = Local::now().timestamp_millis();
    let url = url.replace("?size=1024", "?size=80");
    let response = match reqwest::get(url).await {
        Ok(data) => data,
        Err(_) => {
            return None;
        }
    };
    let bytes = match response.bytes().await {
        Ok(data) => data,
        Err(_) => {
            return None;
        }
    };

    let image = match image::load_from_memory(&bytes) {
        Ok(data) => data.to_rgba8(),
        Err(_) => {
            return None;
        }
    };

    let width = image.width() as i32;
    let height = image.height() as i32;
    let pixels = image.into_vec();
    let row_bytes = (width as usize) * 4;

    let info = skia_rs::codec::ImageInfo::new(
        width,
        height,
        skia_rs::core::ColorType::Rgba8888,
        skia_rs::core::AlphaType::Premul,
    );
    println!(
        "IMAGE GET: {:?}ms",
        Local::now().timestamp_millis() - &start
    );
    Image::from_raster_data_owned(info, pixels, row_bytes)
}

fn draw_grid(
    canvas: &mut Canvas<'_>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    padding: f32,
    color: Color,
    columns: usize,
    rows: usize,
) {
    // add half line thickness to padding
    let padding = padding + 1.5;
    let cell_width = (width - padding * 2.0) / columns as f32;
    let cell_height = (height - padding * 2.0) / rows as f32;
    let mut line_paint = Paint::new();
    line_paint.set_color(color.to_color4f());
    for i in 0..columns + 1 {
        let x = x + padding + i as f32 * cell_width;
        let p0 = Point::new(x, y + padding);
        let p1 = Point::new(x, y + height - padding);
        canvas.draw_line(p0, p1, &line_paint);
    }

    for i in 0..rows + 1 {
        let y = y + padding + i as f32 * cell_height;
        let p0 = Point::new(x + padding, y);
        let p1 = Point::new(x + width - padding, y);
        canvas.draw_line(p0, p1, &line_paint);
    }
}

fn generate_graph(
    canvas: &mut Canvas<'_>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    padding: f32,
    color: Color,
    values: &Vec<f64>,
) {
    let thickness = 3.0;
    let half_thickness = thickness / 2.0;
    let dot_size = thickness;
    let graph_width = width - padding * 2.0 - dot_size;
    let graph_height = height - padding * 2.0 - dot_size;

    let min_value = 0.0; // if I want min to be min of data set //values.iter().copied().fold(f64::INFINITY, f64::min);
    let max_value = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    let value_range = if (max_value - min_value).abs() < f64::EPSILON {
        1.0
    } else {
        max_value - min_value
    };

    let mut paint = Paint::new();
    paint.set_anti_alias(true);
    paint.set_color(color.to_color4f());
    paint.set_style(Style::Fill);
    // Does not work AT ALL for some reason
    paint.set_stroke_width(thickness);

    let points: Vec<(f32, f32)> = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let x = if values.len() == 1 {
                x + padding + dot_size / 2.0 + graph_width / 2.0
            } else {
                x + padding
                    + dot_size / 2.0
                    + index as f32 * graph_width / (values.len() - 1) as f32
            };

            let normalized = ((*value - min_value) / value_range) as f32;
            let y = y + padding + dot_size / 2.0 + graph_height - normalized * graph_height;

            (x, y)
        })
        .collect();

    for pair in points.clone() {
        let (x, y) = pair;
        let point = Point::new(x, y);
        canvas.draw_circle(point, dot_size, &paint);
    }

    for pair in points.windows(2) {
        let (x1, y1) = pair[0];
        let (x2, y2) = pair[1];

        // if stroke width worked
        // let point1 = Point::new(x1, y1);
        // let point2 = Point::new(x2, y2);
        //
        // canvas.draw_line(point1, point2, &paint);

        let dx = x2 - x1;
        let dy = y2 - y1;
        let length = (dx * dx + dy * dy).sqrt();

        if length == 0.0 {
            continue;
        }

        let normal_x = -dy / length;
        let normal_y = dx / length;

        let line_count = thickness.ceil() as i32;

        for index in 0..line_count {
            let offset = -half_thickness + index as f32;

            let offset_x = normal_x * offset;
            let offset_y = normal_y * offset;

            canvas.draw_line(
                Point::new(x1 + offset_x, y1 + offset_y),
                Point::new(x2 + offset_x, y2 + offset_y),
                &paint,
            );
        }
    }
}

fn generate_graph_labels(
    canvas: &mut Canvas<'_>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    padding: f32,
    dot_color: Color,
    label_colors: Vec<Color>,
    values: Vec<Vec<f64>>,
) {
    let thickness = 3.0;
    let dot_size = thickness * 1.0;
    let graph_width = width - padding * 2.0 - dot_size;
    let graph_height = height - padding * 2.0 - dot_size;
    let font = bold_font(18.0);

    let min_values = values
        .iter()
        .map(|v| v.iter().copied().fold(f64::INFINITY, f64::min))
        .collect::<Vec<_>>();

    let value_ranges = values
        .iter()
        .map(|v| {
            v.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                - v.iter().copied().fold(f64::INFINITY, f64::min)
        })
        .collect::<Vec<_>>();

    let mut paint = Paint::new();
    paint.set_anti_alias(true);

    let points: Vec<(f32, f32, f64)> = values
        .iter()
        .enumerate()
        .filter_map(|(outer_index, series)| {
            let (max_index, &max_value) = series
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.total_cmp(b))?;

            let normalized = if value_ranges[outer_index].abs() < f64::EPSILON {
                0.5
            } else {
                ((max_value - min_values[outer_index]) / value_ranges[outer_index]) as f32
            };

            let point_x = if series.len() == 1 {
                x + padding + dot_size / 2.0 + graph_width / 2.0
            } else {
                x + padding
                    + dot_size / 2.0
                    + max_index as f32 * graph_width / (series.len() - 1) as f32
            };

            let point_y = y + padding + dot_size / 2.0 + graph_height - normalized * graph_height;

            Some((point_x, point_y, max_value))
        })
        .collect();

    let mut used_x_values: Vec<f32> = Vec::new();
    for (index, &(point_x, point_y, value)) in points.iter().enumerate() {
        if value < 0.001 {
            continue;
        }
        let count = used_x_values
            .iter()
            .filter(|value| (**value - point_x).abs() < f32::EPSILON)
            .count() as f32;
        let text_size = font.measure_text(&format!("{}", value.round()));
        let mut offset_y = font.size() * count;
        let mut offset_x = point_x + dot_size + (padding / 2.0);
        // check if the label fits within the graph width, adjust offset_x if not
        if point_x + text_size > x + graph_width {
            offset_x = point_x - dot_size - (padding / 2.0) - text_size;
        }
        // check if label overlaps with another label, adjust offset_x if so
        let mut u_count = 0.0;
        for &used_x in &used_x_values {
            // check for overlap and if off graph region
            if point_x - text_size < x && (used_x - point_x).abs() <= (text_size * 2.0) {
                u_count += 1.0;
                offset_y += font.size() * u_count;
                offset_x = used_x + dot_size + (padding / 2.0);
            }
            // check for just overlap
            else if (used_x - point_x).abs() <= (text_size * 2.0) {
                offset_x = point_x - dot_size - (padding / 2.0) - text_size;
            }
        }

        used_x_values.push(point_x);

        paint.set_color(dot_color.to_color4f());
        canvas.draw_circle(Point::new(point_x, point_y), dot_size, &paint);

        paint.set_style(Style::Fill);
        paint.set_color(label_colors[index].to_color4f());
        canvas.draw_string(
            &format!("{}", value.round()),
            offset_x,
            point_y + offset_y + font.size() / 2.0,
            &font,
            &paint,
        );
    }
}

async fn guild_leaderboard_card(ctx: Context<'_>) -> Option<Vec<u8>> {
    let guild_data = guild_data::Entity::find()
        .filter(guild_data::Column::GuildId.eq(&ctx.guild_id().unwrap().to_string()))
        .one(&ctx.data().database)
        .await
        .unwrap();
    let users = user_data::Entity::find()
        .filter(user_data::Column::GuildId.eq(&ctx.guild_id().unwrap().to_string()))
        .filter(user_data::Column::UserLeft.ne(true))
        .columns([
            user_data::Column::UserId,
            user_data::Column::Username,
            user_data::Column::DisplayName,
            user_data::Column::GlobalName,
            user_data::Column::Nickname,
            user_data::Column::Avatar,
            user_data::Column::MessageCount,
            user_data::Column::VoiceTime,
            user_data::Column::Total,
        ])
        .all(&ctx.data().database)
        .await
        .unwrap();

    // should never be the case but who knows
    if users.is_empty() {
        return None;
    }

    let message_total = users.iter().map(|u| u.message_count).sum::<i64>();
    let voice_total = users.iter().map(|u| u.voice_time).sum::<f64>();
    let total = users.iter().map(|u| u.total).sum::<f64>();

    println!("{message_total} {voice_total} {total}");

    let mut num_format = Formatter::new()
        .precision(numfmt::Precision::Decimals(2))
        .separator(',')
        .unwrap();

    let positions = get_user_positions(ctx.clone()).await;

    let width = 1280.0;
    let height = 1013.0;
    let mut surface = Surface::new_raster_n32_premul(width as i32, height as i32)
        .expect("Failed to create surface");
    let mut canvas = surface.canvas();

    let primary_text_color = Color::from_rgb(217, 219, 227);
    let secondary_text_color = Color::from_rgb(137, 144, 164);
    let secondary_text_color_trans = Color::from_argb(64, 137, 144, 164);
    let primary_background_color = Color::from_rgb(15, 18, 26);
    let secondary_background_color = Color::from_rgb(25, 28, 36);
    let tertiary_background_color = Color::from_rgb(33, 36, 45);
    let quaternary_background_color = Color::from_rgb(45, 49, 60);

    // theme black
    // let primary_text_color = Color::from_rgb(217, 219, 227);
    // let secondary_text_color = Color::from_rgb(137, 144, 164);
    // let secondary_text_color_trans = Color::from_argb(64, 137, 144, 164);
    // let primary_background_color = Color::from_rgb(0, 0, 0);
    // let secondary_background_color = Color::from_rgb(10, 10, 10);
    // let tertiary_background_color = Color::from_rgb(20, 20, 20);
    // let quaternary_background_color = Color::from_rgb(30, 30, 30);

    let padding = 16.0;
    let outer_border_radius = 20.0;
    let border_radius = outer_border_radius - (padding / 2.0);
    let inner_border_radius = border_radius / 2.0;
    let full_inner_border_radius = inner_border_radius / 2.0;
    let header_height = 90.0;
    let header_font_size = (header_height - padding) / 2.0;
    let sub_font_size = 26.0;
    let header_font = bold_font(header_font_size);
    let sub_font = default_font(sub_font_size);

    let default_icon_font_size = header_height;
    let default_icon_font = bold_font(default_icon_font_size);

    let section_header_font_size = 30.0;
    let sub_section_header_font_size = 28.0;
    let sub_section_font_size = 26.0;
    let sub_section_small_font_size = 22.0;
    let section_header_font = bold_font(section_header_font_size);
    let sub_section_header_font = bold_font(sub_section_header_font_size);
    let sub_section_font = default_font(sub_section_font_size);
    let sub_section_small_font = default_font(sub_section_small_font_size);
    let sub_section_small_italic_font = italic_font(sub_section_small_font_size);

    let user_position_font_size = 36.0;
    let user_position_font = italic_font(user_position_font_size);

    let middle_section_header_height = 60.0;
    let middle_section_width = width - (padding * 2.0);
    let middle_section_height =
        middle_section_header_height + sub_section_header_font_size + (padding * 2.3);
    let middle_section_x = padding;
    let middle_section_y = header_height + (padding * 2.0);

    let middle_sub_sections = 3.0;
    let middle_sub_section_width =
        (middle_section_width - (padding * (middle_sub_sections + 1.0))) / middle_sub_sections;
    let middle_sub_section_height =
        (middle_section_height - middle_section_header_height) - padding;
    let middle_sub_section_y = middle_section_y + middle_section_header_height;
    let middle_sub_section_a_x = middle_section_x + padding;
    let middle_sub_section_b_x = middle_sub_section_a_x + padding + middle_sub_section_width;
    let middle_sub_section_c_x = middle_sub_section_b_x + padding + middle_sub_section_width;

    let bottom_area_y = middle_section_y + middle_section_height + padding;
    let bottom_area_height = height - (padding * 4.0) - header_height - middle_section_height;
    let bottom_sections = 3.0;
    let bottom_section_width = (width - (padding * (bottom_sections + 1.0))) / bottom_sections;
    let bottom_section_header_height = 60.0;
    let bottom_section_height =
        (bottom_area_height + bottom_section_header_height) + (padding * 2.3);
    let bottom_section_a_x = padding;
    let bottom_section_b_x = bottom_section_a_x + padding + bottom_section_width;
    let bottom_section_c_x = bottom_section_b_x + padding + bottom_section_width;

    let bottom_sub_section_width = bottom_section_width - (padding * 2.0);
    let bottom_sub_section_height =
        (bottom_section_height - bottom_section_header_height) - (padding * 7.0);
    let bottom_sub_section_y = bottom_area_y + bottom_section_header_height;
    let bottom_sub_section_a_x = bottom_section_a_x + padding;
    let bottom_sub_section_b_x =
        bottom_sub_section_a_x + (padding * 3.0) + bottom_sub_section_width;
    let bottom_sub_section_c_x =
        bottom_sub_section_b_x + (padding * 3.0) + bottom_sub_section_width;

    let mut debug_paint = Paint::new();
    debug_paint.set_anti_alias(true);
    debug_paint.set_color(Color::from_rgb(255, 120, 120).to_color4f());
    debug_paint.set_style(Style::Fill);

    let mut primary_text_paint = Paint::new();
    primary_text_paint.set_anti_alias(true);
    primary_text_paint.set_color(primary_text_color.to_color4f());
    primary_text_paint.set_style(Style::Fill);

    let mut secondary_text_paint = Paint::new();
    secondary_text_paint.set_anti_alias(true);
    secondary_text_paint.set_color(secondary_text_color.to_color4f());
    secondary_text_paint.set_style(Style::Fill);

    let mut secondary_text_trans_paint = Paint::new();
    secondary_text_trans_paint.set_anti_alias(true);
    secondary_text_trans_paint.set_color(secondary_text_color_trans.to_color4f());
    secondary_text_trans_paint.set_style(Style::Fill);

    let mut primary_background_paint = Paint::new();
    primary_background_paint.set_anti_alias(true);
    primary_background_paint.set_color(primary_background_color.to_color4f());
    primary_background_paint.set_style(Style::Fill);

    let mut secondary_background_paint = Paint::new();
    secondary_background_paint.set_anti_alias(true);
    secondary_background_paint.set_color(secondary_background_color.to_color4f());
    secondary_background_paint.set_style(Style::Fill);

    let mut tertiary_background_paint = Paint::new();
    tertiary_background_paint.set_anti_alias(true);
    tertiary_background_paint.set_color(tertiary_background_color.to_color4f());
    tertiary_background_paint.set_style(Style::Fill);

    let mut quaternary_background_paint = Paint::new();
    quaternary_background_paint.set_anti_alias(true);
    quaternary_background_paint.set_color(quaternary_background_color.to_color4f());
    quaternary_background_paint.set_style(Style::Fill);

    let rect = Rect::from_xywh(0.0, 0.0, width, height);
    canvas.draw_round_rect(
        &rect,
        outer_border_radius,
        outer_border_radius,
        &primary_background_paint,
    );

    let rect = Rect::from_xywh(padding, padding, header_height, header_height);
    canvas.draw_round_rect(
        &rect,
        border_radius,
        border_radius,
        &secondary_background_paint,
    );
    if let Some(icon_url) = guild_data.clone().unwrap().icon {
        if let Some(icon) = load_image_from_url(&icon_url).await {
            canvas.save();
            let mut builder = PathBuilder::new();
            builder.add_round_rect(&rect, border_radius, border_radius);
            canvas.clip_path(&builder.build(), ClipOp::Intersect, true);
            canvas.draw_image_rect(&icon, None, &rect, Some(&secondary_background_paint));
            canvas.restore();
        } else {
            draw_username_letter(
                &mut canvas,
                &guild_data.clone().unwrap().name,
                padding,
                padding,
                header_height,
                header_height,
                &default_icon_font,
                &primary_text_paint,
            );
        }
    } else {
        draw_username_letter(
            &mut canvas,
            &guild_data.clone().unwrap().name,
            padding,
            padding,
            header_height,
            header_height,
            &default_icon_font,
            &primary_text_paint,
        );
    }

    fn draw_username_letter(
        canvas: &mut Canvas<'_>,
        username: &str,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        font: &Font,
        color: &Paint,
    ) {
        let char = username
            .chars()
            .next()
            .unwrap()
            .to_ascii_uppercase()
            .to_string();
        let char_width = font.measure_text(&char);
        let metrics = font.metrics();
        let baseline_y = (y / 1.5) + (height / 2.0) - ((metrics.ascent + metrics.descent) / 2.0);

        canvas.draw_string(
            &char,
            x + (width / 2.0) - (char_width / 2.0),
            baseline_y,
            font,
            color,
        );
    }

    //middle area sections
    let rect = Rect::from_xywh(
        middle_section_x,
        middle_section_y,
        middle_section_width,
        middle_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        border_radius,
        border_radius,
        &tertiary_background_paint,
    );

    //bottom area sections
    let rect = Rect::from_xywh(
        bottom_section_a_x,
        bottom_area_y,
        bottom_section_width,
        bottom_area_height,
    );
    canvas.draw_round_rect(
        &rect,
        border_radius,
        border_radius,
        &tertiary_background_paint,
    );
    let rect = Rect::from_xywh(
        bottom_section_b_x,
        bottom_area_y,
        bottom_section_width,
        bottom_area_height,
    );
    canvas.draw_round_rect(
        &rect,
        border_radius,
        border_radius,
        &tertiary_background_paint,
    );
    let rect = Rect::from_xywh(
        bottom_section_c_x,
        bottom_area_y,
        bottom_section_width,
        bottom_area_height,
    );
    canvas.draw_round_rect(
        &rect,
        border_radius,
        border_radius,
        &tertiary_background_paint,
    );

    //middle sub sections
    let rect = Rect::from_xywh(
        middle_sub_section_a_x,
        middle_sub_section_y,
        middle_sub_section_width,
        middle_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        inner_border_radius,
        inner_border_radius,
        &secondary_background_paint,
    );
    let rect = Rect::from_xywh(
        middle_sub_section_b_x,
        middle_sub_section_y,
        middle_sub_section_width,
        middle_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        inner_border_radius,
        inner_border_radius,
        &secondary_background_paint,
    );
    let rect = Rect::from_xywh(
        middle_sub_section_c_x,
        middle_sub_section_y,
        middle_sub_section_width,
        middle_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        inner_border_radius,
        inner_border_radius,
        &secondary_background_paint,
    );

    //bottom sub sections
    let rect = Rect::from_xywh(
        bottom_sub_section_a_x,
        bottom_sub_section_y,
        bottom_sub_section_width,
        bottom_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        inner_border_radius,
        inner_border_radius,
        &primary_background_paint,
    );
    let rect = Rect::from_xywh(
        bottom_sub_section_b_x,
        bottom_sub_section_y,
        bottom_sub_section_width,
        bottom_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        inner_border_radius,
        inner_border_radius,
        &primary_background_paint,
    );
    let rect = Rect::from_xywh(
        bottom_sub_section_c_x,
        bottom_sub_section_y,
        bottom_sub_section_width,
        bottom_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        inner_border_radius,
        inner_border_radius,
        &primary_background_paint,
    );

    // card header
    let member_count = &format!("{} Members", users.len());
    canvas.draw_string(
        &guild_data.clone().unwrap().name,
        header_height + (padding * 2.0),
        padding + header_font_size,
        &header_font,
        &primary_text_paint,
    );
    canvas.draw_string(
        member_count,
        header_height + (padding * 2.0),
        (padding * 2.0) + header_font_size + sub_font_size,
        &sub_font,
        &secondary_text_paint,
    );

    // server totals
    let message_total_formatted = num_format.fmt2(message_total).to_string().replace(".0", "");
    let voice_total_formatted = num_format.fmt2(voice_total).to_string().replace(".0", "");
    let total_formatted = num_format.fmt2(total).to_string().replace(".0", "");

    let message_ssh_width = sub_section_header_font.measure_text("Messages") + (padding * 2.0);
    let voice_ssh_width = sub_section_header_font.measure_text("Voice") + (padding * 2.0);
    let total_ssh_width = sub_section_header_font.measure_text("Total") + (padding * 2.0);

    let voice_total_width = sub_section_font.measure_text(&voice_total_formatted);

    canvas.draw_string(
        "Server Totals",
        middle_section_x + padding,
        middle_section_y + (padding / 1.5) + section_header_font_size,
        &section_header_font,
        &primary_text_paint,
    );

    let rect = Rect::from_xywh(
        middle_sub_section_a_x,
        middle_sub_section_y,
        message_ssh_width,
        middle_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        inner_border_radius,
        inner_border_radius,
        &primary_background_paint,
    );
    canvas.draw_string(
        "Messages",
        middle_sub_section_a_x + padding,
        middle_sub_section_y + (padding / 2.2) + sub_section_header_font_size,
        &sub_section_header_font,
        &primary_text_paint,
    );
    canvas.draw_string(
        &message_total_formatted,
        middle_sub_section_a_x + padding + message_ssh_width,
        middle_sub_section_y + (padding / 2.2) + sub_section_font_size,
        &sub_section_font,
        &primary_text_paint,
    );

    let rect = Rect::from_xywh(
        middle_sub_section_b_x,
        middle_sub_section_y,
        voice_ssh_width,
        middle_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        inner_border_radius,
        inner_border_radius,
        &primary_background_paint,
    );
    canvas.draw_string(
        "Voice",
        middle_sub_section_b_x + padding,
        middle_sub_section_y + (padding / 2.2) + sub_section_header_font_size,
        &sub_section_header_font,
        &primary_text_paint,
    );
    canvas.draw_string(
        &voice_total_formatted,
        middle_sub_section_b_x + padding + voice_ssh_width,
        middle_sub_section_y + (padding / 2.2) + sub_section_font_size,
        &sub_section_font,
        &primary_text_paint,
    );
    canvas.draw_string(
        "mins",
        middle_sub_section_b_x + (padding * 1.2) + voice_ssh_width + voice_total_width,
        middle_sub_section_y + sub_section_small_font_size + (sub_section_font_size / 2.0) - 2.0,
        &sub_section_small_italic_font,
        &secondary_text_paint,
    );

    let rect = Rect::from_xywh(
        middle_sub_section_c_x,
        middle_sub_section_y,
        total_ssh_width,
        middle_sub_section_height,
    );
    canvas.draw_round_rect(
        &rect,
        inner_border_radius,
        inner_border_radius,
        &primary_background_paint,
    );
    canvas.draw_string(
        "Total",
        middle_sub_section_c_x + padding,
        middle_sub_section_y + (padding / 2.2) + sub_section_header_font_size,
        &sub_section_header_font,
        &primary_text_paint,
    );
    canvas.draw_string(
        &total_formatted,
        middle_sub_section_c_x + padding + total_ssh_width,
        middle_sub_section_y + (padding / 2.2) + sub_section_font_size,
        &sub_section_font,
        &primary_text_paint,
    );

    // message ranks
    canvas.draw_string(
        "Message Ranks",
        bottom_section_a_x + padding,
        bottom_area_y + (padding / 1.5) + section_header_font_size,
        &section_header_font,
        &primary_text_paint,
    );

    let mut avatars: Vec<(String, Image)> = Vec::new();

    for (index, user_id) in positions.message.iter().enumerate() {
        if index >= 10 {
            break;
        }
        let user = users
            .iter()
            .find(|u| &u.user_id == user_id)
            .unwrap()
            .to_owned();

        let nickname = user.nickname;
        let display_name = user.display_name;
        let global_name = user.global_name;
        let username = user.username;

        let mut display_name =
            nickname.unwrap_or(display_name.unwrap_or(global_name.unwrap_or(username.clone())));
        let display_name_max_length = 32;

        if display_name.chars().count() > display_name_max_length {
            let mut string = display_name.clone();
            string.truncate(display_name_max_length - 3);
            display_name = string.clone() + &"...";
        }

        let width = bottom_sub_section_width - (padding / 1.5);
        let height = (sub_section_font_size * 2.0) + (padding / 2.0);
        let x = bottom_sub_section_a_x + (padding / 3.0);
        let y =
            bottom_sub_section_y + (padding / 3.0) + ((height + (padding / 3.0)) * (index as f32));

        let rect = Rect::from_xywh(x, y, width, height);
        canvas.draw_round_rect(
            &rect,
            full_inner_border_radius,
            full_inner_border_radius,
            &secondary_background_paint,
        );

        let rect = Rect::from_xywh(
            x + (padding / 3.0),
            y + (padding / 3.0),
            height - (padding / 1.5),
            height - (padding / 1.5),
        );
        canvas.draw_round_rect(
            &rect,
            full_inner_border_radius / 2.0,
            full_inner_border_radius / 2.0,
            &quaternary_background_paint,
        );
        let start = Local::now().timestamp_millis();
        if let Some(avatar_url) = user.avatar {
            if let Some(avatar) = avatars.iter().find(|u| &u.0 == user_id) {
                let start = Local::now().timestamp_millis();
                canvas.save();
                let mut builder = PathBuilder::new();
                builder.add_round_rect(
                    &rect,
                    full_inner_border_radius / 2.0,
                    full_inner_border_radius / 2.0,
                );
                canvas.clip_path(&builder.build(), ClipOp::Intersect, false);
                canvas.draw_image_rect(&avatar.1, None, &rect, Some(&secondary_background_paint));
                canvas.restore();
                println!(
                    "IMAGE DRAW: {:?}ms",
                    Local::now().timestamp_millis() - &start
                );
            } else {
                if let Some(avatar) = load_image_from_url(&avatar_url).await {
                    let start = Local::now().timestamp_millis();
                    avatars.push((user_id.clone(), avatar.clone()));
                    let new = Local::now().timestamp_millis();
                    canvas.save();
                    println!(
                        "CANVAS SAVE: {:?}ms",
                        Local::now().timestamp_millis() - &new
                    );
                    let new = Local::now().timestamp_millis();
                    let mut builder = PathBuilder::new();
                    builder.add_round_rect(
                        &rect,
                        full_inner_border_radius / 2.0,
                        full_inner_border_radius / 2.0,
                    );
                    println!("PATH BUILD: {:?}ms", Local::now().timestamp_millis() - &new);
                    let new = Local::now().timestamp_millis();
                    canvas.clip_path(&builder.build(), ClipOp::Intersect, true);
                    println!("CLIP PATH: {:?}ms", Local::now().timestamp_millis() - &new);
                    let new = Local::now().timestamp_millis();
                    canvas.draw_image_rect(&avatar, None, &rect, Some(&secondary_background_paint));
                    println!("IMAGE RECT: {:?}ms", Local::now().timestamp_millis() - &new);
                    let new = Local::now().timestamp_millis();
                    canvas.restore();
                    println!(
                        "CANVAS RESTORE: {:?}ms",
                        Local::now().timestamp_millis() - &new
                    );
                    println!(
                        "IMAGE DRAW: {:?}ms",
                        Local::now().timestamp_millis() - &start
                    );
                } else {
                    draw_username_letter(
                        &mut canvas,
                        &username,
                        padding,
                        padding,
                        header_height,
                        header_height,
                        &section_header_font,
                        &primary_text_paint,
                    );
                }
            }
        } else {
            draw_username_letter(
                &mut canvas,
                &username,
                padding,
                padding,
                header_height,
                header_height,
                &section_header_font,
                &primary_text_paint,
            );
        }
        println!(
            "FULL IMAGE: {:?}ms",
            Local::now().timestamp_millis() - &start
        );

        let pos_width = user_position_font.measure_text(&format!("#{}", index + 1));
        canvas.draw_string(
            &format!("#{}", index + 1),
            x + width - (padding / 1.5) - pos_width,
            vertically_centered_baseline(y, height, &user_position_font),
            &user_position_font,
            &secondary_text_trans_paint,
        );
        canvas.draw_string(
            &display_name,
            x + padding + height - (padding / 1.5),
            y + sub_section_font_size,
            &sub_section_font,
            &primary_text_paint,
        );
        canvas.draw_string(
            &num_format.fmt2(user.message_count).replace(".0", ""),
            x + padding + height - (padding / 1.5),
            y + (sub_section_font_size * 2.0),
            &sub_section_small_font,
            &primary_text_paint,
        );
    }

    // voice ranks
    canvas.draw_string(
        "Voice Ranks",
        bottom_section_b_x + padding,
        bottom_area_y + (padding / 1.5) + section_header_font_size,
        &section_header_font,
        &primary_text_paint,
    );

    for (index, user_id) in positions.voice.iter().enumerate() {
        if index >= 10 {
            break;
        }
        let user = users
            .iter()
            .find(|u| &u.user_id == user_id)
            .unwrap()
            .to_owned();

        let nickname = user.nickname;
        let display_name = user.display_name;
        let global_name = user.global_name;
        let username = user.username;

        let mut display_name =
            nickname.unwrap_or(display_name.unwrap_or(global_name.unwrap_or(username.clone())));
        let display_name_max_length = 32;

        if display_name.chars().count() > display_name_max_length {
            let mut string = display_name.clone();
            string.truncate(display_name_max_length - 3);
            display_name = string.clone() + &"...";
        }

        let width = bottom_sub_section_width - (padding / 1.5);
        let height = (sub_section_font_size * 2.0) + (padding / 2.0);
        let x = bottom_sub_section_b_x + (padding / 3.0);
        let y =
            bottom_sub_section_y + (padding / 3.0) + ((height + (padding / 3.0)) * (index as f32));

        let rect = Rect::from_xywh(x, y, width, height);
        canvas.draw_round_rect(
            &rect,
            full_inner_border_radius,
            full_inner_border_radius,
            &secondary_background_paint,
        );

        let rect = Rect::from_xywh(
            x + (padding / 3.0),
            y + (padding / 3.0),
            height - (padding / 1.5),
            height - (padding / 1.5),
        );
        canvas.draw_round_rect(
            &rect,
            full_inner_border_radius / 2.0,
            full_inner_border_radius / 2.0,
            &quaternary_background_paint,
        );

        if let Some(avatar_url) = user.avatar {
            if let Some(avatar) = avatars.iter().find(|u| &u.0 == user_id) {
                canvas.save();
                let mut builder = PathBuilder::new();
                builder.add_round_rect(
                    &rect,
                    full_inner_border_radius / 2.0,
                    full_inner_border_radius / 2.0,
                );
                canvas.clip_path(&builder.build(), ClipOp::Intersect, true);
                canvas.draw_image_rect(&avatar.1, None, &rect, Some(&secondary_background_paint));
                canvas.restore();
            } else {
                if let Some(avatar) = load_image_from_url(&avatar_url).await {
                    avatars.push((user_id.clone(), avatar.clone()));
                    canvas.save();
                    let mut builder = PathBuilder::new();
                    builder.add_round_rect(
                        &rect,
                        full_inner_border_radius / 2.0,
                        full_inner_border_radius / 2.0,
                    );
                    canvas.clip_path(&builder.build(), ClipOp::Intersect, true);
                    canvas.draw_image_rect(&avatar, None, &rect, Some(&secondary_background_paint));
                    canvas.restore();
                } else {
                    draw_username_letter(
                        &mut canvas,
                        &username,
                        padding,
                        padding,
                        header_height,
                        header_height,
                        &section_header_font,
                        &primary_text_paint,
                    );
                }
            }
        } else {
            draw_username_letter(
                &mut canvas,
                &username,
                padding,
                padding,
                header_height,
                header_height,
                &section_header_font,
                &primary_text_paint,
            );
        }

        let pos_width = user_position_font.measure_text(&format!("#{}", index + 1));
        canvas.draw_string(
            &format!("#{}", index + 1),
            x + width - (padding / 1.5) - pos_width,
            vertically_centered_baseline(y, height, &user_position_font),
            &user_position_font,
            &secondary_text_trans_paint,
        );
        canvas.draw_string(
            &display_name,
            x + padding + height - (padding / 1.5),
            y + sub_section_font_size,
            &sub_section_font,
            &primary_text_paint,
        );
        canvas.draw_string(
            &num_format.fmt2(user.voice_time),
            x + padding + height - (padding / 1.5),
            y + (sub_section_font_size * 2.0),
            &sub_section_small_font,
            &primary_text_paint,
        );
    }

    // total ranks
    canvas.draw_string(
        "Total Ranks",
        bottom_section_c_x + padding,
        bottom_area_y + (padding / 1.5) + section_header_font_size,
        &section_header_font,
        &primary_text_paint,
    );

    for (index, user_id) in positions.total.iter().enumerate() {
        if index >= 10 {
            break;
        }
        let user = users
            .iter()
            .find(|u| &u.user_id == user_id)
            .unwrap()
            .to_owned();

        let nickname = user.nickname;
        let display_name = user.display_name;
        let global_name = user.global_name;
        let username = user.username;

        let mut display_name =
            nickname.unwrap_or(display_name.unwrap_or(global_name.unwrap_or(username.clone())));
        let display_name_max_length = 32;

        if display_name.chars().count() > display_name_max_length {
            let mut string = display_name.clone();
            string.truncate(display_name_max_length - 3);
            display_name = string.clone() + &"...";
        }

        let width = bottom_sub_section_width - (padding / 1.5);
        let height = (sub_section_font_size * 2.0) + (padding / 2.0);
        let x = bottom_sub_section_c_x + (padding / 3.0);
        let y =
            bottom_sub_section_y + (padding / 3.0) + ((height + (padding / 3.0)) * (index as f32));

        let rect = Rect::from_xywh(x, y, width, height);
        canvas.draw_round_rect(
            &rect,
            full_inner_border_radius,
            full_inner_border_radius,
            &secondary_background_paint,
        );

        let rect = Rect::from_xywh(
            x + (padding / 3.0),
            y + (padding / 3.0),
            height - (padding / 1.5),
            height - (padding / 1.5),
        );
        canvas.draw_round_rect(
            &rect,
            full_inner_border_radius / 2.0,
            full_inner_border_radius / 2.0,
            &quaternary_background_paint,
        );

        if let Some(avatar_url) = user.avatar {
            if let Some(avatar) = avatars.iter().find(|u| &u.0 == user_id) {
                canvas.save();
                let mut builder = PathBuilder::new();
                builder.add_round_rect(
                    &rect,
                    full_inner_border_radius / 2.0,
                    full_inner_border_radius / 2.0,
                );
                canvas.clip_path(&builder.build(), ClipOp::Intersect, true);
                canvas.draw_image_rect(&avatar.1, None, &rect, Some(&secondary_background_paint));
                canvas.restore();
            } else {
                if let Some(avatar) = load_image_from_url(&avatar_url).await {
                    avatars.push((user_id.clone(), avatar.clone()));
                    canvas.save();
                    let mut builder = PathBuilder::new();
                    builder.add_round_rect(
                        &rect,
                        full_inner_border_radius / 2.0,
                        full_inner_border_radius / 2.0,
                    );
                    canvas.clip_path(&builder.build(), ClipOp::Intersect, true);
                    canvas.draw_image_rect(&avatar, None, &rect, Some(&secondary_background_paint));
                    canvas.restore();
                } else {
                    draw_username_letter(
                        &mut canvas,
                        &username,
                        padding,
                        padding,
                        header_height,
                        header_height,
                        &section_header_font,
                        &primary_text_paint,
                    );
                }
            }
        } else {
            draw_username_letter(
                &mut canvas,
                &username,
                padding,
                padding,
                header_height,
                header_height,
                &section_header_font,
                &primary_text_paint,
            );
        }

        let pos_width = user_position_font.measure_text(&format!("#{}", index + 1));
        canvas.draw_string(
            &format!("#{}", index + 1),
            x + width - (padding / 1.5) - pos_width,
            vertically_centered_baseline(y, height, &user_position_font),
            &user_position_font,
            &secondary_text_trans_paint,
        );
        canvas.draw_string(
            &display_name,
            x + padding + height - (padding / 1.5),
            y + sub_section_font_size,
            &sub_section_font,
            &primary_text_paint,
        );
        canvas.draw_string(
            &num_format.fmt2(user.total),
            x + padding + height - (padding / 1.5),
            y + (sub_section_font_size * 2.0),
            &sub_section_small_font,
            &primary_text_paint,
        );
    }

    // prepare output
    let width = surface.width();
    let img_info = skia_rs::codec::ImageInfo::new(
        width,
        surface.height(),
        skia_rs::core::ColorType::Rgba8888,
        skia_rs::core::AlphaType::Premul,
    );

    let data =
        skia_rs::codec::Image::from_raster_data(&img_info, surface.pixels(), width as usize * 4)
            .unwrap();
    let encoder = PngEncoder::new();
    Some(encoder.encode_bytes(&data).unwrap())
}

fn vertically_centered_baseline(y: f32, height: f32, font: &Font) -> f32 {
    let metrics = font.metrics();

    y + (height / 2.0) - ((metrics.ascent + metrics.descent) / 2.0)
}
