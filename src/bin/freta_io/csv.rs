// Copyright (C) Microsoft Corporation. All rights reserved.
use freta::{Image, Result};
use futures::{Stream, StreamExt};
use std::pin::Pin;

pub async fn image_list_to_csv(
    stream: &mut Pin<Box<impl Stream<Item = std::result::Result<Image, crate::Error>>>>,
) -> Result<()> {
    print!("image_id,owner_id,state,format\n");
    while let Some(image) = stream.next().await {
        let image: Image = image?;
        print!(
            "{},{},{},{}\n",
            image.image_id, image.owner_id, image.state, image.format
        );
    }
    Ok(())
}

pub async fn artifact_list_to_csv(
    stream: &mut Pin<Box<impl Stream<Item = std::result::Result<String, crate::Error>>>>,
) -> Result<()> {
    print!("artifact\n");
    while let Some(artifact) = stream.next().await {
        let artifact: String = artifact?;
        print!("{}\n", artifact)
    }
    Ok(())
}
