use super::TerminalFonts;
use bevy::prelude::*;
use fontdb::{Database, Family, Query, Source, Style, Weight};

fn try_default(asset_server: Res<AssetServer>) -> Option<TerminalFonts> {
    let mut database = Database::new();
    let families = &[Family::Name("RobotoMono"), Family::Monospace];
    let queries = [
        (Weight::NORMAL, Style::Normal),
        (Weight::NORMAL, Style::Italic),
        (Weight::BOLD, Style::Normal),
        (Weight::BOLD, Style::Italic),
    ];

    database.load_system_fonts();

    let fonts = queries
        .map(|(weight, style)| {
            let id = database.query(&Query {
                families,
                weight,
                style,
                ..Query::default()
            })?;

            let (source, _index) = database.face_source(id)?;

            if let Source::File(path) = source {
                Some(path)
            } else {
                None
            }
        })
        .map(|path| path.map(|path| asset_server.load(path)));

    let [regular, regular_italic, bold, bold_italic] = fonts;

    Some(TerminalFonts {
        regular: regular?,
        regular_italic: regular_italic?,
        bold: bold?,
        bold_italic: bold_italic?,
    })
}

pub fn default(asset_server: Res<AssetServer>) -> TerminalFonts {
    try_default(asset_server).unwrap_or_else(|| panic!("unable to find any suitable fonts to use"))
}
