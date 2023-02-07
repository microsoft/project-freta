// Copyright (C) Microsoft Corporation. All rights reserved.
use std::pin::Pin;

use futures::{Stream, StreamExt};
use serde::ser::{SerializeSeq, Serializer};

use freta::{Image, Result};

pub async fn image_list_to_json(
    stream: &mut Pin<Box<impl Stream<Item = std::result::Result<Image, crate::Error>>>>,
) -> Result<()> {
    // page through results, and build a ImagesListResponse-like output (JSON)
    // by hand.  This is a bit of a hack, but this allows us to not have
    // to coalesce all of the images_list calls in memory before
    // serializing the output.
    print!("{{\"images\":");
    let mut ser = serde_json::Serializer::new(std::io::stdout());
    let mut serializer = ser.serialize_seq(None)?;
    while let Some(image) = stream.next().await {
        let image = image?;
        serializer.serialize_element(&image)?;
    }
    serializer.end()?;
    println!("}}");
    Ok(())
}

pub async fn artifact_list_to_json(
    stream: &mut Pin<Box<impl Stream<Item = std::result::Result<String, crate::Error>>>>,
) -> Result<()> {
    // page through results, and build a JSON output by hand.  This is a
    // bit of a hack, but this allows us to not have to coalesce all of
    // the images_list calls in memory before serializing the output.
    let mut ser = serde_json::Serializer::new(std::io::stdout());
    let mut serializer = ser.serialize_seq(None)?;
    while let Some(name) = stream.next().await {
        let name = name?;
        serializer.serialize_element(&name)?;
    }
    serializer.end()?;
    Ok(())
}
