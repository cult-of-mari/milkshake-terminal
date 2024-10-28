use super::TerminalFonts;
use bevy::prelude::*;
use fontdb::{Database, Family, Query, Source, Style, Weight};

fn try_load(asset_server: &AssetServer, database: &Database, query: Query) -> Option<Handle<Font>> {
    let id = database.query(&query)?;
    let (source, _index) = database.face_source(id)?;

    let Source::File(path) = source else {
        return None;
    };

    Some(asset_server.load(path))
}

fn try_default(asset_server: &AssetServer) -> Option<TerminalFonts> {
    let mut database = Database::new();
    let families = &[Family::Name("Roboto Mono"), Family::Monospace];
    let queries = [
        (Weight::NORMAL, Style::Normal),
        (Weight::NORMAL, Style::Italic),
        (Weight::BOLD, Style::Normal),
        (Weight::BOLD, Style::Italic),
    ];

    database.load_system_fonts();

    let fonts = queries.map(|(weight, style)| {
        let query = Query {
            families,
            weight,
            style,
            ..Query::default()
        };

        try_load(asset_server, &database, query)
    });

    let [regular, regular_italic, bold, bold_italic] = fonts;
    let regular = regular?;

    Some(TerminalFonts {
        regular: regular.clone(),
        regular_italic: regular_italic.unwrap_or_else(|| regular.clone()),
        bold: bold.unwrap_or_else(|| regular.clone()),
        bold_italic: bold_italic.unwrap_or_else(|| regular.clone()),
    })
}

pub fn default(asset_server: &AssetServer) -> TerminalFonts {
    try_default(asset_server).unwrap_or_else(|| panic!("unable to find any suitable fonts to use"))
}
