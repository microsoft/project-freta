// Copyright (C) Microsoft Corporation. All rights reserved.
use freta::{Image, Result};
use futures::{Stream, StreamExt};
use std::pin::Pin;

pub async fn from_images(
    stream: &mut Pin<Box<impl Stream<Item = std::result::Result<Image, crate::Error>>>>,
) -> Result<()> {
    println!("image_id,owner_id,state,format");
    while let Some(image) = stream.next().await {
        let image: Image = image?;
        println!(
            "{},{},{},{}",
            image.image_id, image.owner_id, image.state, image.format
        );
    }
    Ok(())
}

pub async fn from_artifacts(
    stream: &mut Pin<Box<impl Stream<Item = std::result::Result<String, crate::Error>>>>,
) -> Result<()> {
    println!("artifact");
    while let Some(artifact) = stream.next().await {
        let artifact: String = artifact?;
        println!("{artifact}");
    }
    Ok(())
}
