use poise::CreateReply;
use serenity::all::{CacheHttp, CreateMessage};

pub async fn broadcast_message(
    ctx: impl CacheHttp,
    channels: impl IntoIterator<Item = serenity::model::id::ChannelId>,
    msg: serenity::all::CreateEmbed,
) -> Result<(), crate::Error> {
    for c in channels {
        c.send_message(&ctx, CreateMessage::new().embed(msg.clone()))
            .await?;
    }
    Ok(())
}

pub async fn send_admin_message(
    ctx: impl CacheHttp,
    admin: serenity::model::id::ChannelId,
    msg: serenity::all::CreateEmbed,
) -> Result<(), crate::Error> {
    admin
        .send_message(&ctx, CreateMessage::new().embed(msg.clone()))
        .await?;
    Ok(())
}

pub fn create_embed(
    color: impl Into<serenity::all::Color>,
    title: impl AsRef<str>,
    message: impl AsRef<str>,
) -> serenity::all::CreateEmbed {
    serenity::all::CreateEmbed::new()
        .footer(serenity::all::CreateEmbedFooter::new(
            "made by maiboyer with ❤️",
        ))
        .color(color)
        .description(message.as_ref())
        .title(title.as_ref())
}

pub async fn send_error(
    ctx: poise::Context<'_, crate::Data, crate::Error>,
    msg: impl AsRef<str>,
) -> Result<(), crate::Error> {
    ctx.send(
        CreateReply::default()
            .reply(true)
            .ephemeral(false)
            .embed(create_embed((255, 0, 0), "Error!", msg)),
    )
    .await?;
    Ok(())
}

pub async fn send_reply(
    ctx: poise::Context<'_, crate::Data, crate::Error>,
    msg: impl AsRef<str>,
) -> Result<(), crate::Error> {
    ctx.send(
        CreateReply::default()
            .reply(true)
            .ephemeral(false)
            .embed(create_embed((0, 255, 0), "Success!", msg)),
    )
    .await?;
    Ok(())
}
