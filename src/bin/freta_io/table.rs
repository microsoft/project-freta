// Copyright (C) Microsoft Corporation. All rights reserved.
use std::pin::Pin;

use cli_table::{print_stdout, Cell, CellStruct, Style, Table};
use futures::{Stream, StreamExt};

use freta::{Image, Result};

pub async fn from_images(
    stream: &mut Pin<Box<impl Stream<Item = std::result::Result<Image, crate::Error>>>>,
) -> Result<()> {
    let mut table: Vec<Vec<CellStruct>> = Vec::new();
    while let Some(image) = stream.next().await {
        let image: Image = image?;
        table.push(vec![
            image.image_id.cell(),
            image.owner_id.cell(),
            image.state.cell(),
            image.format.cell(),
        ]);
    }
    let table = table
        .table()
        .title(vec![
            "Image ID".cell().bold(true),
            "Owner ID".cell().bold(true),
            "State".cell().bold(true),
            "Format".cell().bold(true),
        ])
        .bold(true);

    assert!(print_stdout(table).is_ok());
    Ok(())
}

pub async fn from_artifacts(
    stream: &mut Pin<Box<impl Stream<Item = std::result::Result<String, crate::Error>>>>,
) -> Result<()> {
    let mut table: Vec<Vec<CellStruct>> = Vec::new();
    while let Some(artifact) = stream.next().await {
        let artifact: String = artifact?;
        table.push(vec![artifact.cell()]);
    }
    let table = table
        .table()
        .title(vec!["Artifact".cell().bold(true)])
        .bold(true);

    assert!(print_stdout(table).is_ok());
    Ok(())
}
